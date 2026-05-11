use std::sync::Mutex;
use std::time::{Duration, Instant};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ErrorData, tool, tool_router};

use crate::db::Database;
use crate::util::resolve_since;

use super::tools::*;

pub struct ChatMcpServer {
    db: Mutex<Database>,
    default_namespace: Option<String>,
}

impl ChatMcpServer {
    pub fn new(db: Database, default_namespace: Option<String>) -> Self {
        Self {
            db: Mutex::new(db),
            default_namespace,
        }
    }

    fn resolve_namespace<'a>(&'a self, params_ns: &'a Option<String>) -> Option<&'a str> {
        params_ns.as_deref().or(self.default_namespace.as_deref())
    }
}

#[tool_router(server_handler)]
impl ChatMcpServer {
    #[allow(dead_code)]
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder().enable_tools().build();
        ServerInfo::new(capabilities).with_server_info(Implementation::new(
            "chat-management",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    #[tool(description = "Create a new channel")]
    fn channel_create(
        &self,
        Parameters(params): Parameters<ChannelCreateParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let namespace = ns.unwrap_or("default");

        let db = self.db.lock().unwrap();
        let channel = db
            .create_channel(&params.name, namespace, params.purpose.as_deref())
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&channel).unwrap(),
        )]))
    }

    #[tool(description = "List channels with optional namespace filter")]
    fn channel_list(
        &self,
        Parameters(params): Parameters<ChannelListParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let limit = params.limit.unwrap_or(50);
        let offset = params.offset.unwrap_or(0);
        let include_archived = params.include_archived.unwrap_or(false);

        let db = self.db.lock().unwrap();
        let result = db
            .list_channels(ns, limit, offset, include_archived)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result).unwrap(),
        )]))
    }

    #[tool(description = "Show details of a specific channel")]
    fn channel_show(
        &self,
        Parameters(params): Parameters<ChannelShowParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);

        let db = self.db.lock().unwrap();
        let channel = db
            .get_channel(&params.name_or_id, ns)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("Channel not found: {}", params.name_or_id), None)
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&channel).unwrap(),
        )]))
    }

    #[tool(description = "Delete a channel and all its messages")]
    fn channel_delete(
        &self,
        Parameters(params): Parameters<ChannelDeleteParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);

        let db = self.db.lock().unwrap();
        let deleted_id = db
            .delete_channel(&params.name_or_id, ns)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("Channel not found: {}", params.name_or_id), None)
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&serde_json::json!({"deleted": true, "channel_id": deleted_id}))
                .unwrap(),
        )]))
    }

    #[tool(description = "Archive a channel (prevents new posts)")]
    fn archive_channel(
        &self,
        Parameters(params): Parameters<ArchiveChannelParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);

        let db = self.db.lock().unwrap();
        let channel = db
            .archive_channel(&params.channel, ns)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("Channel not found: {}", params.channel), None)
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&channel).unwrap(),
        )]))
    }

    #[tool(description = "Unarchive a channel (allows new posts again)")]
    fn unarchive_channel(
        &self,
        Parameters(params): Parameters<UnarchiveChannelParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);

        let db = self.db.lock().unwrap();
        let channel = db
            .unarchive_channel(&params.channel, ns)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("Channel not found: {}", params.channel), None)
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&channel).unwrap(),
        )]))
    }

    #[tool(description = "Post a message to a channel")]
    fn post_message(
        &self,
        Parameters(params): Parameters<PostMessageParams>,
    ) -> Result<CallToolResult, ErrorData> {
        if params.content.trim().is_empty() {
            return Err(ErrorData::invalid_params(
                "Message content cannot be empty".to_string(),
                None,
            ));
        }

        let ns = self.resolve_namespace(&params.namespace);

        let db = self.db.lock().unwrap();
        let channel = db
            .get_channel(&params.channel, ns)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("Channel not found: {}", params.channel), None)
            })?;

        if channel.archived {
            return Err(ErrorData::invalid_params(
                format!("Cannot post to archived channel '{}'", channel.name),
                None,
            ));
        }

        let message = db
            .post_message(
                channel.id,
                &params.sender,
                &params.content,
                params.reply_to.as_deref(),
                params.idempotency_key.as_deref(),
            )
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&message).unwrap(),
        )]))
    }

    #[tool(description = "Read messages from a channel")]
    fn read_messages(
        &self,
        Parameters(params): Parameters<ReadMessagesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let limit = params.limit.unwrap_or(50);
        let offset = params.offset.unwrap_or(0);

        let resolved_since = params
            .since
            .as_deref()
            .map(|s| resolve_since(s).map_err(|e| ErrorData::invalid_params(e, None)))
            .transpose()?;

        let db = self.db.lock().unwrap();
        let channel = db
            .get_channel(&params.channel, ns)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("Channel not found: {}", params.channel), None)
            })?;

        let result = db
            .read_messages(
                channel.id,
                limit,
                offset,
                resolved_since.as_deref(),
                params.sender.as_deref(),
            )
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result).unwrap(),
        )]))
    }

    #[tool(description = "Inspect a channel's details and metadata")]
    fn inspect_channel(
        &self,
        Parameters(params): Parameters<InspectChannelParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);

        let db = self.db.lock().unwrap();
        let channel = db
            .inspect_channel(&params.channel, ns)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("Channel not found: {}", params.channel), None)
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&channel).unwrap(),
        )]))
    }

    #[tool(description = "List mentions, optionally filtered by agent or channel")]
    fn list_mentions(
        &self,
        Parameters(params): Parameters<ListMentionsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let limit = params.limit.unwrap_or(50);
        let offset = params.offset.unwrap_or(0);

        let db = self.db.lock().unwrap();
        let channel_id = match &params.channel {
            Some(ch) => {
                let channel = db
                    .get_channel(ch, ns)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
                    .ok_or_else(|| {
                        ErrorData::invalid_params(format!("Channel not found: {ch}"), None)
                    })?;
                Some(channel.id)
            }
            None => None,
        };

        let result = db
            .list_mentions(params.agent.as_deref(), channel_id, limit, offset)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result).unwrap(),
        )]))
    }

    #[tool(
        description = "Search messages using full-text search across all channels or filtered by channel"
    )]
    fn search_messages(
        &self,
        Parameters(params): Parameters<SearchMessagesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = params.limit.unwrap_or(20);
        let ns = Some(params.namespace.as_str());

        let db = self.db.lock().unwrap();
        let channel_id = match &params.channel {
            Some(ch) => {
                let channel = db
                    .get_channel(ch, ns)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
                    .ok_or_else(|| {
                        ErrorData::invalid_params(format!("Channel not found: {ch}"), None)
                    })?;
                Some(channel.id)
            }
            None => None,
        };

        let result = db
            .search_messages(&params.query, channel_id, ns, limit)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result).unwrap(),
        )]))
    }

    #[tool(
        description = "Wait for a new message in a channel (blocks until message arrives or timeout)"
    )]
    async fn wait_for_message(
        &self,
        Parameters(params): Parameters<WaitForMessageParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = Some(params.namespace.as_str());
        let timeout = params.timeout.unwrap_or(300);

        let (channel_id, baseline) = {
            let db = self.db.lock().unwrap();
            let channel = db
                .get_channel(&params.channel, ns)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
                .ok_or_else(|| {
                    ErrorData::invalid_params(
                        format!("Channel not found: {}", params.channel),
                        None,
                    )
                })?;

            if channel.archived {
                return Err(ErrorData::invalid_params(
                    format!("Cannot wait on archived channel '{}'", channel.name),
                    None,
                ));
            }

            let baseline = db
                .get_max_message_rowid(channel.id)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            (channel.id, baseline)
        };

        let deadline = Duration::from_secs(timeout);
        let start = Instant::now();
        loop {
            {
                let db = self.db.lock().unwrap();
                let messages = db
                    .get_messages_after_rowid(channel_id, baseline)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                if let Some(msg) = messages.first() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string(msg).unwrap(),
                    )]));
                }
            }
            if start.elapsed() >= deadline {
                return Err(ErrorData::internal_error(
                    format!(
                        "Timeout: no new messages in {} after {} seconds",
                        params.channel, timeout
                    ),
                    None,
                ));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}
