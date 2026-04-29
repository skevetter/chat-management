mod db;
mod models;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use db::Database;

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

    #[arg(long, global = true)]
    json: bool,

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
    },
    Show {
        name_or_id: String,
    },
    Delete {
        name_or_id: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let db_path = match cli.db {
        Some(p) => PathBuf::from(p),
        None => default_db_path(),
    };
    if let Some(parent) = db_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("Failed to create database directory: {e}");
            std::process::exit(1);
        });
    }
    let db_str = db_path.to_string_lossy();
    let db = Database::open(&db_str).unwrap_or_else(|e| {
        eprintln!("Failed to open database: {e}");
        std::process::exit(1);
    });

    let json = cli.json;
    let namespace = cli.namespace.as_deref();

    match cli.command {
        Commands::Channel { command } => match command {
            ChannelCommands::Create { name, purpose } => {
                let ns = namespace.unwrap_or("default");
                let channel = db
                    .create_channel(&name, ns, purpose.as_deref())
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to create channel: {e}");
                        std::process::exit(1);
                    });
                if json {
                    println!("{}", serde_json::to_string(&channel).unwrap());
                } else {
                    println!("{channel}");
                }
            }
            ChannelCommands::List { limit, offset } => {
                let result = db
                    .list_channels(namespace, limit, offset)
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to list channels: {e}");
                        std::process::exit(1);
                    });
                if json {
                    println!("{}", serde_json::to_string(&result).unwrap());
                } else if result.channels.is_empty() {
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
            ChannelCommands::Show { name_or_id } => {
                let channel = db.get_channel(&name_or_id, namespace).unwrap_or_else(|e| {
                    eprintln!("Failed to get channel: {e}");
                    std::process::exit(1);
                });
                match channel {
                    Some(ch) => {
                        if json {
                            println!("{}", serde_json::to_string(&ch).unwrap());
                        } else {
                            println!("{ch}");
                        }
                    }
                    None => {
                        eprintln!("Channel not found: {name_or_id}");
                        std::process::exit(1);
                    }
                }
            }
            ChannelCommands::Delete { name_or_id } => {
                let deleted_id = db
                    .delete_channel(&name_or_id, namespace)
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to delete channel: {e}");
                        std::process::exit(1);
                    });
                if let Some(id) = deleted_id {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string(&serde_json::json!({"deleted": true, "channel_id": id})).unwrap()
                        );
                    } else {
                        println!("Channel deleted: {name_or_id}");
                    }
                } else {
                    eprintln!("Channel not found: {name_or_id}");
                    std::process::exit(1);
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
            let ch = db
                .get_channel(&channel, namespace)
                .unwrap_or_else(|e| {
                    eprintln!("Failed to resolve channel: {e}");
                    std::process::exit(1);
                })
                .unwrap_or_else(|| {
                    eprintln!("Channel not found: {channel}");
                    std::process::exit(1);
                });
            let message = db
                .post_message(
                    ch.id,
                    &sender,
                    &content,
                    reply_to.as_deref(),
                    idempotency_key.as_deref(),
                )
                .unwrap_or_else(|e| {
                    eprintln!("Failed to post message: {e}");
                    std::process::exit(1);
                });
            if json {
                println!("{}", serde_json::to_string(&message).unwrap());
            } else {
                println!("{message}");
            }
        }
        Commands::Read {
            channel,
            limit,
            offset,
            since,
            sender,
        } => {
            let ch = db
                .get_channel(&channel, namespace)
                .unwrap_or_else(|e| {
                    eprintln!("Failed to resolve channel: {e}");
                    std::process::exit(1);
                })
                .unwrap_or_else(|| {
                    eprintln!("Channel not found: {channel}");
                    std::process::exit(1);
                });
            let result = db
                .read_messages(ch.id, limit, offset, since.as_deref(), sender.as_deref())
                .unwrap_or_else(|e| {
                    eprintln!("Failed to read messages: {e}");
                    std::process::exit(1);
                });
            if json {
                println!("{}", serde_json::to_string(&result).unwrap());
            } else if result.messages.is_empty() {
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
        Commands::Inspect { channel } => {
            let ch = db.inspect_channel(&channel, namespace).unwrap_or_else(|e| {
                eprintln!("Failed to inspect channel: {e}");
                std::process::exit(1);
            });
            match ch {
                Some(c) => {
                    if json {
                        println!("{}", serde_json::to_string(&c).unwrap());
                    } else {
                        println!("{c}");
                    }
                }
                None => {
                    eprintln!("Channel not found: {channel}");
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
                        eprintln!("Failed to resolve channel: {e}");
                        std::process::exit(1);
                    })
                    .unwrap_or_else(|| {
                        eprintln!("Channel not found: {ch}");
                        std::process::exit(1);
                    })
                    .id
            });
            let result = db
                .list_mentions(agent.as_deref(), channel_id, limit, offset)
                .unwrap_or_else(|e| {
                    eprintln!("Failed to list mentions: {e}");
                    std::process::exit(1);
                });
            if json {
                println!("{}", serde_json::to_string(&result).unwrap());
            } else if result.mentions.is_empty() {
                println!("No mentions found.");
            } else {
                println!(
                    "{:<6} {:<38} {:<6} AGENT",
                    "ID", "MESSAGE_ID", "CH_ID"
                );
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
        Commands::Serve { transport, .. } => {
            if transport != "stdio" {
                eprintln!("Only stdio transport is supported");
                std::process::exit(1);
            }
            println!("MCP server not yet implemented");
        }
    }
}
