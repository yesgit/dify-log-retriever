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
}

// ===== Dashboard Stats =====
export interface DashboardStats {
  total_apps: number;
  total_conversations: number;
  total_messages: number;
  total_answer_tokens: number;
  total_prompt_tokens: number;
  feedback_like: number;
  feedback_dislike: number;
  feedback_none: number;
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
