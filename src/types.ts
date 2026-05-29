// ===== Settings =====
export interface DifyConfig {
  api_base: string;
  api_key: string;
}

// ===== App =====
export interface DifyApp {
  id: string;
  name: string;
  description: string;
  mode: string; // 'chat' | 'completion' | 'workflow'
  icon: string;
  icon_background: string;
  created_at: number;
}

// ===== Conversation =====
export interface Conversation {
  id: string;
  app_id: string;
  conversation_id: string;
  name: string;
  inputs: Record<string, any>;
  status: string;
  introduction: string;
  created_at: number;
  updated_at: number;
  _synced_at?: number;
}

// ===== Message =====
export interface Message {
  id: string;
  app_id: string;
  conversation_id: string;
  message_id: string;
  query: string;
  answer: string;
  feedback: string | null; // 'like' | 'dislike' | null
  retriever_resources: any[];
  message_metadata: Record<string, any>;
  agent_thoughts: any[];
  answer_tokens: number;
  prompt_tokens: number;
  created_at: number;
  _synced_at?: number;
}

// ===== Sync State =====
export interface SyncState {
  app_id: string;
  app_name: string;
  status: 'idle' | 'syncing' | 'completed' | 'error';
  total_conversations: number;
  synced_conversations: number;
  total_messages: number;
  synced_messages: number;
  error_message?: string;
  started_at?: number;
  completed_at?: number;
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
  top_apps: { app_id: string; app_name: string; conversation_count: number; message_count: number }[];
  recent_daily: { date: string; conversations: number; messages: number }[];
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

// ===== Page State =====
export type Page = 'config' | 'apps' | 'sync' | 'conversations' | 'dashboard' | 'export';