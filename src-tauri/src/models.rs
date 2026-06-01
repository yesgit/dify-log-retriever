use serde::{Deserialize, Serialize};

pub fn json_string(value: &serde_json::Value, fallback: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| fallback.to_string())
}

// ===== Config =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifyConfig {
    pub api_base: String,
    pub api_key: String,
    #[serde(default)]
    pub proxy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifyConfigDisplay {
    pub api_base: String,
    pub api_key_masked: String,
    #[serde(default)]
    pub proxy: Option<String>,
    pub has_key: bool,
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

// ===== Stored records =====
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub app_id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub query: String,
    pub answer: String,
    pub feedback: Option<String>,
    pub retriever_resources: String,
    pub message_metadata: String,
    pub agent_thoughts: String,
    pub answer_tokens: i64,
    pub prompt_tokens: i64,
    pub elapsed_time: f64,
    pub created_at: i64,
    pub workflow_run_id: Option<String>,
    pub inputs: String,
    pub message_tokens: i64,
    pub provider_response_latency: f64,
    pub feedbacks: String,
    pub annotation: String,
    pub annotation_hit_history: String,
    pub message_files: String,
    pub status: String,
    pub error: String,
    pub parent_message_id: String,
    pub raw_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunDetail {
    pub id: String,
    pub workflow_run_id: String,
    pub workflow_id: String,
    pub status: String,
    pub version: String,
    pub graph: serde_json::Value,
    pub elapsed_time: f64,
    pub total_tokens: i64,
    pub total_steps: i64,
    pub created_at: i64,
    pub finished_at: i64,
    pub raw_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionDetail {
    pub id: String,
    pub execution_id: String,
    pub workflow_run_id: String,
    pub node_id: String,
    pub node_type: String,
    pub title: String,
    pub inputs: serde_json::Value,
    pub process_data: serde_json::Value,
    pub outputs: serde_json::Value,
    pub execution_metadata: serde_json::Value,
    pub extras: serde_json::Value,
    pub status: String,
    pub error: serde_json::Value,
    pub elapsed_time: f64,
    pub created_at: i64,
    pub finished_at: i64,
    pub raw_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDetail {
    pub id: String,
    pub app_id: String,
    pub message_id: String,
    pub conversation_id: String,
    pub query: String,
    pub answer: String,
    pub feedback: Option<String>,
    pub feedbacks: serde_json::Value,
    pub retriever_resources: serde_json::Value,
    pub message_metadata: serde_json::Value,
    pub agent_thoughts: serde_json::Value,
    pub inputs: serde_json::Value,
    pub message_files: serde_json::Value,
    pub annotation: serde_json::Value,
    pub annotation_hit_history: serde_json::Value,
    pub status: String,
    pub error: serde_json::Value,
    pub parent_message_id: String,
    pub workflow_run_id: Option<String>,
    pub workflow_run: Option<WorkflowRunDetail>,
    pub node_executions: Vec<NodeExecutionDetail>,
    pub raw_json: serde_json::Value,
    pub answer_tokens: i64,
    pub prompt_tokens: i64,
    pub message_tokens: i64,
    pub provider_response_latency: f64,
    pub elapsed_time: f64,
    pub created_at: i64,
}

// ===== Dify API Response Types =====
#[derive(Debug, Deserialize)]
pub struct DifyAppsResponse {
    #[serde(default)]
    pub data: Vec<DifyAppItem>,
}

#[derive(Debug, Deserialize)]
pub struct DifyAppItem {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
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
    #[serde(default)]
    pub data: Vec<DifyConversationItem>,
    #[serde(default)]
    pub has_more: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DifyConversationItem {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub inputs: serde_json::Value,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub introduction: String,
    #[serde(default)]
    pub from_source: String,
    #[serde(default)]
    pub from_end_user_id: String,
    #[serde(default)]
    pub from_end_user_session_id: String,
    #[serde(default)]
    pub read_at: Option<i64>,
    #[serde(default)]
    pub annotated: bool,
    #[serde(default)]
    pub model_config: serde_json::Value,
    #[serde(default)]
    pub user_feedback_stats: serde_json::Value,
    #[serde(default)]
    pub admin_feedback_stats: serde_json::Value,
    #[serde(default)]
    pub status_count: serde_json::Value,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(skip)]
    pub raw_json: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct DifyMessagesResponse {
    #[serde(default)]
    pub data: Vec<DifyMessageItem>,
    #[serde(default)]
    pub has_more: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DifyMessageItem {
    pub id: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub conversation_id: String,
    #[serde(default)]
    pub inputs: serde_json::Value,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub message: String,
    #[serde(default)]
    pub answer: String,
    #[serde(default)]
    pub feedback: Option<DifyFeedback>,
    #[serde(default)]
    pub feedbacks: serde_json::Value,
    #[serde(default)]
    pub retriever_resources: serde_json::Value,
    #[serde(default, rename = "metadata")]
    pub message_metadata: serde_json::Value,
    #[serde(default)]
    pub agent_thoughts: serde_json::Value,
    #[serde(default)]
    pub message_files: serde_json::Value,
    #[serde(default)]
    pub annotation: serde_json::Value,
    #[serde(default)]
    pub annotation_hit_history: serde_json::Value,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub error: serde_json::Value,
    #[serde(default)]
    pub parent_message_id: String,
    #[serde(default)]
    pub workflow_run_id: Option<String>,
    #[serde(default)]
    pub answer_tokens: i64,
    #[serde(default)]
    pub prompt_tokens: i64,
    #[serde(default)]
    pub message_tokens: i64,
    #[serde(default)]
    pub provider_response_latency: f64,
    #[serde(default)]
    pub elapsed_time: f64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(skip)]
    pub raw_json: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DifyFeedback {
    #[serde(default)]
    pub rating: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DifyWorkflowRun {
    pub id: String,
    #[serde(default)]
    pub workflow_id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub graph: serde_json::Value,
    #[serde(default)]
    pub elapsed_time: f64,
    #[serde(default)]
    pub total_tokens: i64,
    #[serde(default)]
    pub total_steps: i64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub finished_at: i64,
    #[serde(skip)]
    pub raw_json: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct DifyNodeExecutionsResponse {
    #[serde(default)]
    pub data: Vec<DifyNodeExecution>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DifyNodeExecution {
    pub id: String,
    #[serde(default)]
    pub index: i64,
    #[serde(default)]
    pub node_id: String,
    #[serde(default)]
    pub node_type: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub inputs: serde_json::Value,
    #[serde(default)]
    pub process_data: serde_json::Value,
    #[serde(default)]
    pub outputs: serde_json::Value,
    #[serde(default)]
    pub execution_metadata: serde_json::Value,
    #[serde(default)]
    pub extras: serde_json::Value,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub error: serde_json::Value,
    #[serde(default)]
    pub elapsed_time: f64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub finished_at: i64,
    #[serde(skip)]
    pub raw_json: serde_json::Value,
}

// ===== Sync Result =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub total_conversations: i64,
    pub synced_conversations: i64,
    pub total_messages: i64,
    pub synced_messages: i64,
    pub synced_workflow_runs: i64,
    pub synced_node_executions: i64,
    pub failed_details: i64,
    #[serde(default)]
    pub new_conversations: i64,
    #[serde(default)]
    pub updated_conversations: i64,
    #[serde(default)]
    pub skipped_conversations: i64,
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

// ===== Feedback Query Result =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackMessage {
    pub id: String,
    pub app_id: String,
    pub app_name: String,
    pub conversation_id: String,
    pub message_id: String,
    pub query: String,
    pub answer: String,
    pub feedback: Option<String>,
    pub feedbacks: serde_json::Value,
    pub answer_tokens: i64,
    pub prompt_tokens: i64,
    pub elapsed_time: f64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackResult {
    pub data: Vec<FeedbackMessage>,
    pub total: i64,
}
