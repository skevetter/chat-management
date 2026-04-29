use std::sync::Mutex;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ErrorData, tool, tool_router};

use crate::db::Database;

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

        let db = self.db.lock().unwrap();
        let result = db
            .list_channels(ns, limit, offset)
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

    #[tool(description = "Post a message to a channel")]
    fn post_message(
        &self,
        Parameters(params): Parameters<PostMessageParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);

        let db = self.db.lock().unwrap();
        let channel = db
            .get_channel(&params.channel, ns)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("Channel not found: {}", params.channel), None)
            })?;

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
                params.since.as_deref(),
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
}
