use serde::{Deserialize, Serialize};

// ===== Config =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifyConfig {
    pub api_base: String,
    pub api_key: String,
}

// ===== App =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifyApp {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mode: String,
    pub icon: String,
    pub icon_background: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: String,
    pub app_id: String,
    pub conversation_id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: i64,
    pub app_name: String,
}

// ===== Message =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub app_id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub query: String,
    pub answer: String,
    pub feedback: Option<String>,
    pub retriever_resources: String, // JSON string
    pub message_metadata: String,    // JSON string
    pub agent_thoughts: String,      // JSON string
    pub answer_tokens: i64,
    pub prompt_tokens: i64,
    pub elapsed_time: f64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDetail {
    pub id: String,
    pub message_id: String,
    pub conversation_id: String,
    pub query: String,
    pub answer: String,
    pub feedback: Option<String>,
    pub retriever_resources: serde_json::Value,
    pub message_metadata: serde_json::Value,
    pub agent_thoughts: serde_json::Value,
    pub answer_tokens: i64,
    pub prompt_tokens: i64,
    pub elapsed_time: f64,
    pub created_at: i64,
}

impl From<Message> for MessageDetail {
    fn from(m: Message) -> Self {
        MessageDetail {
            id: m.id,
            message_id: m.message_id,
            conversation_id: m.conversation_id,
            query: m.query,
            answer: m.answer,
            feedback: m.feedback,
            retriever_resources: serde_json::from_str(&m.retriever_resources).unwrap_or(serde_json::Value::Null),
            message_metadata: serde_json::from_str(&m.message_metadata).unwrap_or(serde_json::Value::Null),
            agent_thoughts: serde_json::from_str(&m.agent_thoughts).unwrap_or(serde_json::Value::Null),
            answer_tokens: m.answer_tokens,
            prompt_tokens: m.prompt_tokens,
            elapsed_time: m.elapsed_time,
            created_at: m.created_at,
        }
    }
}

// ===== Dify API Response Types =====
#[derive(Debug, Deserialize)]
pub struct DifyAppsResponse {
    pub data: Vec<DifyAppItem>,
}

#[derive(Debug, Deserialize)]
pub struct DifyAppItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub mode: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default, rename = "icon_background")]
    pub icon_background: String,
    #[serde(default)]
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct DifyConversationsResponse {
    pub data: Vec<DifyConversationItem>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize)]
pub struct DifyConversationItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub inputs: serde_json::Value,
    pub status: String,
    #[serde(default)]
    pub introduction: String,
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct DifyMessagesResponse {
    pub data: Vec<DifyMessageItem>,
    #[allow(dead_code)]
    pub has_more: bool,
}

#[derive(Debug, Deserialize)]
pub struct DifyMessageItem {
    pub id: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub conversation_id: String,
    #[serde(default)]
    pub query: String,
    pub answer: String,
    #[serde(default)]
    pub feedback: Option<DifyFeedback>,
    #[serde(default)]
    pub retriever_resources: Vec<serde_json::Value>,
    #[serde(default, rename = "metadata")]
    pub message_metadata: serde_json::Value,
    #[serde(default)]
    pub agent_thoughts: Vec<serde_json::Value>,
    #[serde(default)]
    pub answer_tokens: i64,
    #[serde(default)]
    pub prompt_tokens: i64,
    #[serde(default)]
    pub elapsed_time: f64,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct DifyFeedback {
    pub rating: String,
}

// ===== Sync Result =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub total_conversations: i64,
    pub synced_conversations: i64,
    pub total_messages: i64,
    pub synced_messages: i64,
}

// ===== Dashboard Stats =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_apps: i64,
    pub total_conversations: i64,
    pub total_messages: i64,
    pub total_answer_tokens: i64,
    pub total_prompt_tokens: i64,
    pub feedback_like: i64,
    pub feedback_dislike: i64,
    pub feedback_none: i64,
    pub top_apps: Vec<AppRanking>,
    pub recent_daily: Vec<DailyStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRanking {
    pub app_id: String,
    pub app_name: String,
    pub conversation_count: i64,
    pub message_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub conversations: i64,
    pub messages: i64,
}

// ===== Conversations Query Result =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationsResult {
    pub data: Vec<ConversationSummary>,
    pub total: i64,
}