#![allow(dead_code)]

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChannelCreateParams {
    pub name: String,
    pub purpose: Option<String>,
    pub namespace: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChannelListParams {
    pub namespace: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChannelShowParams {
    pub name_or_id: String,
    pub namespace: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChannelDeleteParams {
    pub name_or_id: String,
    pub namespace: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PostMessageParams {
    pub channel: String,
    pub sender: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub idempotency_key: Option<String>,
    pub namespace: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadMessagesParams {
    pub channel: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub since: Option<String>,
    pub sender: Option<String>,
    pub namespace: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InspectChannelParams {
    pub channel: String,
    pub namespace: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListMentionsParams {
    pub agent: Option<String>,
    pub channel: Option<String>,
    pub namespace: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
