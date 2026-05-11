mod db;
mod mcp;
mod models;
mod util;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand, ValueEnum};

use db::Database;
use util::resolve_since;

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum OutputFormat {
    #[default]
    Tabular,
    Json,
    Csv,
}

fn output_error(msg: &str, format: OutputFormat) {
    match format {
        OutputFormat::Json => eprintln!("{}", serde_json::json!({"error": msg})),
        _ => eprintln!("{msg}"),
    }
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn default_db_path() -> PathBuf {
    let base = match std::env::var("XDG_DATA_HOME") {
        Ok(val) if !val.is_empty() => {
            let path = PathBuf::from(&val);
            if path.is_relative() {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(path)
            } else {
                path
            }
        }
        _ => match std::env::var("HOME") {
            Ok(home) if !home.is_empty() => PathBuf::from(home).join(".local").join("share"),
            _ => PathBuf::from("."),
        },
    };
    base.join("chat-management").join("chat.db")
}

#[derive(Parser)]
#[command(
    name = "chat-management",
    about = "A communication management CLI tool",
    version
)]
struct Cli {
    #[arg(long, global = true)]
    db: Option<String>,

    #[arg(long, short = 'o', global = true, default_value_t = OutputFormat::Tabular, value_enum)]
    output: OutputFormat,

