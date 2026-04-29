use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: i64,
    pub name: String,
    pub namespace: String,
    pub purpose: Option<String>,
    pub created_at: String,
    pub message_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: i64,
    pub sender: String,
    pub content: String,
    pub timestamp: String,
    pub reply_to: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mention {
    pub id: i64,
    pub message_id: String,
    pub channel_id: i64,
    pub mentioned_agent: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelListResult {
    pub channels: Vec<Channel>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageListResult {
    pub messages: Vec<Message>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionListResult {
    pub mentions: Vec<Mention>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub id: String,
    pub channel: String,
    pub sender: String,
    pub timestamp: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub results: Vec<SearchResultItem>,
    pub total: i64,
}

impl fmt::Display for SearchResultItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "[{}] #{} @{}: {}",
            self.timestamp, self.channel, self.sender, self.content
        )?;
        write!(f, "  id: {}", self.id)
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ID:            {}", self.id)?;
        writeln!(f, "Name:          {}", self.name)?;
        writeln!(f, "Namespace:     {}", self.namespace)?;
        if let Some(purpose) = &self.purpose {
            writeln!(f, "Purpose:       {purpose}")?;
        }
        writeln!(f, "Messages:      {}", self.message_count)?;
        write!(f, "Created:       {}", self.created_at)
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[{}] @{}: {}", self.timestamp, self.sender, self.content)?;
        if let Some(reply) = &self.reply_to {
            writeln!(f, "  (reply to {reply})")?;
        }
        write!(f, "  id: {}", self.id)
    }
}
