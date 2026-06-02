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
    #[serde(default)]
    pub auth_email: Option<String>,
    #[serde(default)]
    pub auth_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifyConfigDisplay {
    pub api_base: String,
    pub api_key_masked: String,
    #[serde(default)]
    pub proxy: Option<String>,
    pub has_key: bool,
    #[serde(default)]
    pub auth_mode: String,
    #[serde(default)]
    pub auth_email: Option<String>,
}

// ===== Login =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
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
pub struct StatDistribution {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub p50: f64,
    pub p80: f64,
    pub p95: f64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    // Basic counts
    pub total_apps: i64,
    pub total_users: i64,
    pub total_conversations: i64,
    pub total_messages: i64,
    // Token totals
    pub total_answer_tokens: i64,
    pub total_prompt_tokens: i64,
    pub total_tokens: i64,
    pub daily_avg_tokens: f64,
    // Averages
    pub messages_per_conversation_distribution: Option<StatDistribution>,
    pub conversations_per_user_distribution: Option<StatDistribution>,
    pub messages_per_user_distribution: Option<StatDistribution>,
    // Feedback counts
    pub feedback_total: i64,
    pub feedback_like: i64,
    pub feedback_dislike: i64,
    pub feedback_none: i64,
    pub feedback_with_content: i64,
    pub feedback_like_rate: f64,
    pub avg_feedback_per_user: f64,
    pub avg_feedback_per_conversation: f64,
    pub avg_feedback_per_message: f64,
    // Error stats
    pub error_count: i64,
    pub error_rate: f64,
    // Dify-aligned metrics
    pub satisfaction_rate: f64,                // 用户满意度 = likes / messages * 1000
    pub avg_conversation_interactions: f64,    // 平均会话互动数 = messages / conversations
    // Distributions
    pub ttft_distribution: Option<StatDistribution>,       // 首Token时间
    pub elapsed_time_distribution: Option<StatDistribution>, // 总响应时间
    pub token_per_message_distribution: Option<StatDistribution>, // 每条消息Token消耗
    pub token_speed_distribution: Option<StatDistribution>,  // Token生成速度 (tokens/s)
    pub user_feedback_count_distribution: Option<StatDistribution>, // 用户反馈数分布
    pub conversation_feedback_count_distribution: Option<StatDistribution>, // 会话反馈数分布
    pub message_feedback_count_distribution: Option<StatDistribution>, // 消息反馈数分布
    // Feedback label stats
    pub feedback_label_stats: Vec<FeedbackLabelStat>,
    // Rankings & trends
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
    #[serde(default)]
    pub tokens: i64,
    #[serde(default)]
    pub users: i64,
    // Extended fields (aligned with Dify monitoring)
    #[serde(default)]
    pub errors: i64,
    #[serde(default)]
    pub likes: i64,
    #[serde(default)]
    pub dislikes: i64,
    #[serde(default)]
    pub avg_elapsed_time: f64,
    #[serde(default)]
    pub avg_ttft: f64,
    #[serde(default)]
    pub avg_token_speed: f64,
    #[serde(default)]
    pub total_answer_tokens: i64,
    #[serde(default)]
    pub total_prompt_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackLabelStat {
    pub feedback: String,
    pub count: i64,
}

// ===== Conversations Query Result =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationsResult {
    pub data: Vec<ConversationSummary>,
    pub total: i64,
}

// ===== Node Eval Export =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEvalRecord {
    pub execution_id: String,
    pub workflow_run_id: String,
    pub node_id: String,
    pub node_type: String,
    pub node_title: String,
    pub app_id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub query: String,
    pub inputs: serde_json::Value,
    pub outputs: serde_json::Value,
    pub process_data: serde_json::Value,
    pub status: String,
    pub elapsed_time: f64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTypeSummary {
    pub node_type: String,
    pub node_id: String,
    pub node_title: String,
    pub count: i64,
}

// ===== Workflow App Logs =====
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DifyWorkflowAppLogsResponse {
    #[serde(default)]
    pub data: Vec<DifyWorkflowAppLogItem>,
    #[serde(default)]
    pub has_more: bool,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub limit: i64,
    #[serde(default)]
    pub page: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DifyWorkflowAppLogItem {
    pub id: String,
    #[serde(default)]
    pub workflow_run: DifyWorkflowAppLogRunSummary,
    #[serde(default, rename = "created_from")]
    pub created_from: String,
    #[serde(default, rename = "created_by_role")]
    pub created_by_role: String,
    #[serde(default, rename = "created_by_account")]
    pub created_by_account: Option<serde_json::Value>,
    #[serde(default, rename = "created_by_end_user")]
    pub created_by_end_user: Option<DifyWorkflowAppLogEndUser>,
    #[serde(default)]
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct DifyWorkflowAppLogRunSummary {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
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
    #[serde(default)]
    pub exceptions_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DifyWorkflowAppLogEndUser {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    #[serde(rename = "type")]
    pub end_user_type: String,
    #[serde(default)]
    pub is_anonymous: bool,
    #[serde(default)]
    pub session_id: String,
}

// ===== Auto Sync Settings =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSyncSettings {
    pub enabled: bool,
    pub interval_minutes: i64,
    pub mode: String, // "incremental" or "full"
    #[serde(default)]
    pub last_synced_at: Option<i64>,
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