    #[arg(long, short = 'n', global = true)]
    namespace: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Channel {
        #[command(subcommand)]
        command: ChannelCommands,
    },
    Post {
        channel: String,
        #[arg(long)]
        sender: String,
        #[arg(long)]
        content: String,
        #[arg(long)]
        reply_to: Option<String>,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
    Read {
        channel: String,
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        sender: Option<String>,
    },
    Inspect {
        channel: String,
    },
    Mentions {
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },
    Search {
        #[arg(long)]
        query: String,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
    Wait {
        channel: String,
        #[arg(long, default_value_t = 300)]
        timeout: u64,
    },
    Serve {
        #[arg(long, default_value = "stdio")]
        transport: String,
        #[arg(long)]
        namespace: Option<String>,
    },
}

#[derive(Subcommand)]
enum ChannelCommands {
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        purpose: Option<String>,
    },
    List {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
        #[arg(long)]
        include_archived: bool,
    },
    Show {
        name_or_id: String,
    },
    Delete {
        name_or_id: String,
    },
    Archive {
        name_or_id: String,
    },
    Unarchive {
        name_or_id: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let fmt = cli.output;
    let db_path = match cli.db {
        Some(p) => PathBuf::from(p),
        None => default_db_path(),
    };
    if let Some(parent) = db_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            output_error(&format!("Failed to create database directory: {e}"), fmt);
            std::process::exit(1);
        });
    }
    let db_str = db_path.to_string_lossy();
    let db = Database::open(&db_str).unwrap_or_else(|e| {
        output_error(&format!("Failed to open database: {e}"), fmt);
        std::process::exit(1);
    });

    let namespace = cli.namespace.as_deref();

    match cli.command {
        Commands::Channel { command } => match command {
            ChannelCommands::Create { name, purpose } => {
                let ns = namespace.unwrap_or("default");
                let channel = db
                    .create_channel(&name, ns, purpose.as_deref())
                    .unwrap_or_else(|e| {
                        output_error(&format!("Failed to create channel: {e}"), fmt);
                        std::process::exit(1);
                    });
                match fmt {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string(&channel).unwrap());
                    }
                    OutputFormat::Csv => {
                        println!("id,name,namespace,purpose,message_count");
                        println!(
                            "{},{},{},{},{}",
                            channel.id,
                            csv_escape(&channel.name),
                            csv_escape(&channel.namespace),
                            csv_escape(channel.purpose.as_deref().unwrap_or("")),
                            channel.message_count
                        );
                    }
                    OutputFormat::Tabular => {
                        println!("{channel}");
                    }
                }
            }
            ChannelCommands::List {
                limit,
                offset,
                include_archived,
            } => {
                let result = db
                    .list_channels(namespace, limit, offset, include_archived)
                    .unwrap_or_else(|e| {
                        output_error(&format!("Failed to list channels: {e}"), fmt);
                        std::process::exit(1);
                    });
                match fmt {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string(&result).unwrap());
                    }
                    OutputFormat::Csv => {
                        println!("id,name,namespace,purpose,message_count");
                        for ch in &result.channels {
                            println!(
                                "{},{},{},{},{}",
                                ch.id,
                                csv_escape(&ch.name),
                                csv_escape(&ch.namespace),
                                csv_escape(ch.purpose.as_deref().unwrap_or("")),
                                ch.message_count
                            );
                        }
                    }
                    OutputFormat::Tabular => {
                        if result.channels.is_empty() {
                            println!("No channels found.");
                        } else {
                            println!(
                                "{:<6} {:<20} {:<12} {:<8} PURPOSE",
                                "ID", "NAME", "NAMESPACE", "MSGS"
                            );
                            println!("{}", "-".repeat(60));
                            for ch in &result.channels {
                                let purpose = ch.purpose.as_deref().unwrap_or("-");
                                println!(
                                    "{:<6} {:<20} {:<12} {:<8} {}",
                                    ch.id, ch.name, ch.namespace, ch.message_count, purpose
                                );
                            }
                            let start = offset + 1;
                            let end = offset + result.channels.len() as i64;
                            println!("\nShowing {start}-{end} of {} channel(s)", result.total);
                        }
                    }
                }
            }
            ChannelCommands::Show { name_or_id } => {
                let channel = db.get_channel(&name_or_id, namespace).unwrap_or_else(|e| {
                    output_error(&format!("Failed to get channel: {e}"), fmt);
                    std::process::exit(1);
                });
                match channel {
                    Some(ch) => match fmt {
                        OutputFormat::Json => {
                            println!("{}", serde_json::to_string(&ch).unwrap());
                        }
                        OutputFormat::Csv => {
                            println!("id,name,namespace,purpose,message_count");
                            println!(
                                "{},{},{},{},{}",
                                ch.id,
                                csv_escape(&ch.name),
                                csv_escape(&ch.namespace),
                                csv_escape(ch.purpose.as_deref().unwrap_or("")),
                                ch.message_count
                            );
                        }
                        OutputFormat::Tabular => {
                            println!("{ch}");
                        }
                    },
                    None => {
                        output_error(&format!("Channel not found: {name_or_id}"), fmt);
                        std::process::exit(1);
                    }
                }
            }
            ChannelCommands::Delete { name_or_id } => {
                let deleted_id = db
                    .delete_channel(&name_or_id, namespace)
                    .unwrap_or_else(|e| {
                        output_error(&format!("Failed to delete channel: {e}"), fmt);
                        std::process::exit(1);
                    });
                if let Some(id) = deleted_id {
                    match fmt {
                        OutputFormat::Json => {
                            println!(
                                "{}",
                                serde_json::to_string(
                                    &serde_json::json!({"deleted": true, "channel_id": id})
                                )
                                .unwrap()
                            );
                        }
                        OutputFormat::Csv => {
                            println!("deleted,channel_id");
                            println!("true,{id}");
                        }
                        OutputFormat::Tabular => {
                            println!("Channel deleted: {name_or_id}");
                        }
                    }
                } else {
                    output_error(&format!("Channel not found: {name_or_id}"), fmt);
                    std::process::exit(1);
                }
            }
            ChannelCommands::Archive { name_or_id } => {
                let channel = db
                    .archive_channel(&name_or_id, namespace)
                    .unwrap_or_else(|e| {
                        output_error(&format!("Failed to archive channel: {e}"), fmt);
                        std::process::exit(1);
                    });
                match channel {
                    Some(ch) => match fmt {
                        OutputFormat::Json => {
                            println!("{}", serde_json::to_string(&ch).unwrap());
                        }
                        OutputFormat::Csv => {
                            println!("id,name,namespace,purpose,message_count");
                            println!(
                                "{},{},{},{},{}",
                                ch.id,
                                csv_escape(&ch.name),
                                csv_escape(&ch.namespace),
                                csv_escape(ch.purpose.as_deref().unwrap_or("")),
                                ch.message_count
                            );
                        }
                        OutputFormat::Tabular => {
                            println!("Channel archived: {}", ch.name);
                        }
                    },
                    None => {
                        output_error(&format!("Channel not found: {name_or_id}"), fmt);
                        std::process::exit(1);
                    }
                }
            }
            ChannelCommands::Unarchive { name_or_id } => {
                let channel = db
                    .unarchive_channel(&name_or_id, namespace)
                    .unwrap_or_else(|e| {
                        output_error(&format!("Failed to unarchive channel: {e}"), fmt);
                        std::process::exit(1);
                    });
                match channel {
                    Some(ch) => match fmt {
                        OutputFormat::Json => {
                            println!("{}", serde_json::to_string(&ch).unwrap());
                        }
                        OutputFormat::Csv => {
                            println!("id,name,namespace,purpose,message_count");
                            println!(
                                "{},{},{},{},{}",
                                ch.id,
                                csv_escape(&ch.name),
                                csv_escape(&ch.namespace),
                                csv_escape(ch.purpose.as_deref().unwrap_or("")),
                                ch.message_count
                            );
                        }
                        OutputFormat::Tabular => {
                            println!("Channel unarchived: {}", ch.name);
                        }
                    },
                    None => {
                        output_error(&format!("Channel not found: {name_or_id}"), fmt);
                        std::process::exit(1);
                    }
                }
            }
        },
        Commands::Post {
            channel,
            sender,
            content,
            reply_to,
            idempotency_key,
        } => {
            if content.trim().is_empty() {
                output_error("Message content cannot be empty", fmt);
                std::process::exit(1);
            }
            let ch = db
                .get_channel(&channel, namespace)
                .unwrap_or_else(|e| {
                    output_error(&format!("Failed to resolve channel: {e}"), fmt);
                    std::process::exit(1);
                })
                .unwrap_or_else(|| {
                    output_error(&format!("Channel not found: {channel}"), fmt);
                    std::process::exit(1);
                });
            if ch.archived {
                output_error(
                    &format!("Cannot post to archived channel '{}'", ch.name),
                    fmt,
                );
                std::process::exit(1);
            }
            let message = db
                .post_message(
                    ch.id,
                    &sender,
                    &content,
                    reply_to.as_deref(),
                    idempotency_key.as_deref(),
                )
                .unwrap_or_else(|e| {
                    output_error(&format!("Failed to post message: {e}"), fmt);
                    std::process::exit(1);
                });
            match fmt {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string(&message).unwrap());
                }
                OutputFormat::Csv => {
                    println!("id,channel_id,sender,timestamp,content");
                    println!(
                        "{},{},{},{},{}",
                        csv_escape(&message.id),
                        message.channel_id,
                        csv_escape(&message.sender),
                        csv_escape(&message.timestamp),
                        csv_escape(&message.content)
                    );
                }
                OutputFormat::Tabular => {
                    println!("{message}");
                }
            }
        }
        Commands::Read {
            channel,
            limit,
            offset,
            since,
            sender,
        } => {
            let resolved_since = since.map(|s| {
                resolve_since(&s).unwrap_or_else(|e| {
                    output_error(&e, fmt);
                    std::process::exit(1);
                })
            });
            let ch = db
                .get_channel(&channel, namespace)
                .unwrap_or_else(|e| {
                    output_error(&format!("Failed to resolve channel: {e}"), fmt);
                    std::process::exit(1);
                })
                .unwrap_or_else(|| {
                    output_error(&format!("Channel not found: {channel}"), fmt);
                    std::process::exit(1);
                });
            let result = db
                .read_messages(
                    ch.id,
                    limit,
                    offset,
                    resolved_since.as_deref(),
                    sender.as_deref(),
                )
                .unwrap_or_else(|e| {
                    output_error(&format!("Failed to read messages: {e}"), fmt);
                    std::process::exit(1);
                });
            match fmt {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string(&result).unwrap());
                }
                OutputFormat::Csv => {
                    println!("id,channel_id,sender,timestamp,content");
                    for msg in &result.messages {
                        println!(
                            "{},{},{},{},{}",
                            csv_escape(&msg.id),
                            msg.channel_id,
                            csv_escape(&msg.sender),
                            csv_escape(&msg.timestamp),
                            csv_escape(&msg.content)
                        );
                    }
                }
                OutputFormat::Tabular => {
                    if result.messages.is_empty() {
                        println!("No messages found.");
                    } else {
                        for msg in &result.messages {
                            println!("{msg}");
                            println!();
                        }
                        let start = offset + 1;
                        let end = offset + result.messages.len() as i64;
                        println!("Showing {start}-{end} of {} message(s)", result.total);
                    }
                }
            }
        }
        Commands::Inspect { channel } => {
            let ch = db.inspect_channel(&channel, namespace).unwrap_or_else(|e| {
                output_error(&format!("Failed to inspect channel: {e}"), fmt);
                std::process::exit(1);
            });
            match ch {
                Some(c) => match fmt {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string(&c).unwrap());
                    }
                    OutputFormat::Csv => {
                        println!("id,name,namespace,purpose,message_count");
                        println!(
                            "{},{},{},{},{}",
                            c.id,
                            csv_escape(&c.name),
                            csv_escape(&c.namespace),
                            csv_escape(c.purpose.as_deref().unwrap_or("")),
                            c.message_count
                        );
                    }
                    OutputFormat::Tabular => {
                        println!("{c}");
                    }
                },
                None => {
                    output_error(&format!("Channel not found: {channel}"), fmt);
                    std::process::exit(1);
                }
            }
        }
        Commands::Mentions {
            agent,
            channel,
            limit,
            offset,
        } => {
            let channel_id = channel.map(|ch| {
                db.get_channel(&ch, namespace)
                    .unwrap_or_else(|e| {
                        output_error(&format!("Failed to resolve channel: {e}"), fmt);
                        std::process::exit(1);
                    })
                    .unwrap_or_else(|| {
                        output_error(&format!("Channel not found: {ch}"), fmt);
                        std::process::exit(1);
                    })
                    .id
            });
            let result = db
                .list_mentions(agent.as_deref(), channel_id, limit, offset)
                .unwrap_or_else(|e| {
                    output_error(&format!("Failed to list mentions: {e}"), fmt);
                    std::process::exit(1);
                });
            match fmt {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string(&result).unwrap());
                }
                OutputFormat::Csv => {
                    println!("id,message_id,channel_id,mentioned_agent");
                    for m in &result.mentions {
                        println!(
                            "{},{},{},{}",
                            m.id,
                            csv_escape(&m.message_id),
                            m.channel_id,
                            csv_escape(&m.mentioned_agent)
                        );
                    }
                }
                OutputFormat::Tabular => {
                    if result.mentions.is_empty() {
                        println!("No mentions found.");
                    } else {
                        println!("{:<6} {:<38} {:<6} AGENT", "ID", "MESSAGE_ID", "CH_ID");
                        println!("{}", "-".repeat(70));
                        for m in &result.mentions {
                            println!(
                                "{:<6} {:<38} {:<6} {}",
                                m.id, m.message_id, m.channel_id, m.mentioned_agent
                            );
                        }
                        let start = offset + 1;
                        let end = offset + result.mentions.len() as i64;
                        println!("\nShowing {start}-{end} of {} mention(s)", result.total);
                    }
                }
            }
        }
        Commands::Search {
            query,
            channel,
            limit,
        } => {
            let channel_id = channel.map(|ch| {
                db.get_channel(&ch, namespace)
                    .unwrap_or_else(|e| {
                        output_error(&format!("Failed to resolve channel: {e}"), fmt);
                        std::process::exit(1);
                    })
                    .unwrap_or_else(|| {
                        output_error(&format!("Channel not found: {ch}"), fmt);
                        std::process::exit(1);
                    })
                    .id
            });
            let result = db
                .search_messages(&query, channel_id, namespace, limit)
                .unwrap_or_else(|e| {
                    output_error(&format!("Failed to search messages: {e}"), fmt);
                    std::process::exit(1);
                });
            match fmt {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string(&result).unwrap());
                }
                OutputFormat::Csv => {
                    println!("id,channel,sender,timestamp,content");
                    for item in &result.results {
                        println!(
                            "{},{},{},{},{}",
                            csv_escape(&item.id),
                            csv_escape(&item.channel),
                            csv_escape(&item.sender),
                            csv_escape(&item.timestamp),
                            csv_escape(&item.content)
                        );
                    }
                }
                OutputFormat::Tabular => {
                    if result.results.is_empty() {
                        println!("No messages found.");
                    } else {
                        for item in &result.results {
                            println!("{item}");
                            println!();
                        }
                        println!("{} result(s)", result.total);
                    }
                }
            }
        }
        Commands::Wait { channel, timeout } => {
            let ch = db
                .get_channel(&channel, namespace)
                .unwrap_or_else(|e| {
                    output_error(&format!("Failed to resolve channel: {e}"), fmt);
                    std::process::exit(1);
                })
                .unwrap_or_else(|| {
                    output_error(&format!("Channel not found: {channel}"), fmt);
                    std::process::exit(1);
                });
            if ch.archived {
                output_error(
                    &format!("Cannot wait on archived channel '{}'", ch.name),
                    fmt,
                );
                std::process::exit(1);
            }
            let baseline = db.get_max_message_rowid(ch.id).unwrap_or_else(|e| {
                output_error(&format!("Failed to get baseline: {e}"), fmt);
                std::process::exit(1);
            });
            let deadline = Duration::from_secs(timeout);
            let start = Instant::now();
            loop {
                let messages = db
                    .get_messages_after_rowid(ch.id, baseline)
                    .unwrap_or_else(|e| {
                        output_error(&format!("Failed to poll messages: {e}"), fmt);
                        std::process::exit(1);
                    });
                if let Some(msg) = messages.first() {
                    match fmt {
                        OutputFormat::Json => {
                            println!("{}", serde_json::to_string(msg).unwrap());
                        }
                        OutputFormat::Csv => {
                            println!("id,channel_id,sender,timestamp,content");
                            println!(
                                "{},{},{},{},{}",
                                csv_escape(&msg.id),
                                msg.channel_id,
                                csv_escape(&msg.sender),
                                csv_escape(&msg.timestamp),
                                csv_escape(&msg.content)
                            );
                        }
                        OutputFormat::Tabular => {
                            println!("{msg}");
                        }
                    }
                    std::process::exit(0);
                }
                if start.elapsed() >= deadline {
                    output_error(
                        &format!("Timeout: no new messages in {channel} after {timeout} seconds"),
                        fmt,
                    );
                    std::process::exit(1);
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
        Commands::Serve {
            transport,
            namespace,
        } => {
            if transport != "stdio" {
                eprintln!("Only stdio transport is supported");
                std::process::exit(1);
            }
            let server = mcp::server::ChatMcpServer::new(db, namespace);
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                use rmcp::ServiceExt;
                let transport = rmcp::transport::io::stdio();
                let service = server.serve(transport).await.unwrap();
                service.waiting().await.unwrap();
            });
        }
    }
}
