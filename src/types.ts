// ===== Page State =====
export type Page = 'config' | 'apps' | 'sync' | 'conversations' | 'dashboard' | 'export';

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
  message_id: string;
  conversation_id: string;
  query: string;
  answer: string;
  feedback: string | null; // 'like' | 'dislike' | null
  retriever_resources: any[];
  message_metadata: Record<string, any>;
  agent_thoughts: any[];
  answer_tokens: number;
  prompt_tokens: number;
  elapsed_time?: number;
  created_at: number;
}

// ===== Sync State =====
export interface SyncResult {
  total_conversations: number;
  synced_conversations: number;
  total_messages: number;
  synced_messages: number;
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