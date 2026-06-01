// ===== Page State =====
export type Page = 'config' | 'apps' | 'sync' | 'conversations' | 'dashboard' | 'export' | 'feedback';

// ===== Config =====
export interface DifyConfig {
  api_base: string;
  api_key: string;
  proxy?: string;
}

export interface DifyConfigDisplay {
  api_base: string;
  api_key_masked: string;
  proxy?: string;
  has_key: boolean;
  auth_mode: string;
  auth_email?: string;
}

// ===== App =====
export interface DifyApp {
  id: string;
  name: string;
  description: string;
  mode: string; // 'chat' | 'completion' | 'workflow' | 'agent-chat'
  icon: string;
  icon_background: string;
  created_at: number;
}

// ===== Conversation =====
export interface ConversationSummary {
  id: string;
  app_id: string;
  conversation_id: string;
  name: string;
  created_at: number;
  updated_at: number;
  message_count: number;
  app_name: string;
}

export interface ConversationsResult {
  data: ConversationSummary[];
  total: number;
}

// ===== Message =====
export interface MessageDetail {
  id: string;
  app_id: string;
  message_id: string;
  conversation_id: string;
  query: string;
  answer: string;
  feedback: string | null; // 'like' | 'dislike' | null
  feedbacks: any[];
  retriever_resources: any[];
  message_metadata: Record<string, any>;
  agent_thoughts: any[];
  inputs: Record<string, any>;
  message_files: any[];
  annotation: any;
  annotation_hit_history: any;
  status: string;
  error: any;
  parent_message_id: string;
  workflow_run_id: string | null;
  workflow_run: WorkflowRunDetail | null;
  node_executions: NodeExecutionDetail[];
  raw_json: Record<string, any>;
  answer_tokens: number;
  prompt_tokens: number;
  message_tokens: number;
  provider_response_latency: number;
  elapsed_time?: number;
  created_at: number;
}

export interface WorkflowRunDetail {
  id: string;
  workflow_run_id: string;
  workflow_id: string;
  status: string;
  version: string;
  graph: Record<string, any>;
  elapsed_time: number;
  total_tokens: number;
  total_steps: number;
  created_at: number;
  finished_at: number;
  raw_json: Record<string, any>;
}

export interface NodeExecutionDetail {
  id: string;
  execution_id: string;
  workflow_run_id: string;
  node_id: string;
  node_type: string;
  title: string;
  inputs: any;
  process_data: any;
  outputs: any;
  execution_metadata: any;
  extras: any;
  status: string;
  error: any;
  elapsed_time: number;
  created_at: number;
  finished_at: number;
  raw_json: any;
}

// ===== Sync State =====
export interface SyncResult {
  total_conversations: number;
  synced_conversations: number;
  total_messages: number;
  synced_messages: number;
  synced_workflow_runs: number;
  synced_node_executions: number;
  failed_details: number;
  new_conversations?: number;
  updated_conversations?: number;
  skipped_conversations?: number;
}

// ===== Dashboard Stats =====
export interface StatDistribution {
  min: number;
  max: number;
  avg: number;
  p50: number;
  p80: number;
  p95: number;
  count: number;
}

export interface DashboardStats {
  // Basic counts
  total_apps: number;
  total_users: number;
  total_conversations: number;
  total_messages: number;
  // Token totals
  total_answer_tokens: number;
  total_prompt_tokens: number;
  total_tokens: number;
  daily_avg_tokens: number;
  // Averages
  avg_messages_per_conversation: number;
  avg_conversations_per_user: number;
  avg_messages_per_user: number;
  // Feedback counts
  feedback_total: number;
  feedback_like: number;
  feedback_dislike: number;
  feedback_none: number;
  feedback_with_content: number;
  feedback_like_rate: number;
  avg_feedback_per_user: number;
  avg_feedback_per_conversation: number;
  avg_feedback_per_message: number;
  // Error stats
  error_count: number;
  error_rate: number;
  // Distributions
  ttft_distribution: StatDistribution | null;
  elapsed_time_distribution: StatDistribution | null;
  token_per_message_distribution: StatDistribution | null;
  token_speed_distribution: StatDistribution | null;
  user_feedback_count_distribution: StatDistribution | null;
  conversation_feedback_count_distribution: StatDistribution | null;
  message_feedback_count_distribution: StatDistribution | null;
  // Feedback label stats
  feedback_label_stats: FeedbackLabelStat[];
  // Rankings & trends
  top_apps: AppRanking[];
  recent_daily: DailyStats[];
}

export interface AppRanking {
  app_id: string;
  app_name: string;
  conversation_count: number;
  message_count: number;
}

export interface DailyStats {
  date: string;
  conversations: number;
  messages: number;
  tokens: number;
  users: number;
}

export interface FeedbackLabelStat {
  feedback: string;
  count: number;
}

// ===== Auto Sync Settings =====
export interface AutoSyncSettings {
  enabled: boolean;
  interval_minutes: number;
  mode: string;
  last_synced_at?: number | null;
}

// ===== Export Options =====
export interface ExportOptions {
  format: 'json' | 'csv' | 'jsonl';
  app_id?: string;
  start_date?: string;
  end_date?: string;
  keyword?: string;
  include_metadata: boolean;
  include_agent_thoughts: boolean;
}

// ===== Feedback =====
export interface FeedbackMessage {
  id: string;
  app_id: string;
  app_name: string;
  conversation_id: string;
  message_id: string;
  query: string;
  answer: string;
  feedback: string | null;
  feedbacks: any[];
  answer_tokens: number;
  prompt_tokens: number;
  elapsed_time: number;
  created_at: number;
}

export interface FeedbackResult {
  data: FeedbackMessage[];
  total: number;
}
