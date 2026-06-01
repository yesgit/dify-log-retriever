import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { MessageSquare, Search, ChevronLeft, ChevronRight, User, Clock, ThumbsUp, ThumbsDown, Minus, Loader2, GitBranch, AlertCircle, FileJson } from 'lucide-react';
import type { ConversationSummary, MessageDetail, ConversationsResult, NodeExecutionDetail } from '../types';

export function ConversationsPage() {
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageDetail[]>([]);
  const [apps, setApps] = useState<{ id: string; name: string }[]>([]);
  const [selectedApp, setSelectedApp] = useState<string>('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [loading, setLoading] = useState(false);
  const [msgLoading, setMsgLoading] = useState(false);
  const [searchTrigger, setSearchTrigger] = useState(0);
  const pageSize = 20;

  useEffect(() => {
    loadApps();
  }, []);

  useEffect(() => {
    loadConversations();
  }, [page, selectedApp, searchTrigger]);

  const loadApps = async () => {
    try {
      const result = await invoke<{ id: string; name: string }[]>('get_local_apps');
      setApps((result || []).map((a) => ({ id: a.id, name: a.name })));
    } catch (e) {
      console.error(e);
    }
  };

  const loadConversations = async () => {
    setLoading(true);
    try {
      const result = await invoke<ConversationsResult>('get_conversations', {
        appId: selectedApp || null,
        keyword: searchKeyword || null,
        page,
        pageSize,
      });
      setConversations(result.data || []);
      setTotalPages(Math.ceil((result.total || 0) / pageSize));
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const loadMessages = async (conv: ConversationSummary) => {
    setSelectedConversation(conv.conversation_id);
    setMsgLoading(true);
    try {
      const result = await invoke<MessageDetail[]>('get_messages', {
        conversationId: conv.conversation_id,
        appId: conv.app_id,
      });
      setMessages(result || []);
    } catch (e) {
      console.error(e);
    } finally {
      setMsgLoading(false);
    }
  };

  const handleSearch = () => {
    setPage(1);
    setSearchTrigger((prev) => prev + 1);
  };

  const formatTime = (ts: number) => {
    if (!ts) return '-';
    return new Date(ts * 1000).toLocaleString('zh-CN');
  };

  const formatTokens = (n: number) => {
    if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
    return n.toString();
  };

  const getFeedbackIcon = (feedback: string | null) => {
    if (feedback === 'like') return <ThumbsUp size={14} className="text-green-500" />;
    if (feedback === 'dislike') return <ThumbsDown size={14} className="text-red-500" />;
    return <Minus size={14} className="text-gray-300" />;
  };

  const feedbackItems = (msg: MessageDetail) => Array.isArray(msg.feedbacks) ? msg.feedbacks : [];

  const hasJsonValue = (value: any) => {
    if (value == null) return false;
    if (Array.isArray(value)) return value.length > 0;
    if (typeof value === 'object') return Object.keys(value).length > 0;
    return value !== '';
  };

  const formatJson = (value: any) => {
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  };

  const statusClass = (status?: string) => {
    if (status === 'succeeded' || status === 'success' || status === 'normal') return 'bg-green-50 text-green-700';
    if (status === 'failed' || status === 'error') return 'bg-red-50 text-red-700';
    if (status === 'running') return 'bg-blue-50 text-blue-700';
    return 'bg-gray-50 text-gray-600';
  };

  return (
    <div className="h-full flex flex-col">
      <div className="mb-4">
        <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
          <MessageSquare size={24} />
          对话浏览
        </h2>
        <p className="text-gray-500 mt-1">浏览已同步的对话记录和消息详情</p>
      </div>

      {/* Filters */}
      <div className="flex gap-3 mb-4">
        <select
          value={selectedApp}
          onChange={(e) => { setSelectedApp(e.target.value); setPage(1); }}
          className="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">全部应用</option>
          {apps.map((app) => (
            <option key={app.id} value={app.id}>{app.name}</option>
          ))}
        </select>
        <div className="flex-1 flex gap-2">
          <input
            type="text"
            value={searchKeyword}
            onChange={(e) => setSearchKeyword(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
            placeholder="搜索对话内容..."
            className="flex-1 px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <button
            onClick={handleSearch}
            className="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 flex items-center gap-1"
          >
            <Search size={14} />
            搜索
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 flex gap-4 min-h-0">
        {/* Conversation List */}
        <div className="w-96 flex flex-col bg-white rounded-xl border border-gray-200 overflow-hidden">
          <div className="px-4 py-3 border-b border-gray-100 bg-gray-50">
            <span className="text-sm font-medium text-gray-600">对话列表</span>
          </div>
          <div className="flex-1 overflow-auto">
            {loading ? (
              <div className="flex items-center justify-center py-10">
                <Loader2 size={24} className="animate-spin text-blue-500" />
              </div>
            ) : conversations.length === 0 ? (
              <div className="p-6 text-center text-gray-400 text-sm">
                暂无对话数据
              </div>
            ) : (
              conversations.map((conv) => (
                <div
                  key={conv.id}
                  onClick={() => loadMessages(conv)}
                  className={`px-4 py-3 border-b border-gray-50 cursor-pointer hover:bg-blue-50 transition-colors ${
                    selectedConversation === conv.conversation_id ? 'bg-blue-50 border-l-2 border-l-blue-500' : ''
                  }`}
                >
                  <div className="flex items-start justify-between">
                    <div className="min-w-0 flex-1">
                      <p className="text-sm font-medium text-gray-900 truncate">
                        {conv.name || conv.conversation_id}
                      </p>
                      <p className="text-xs text-gray-400 mt-1">
                        {conv.app_name} · {conv.message_count} 条消息
                      </p>
                    </div>
                    <span className="text-xs text-gray-400 whitespace-nowrap ml-2">
                      {formatTime(conv.created_at)}
                    </span>
                  </div>
                </div>
              ))
            )}
          </div>
          {/* Pagination */}
          {totalPages > 1 && (
            <div className="px-4 py-2 border-t border-gray-100 flex items-center justify-between">
              <button
                onClick={() => setPage(Math.max(1, page - 1))}
                disabled={page <= 1}
                className="p-1 text-gray-400 hover:text-gray-600 disabled:opacity-30"
              >
                <ChevronLeft size={18} />
              </button>
              <span className="text-xs text-gray-500">{page} / {totalPages}</span>
              <button
                onClick={() => setPage(Math.min(totalPages, page + 1))}
                disabled={page >= totalPages}
                className="p-1 text-gray-400 hover:text-gray-600 disabled:opacity-30"
              >
                <ChevronRight size={18} />
              </button>
            </div>
          )}
        </div>

        {/* Message Detail */}
        <div className="flex-1 bg-white rounded-xl border border-gray-200 overflow-hidden flex flex-col">
          <div className="px-4 py-3 border-b border-gray-100 bg-gray-50">
            <span className="text-sm font-medium text-gray-600">消息详情</span>
          </div>
          <div className="flex-1 overflow-auto p-4">
            {!selectedConversation ? (
              <div className="flex items-center justify-center h-full text-gray-400 text-sm">
                请选择一个对话查看详情
              </div>
            ) : msgLoading ? (
              <div className="flex items-center justify-center py-10">
                <Loader2 size={24} className="animate-spin text-blue-500" />
              </div>
            ) : messages.length === 0 ? (
              <div className="text-center text-gray-400 text-sm py-10">无消息</div>
            ) : (
              <div className="space-y-4">
                {messages.map((msg, idx) => (
                  <div key={msg.id || idx} className="border border-gray-100 rounded-lg overflow-hidden">
                    {/* Query */}
                    <div className="bg-blue-50 px-4 py-3">
                      <div className="flex items-center gap-2 mb-1">
                        <User size={14} className="text-blue-600" />
                        <span className="text-xs font-medium text-blue-600">用户提问</span>
                        <span className="text-xs text-blue-400 ml-auto">
                          <Clock size={12} className="inline mr-1" />
                          {formatTime(msg.created_at)}
                        </span>
                      </div>
                      <p className="text-sm text-gray-800 whitespace-pre-wrap">{msg.query}</p>
                    </div>
                    {/* Answer */}
                    <div className="px-4 py-3">
                      <div className="flex items-center gap-2 mb-1">
                        <MessageSquare size={14} className="text-gray-500" />
                        <span className="text-xs font-medium text-gray-500">AI 回答</span>
                        <div className="ml-auto flex items-center gap-3">
                          {getFeedbackIcon(msg.feedback)}
                          <span className="text-xs text-gray-400">
                            Tokens: {formatTokens(msg.prompt_tokens)}+{formatTokens(msg.answer_tokens)}
                          </span>
                          {msg.elapsed_time != null && msg.elapsed_time > 0 && (
                            <span className="text-xs text-gray-400">
                              {msg.elapsed_time.toFixed(2)}s
                            </span>
                          )}
                        </div>
                      </div>
                      <p className="text-sm text-gray-800 whitespace-pre-wrap">{msg.answer}</p>
                      <div className="mt-3 flex flex-wrap items-center gap-2 text-xs">
                        {msg.status && (
                          <span className={`px-2 py-1 rounded ${statusClass(msg.status)}`}>状态: {msg.status}</span>
                        )}
                        {msg.workflow_run_id && (
                          <span className="px-2 py-1 rounded bg-indigo-50 text-indigo-700">
                            Workflow: {msg.workflow_run_id}
                          </span>
                        )}
                        {msg.provider_response_latency > 0 && (
                          <span className="px-2 py-1 rounded bg-gray-50 text-gray-600">
                            模型延迟: {msg.provider_response_latency.toFixed(2)}s
                          </span>
                        )}
                        {msg.message_tokens > 0 && (
                          <span className="px-2 py-1 rounded bg-gray-50 text-gray-600">
                            Message Tokens: {formatTokens(msg.message_tokens)}
                          </span>
                        )}
                      </div>
                      {hasJsonValue(msg.error) && (
                        <div className="mt-2 flex items-start gap-2 rounded bg-red-50 px-3 py-2 text-xs text-red-700">
                          <AlertCircle size={14} className="mt-0.5 shrink-0" />
                          <pre className="whitespace-pre-wrap break-all">{formatJson(msg.error)}</pre>
                        </div>
                      )}
                      {hasJsonValue(msg.inputs) && (
                        <JsonBlock title="消息输入" value={msg.inputs} />
                      )}
                      {feedbackItems(msg).length > 0 && (
                        <div className="mt-2 rounded bg-amber-50 p-2 text-xs">
                          <p className="mb-2 font-medium text-amber-700">反馈详情 ({feedbackItems(msg).length})</p>
                          <div className="space-y-2">
                            {feedbackItems(msg).map((feedback: any, i: number) => (
                              <div key={i} className="rounded border border-amber-100 bg-white p-2">
                                <div className="mb-1 flex flex-wrap items-center gap-2 text-amber-700">
                                  <span>#{i + 1}</span>
                                  {feedback.rating && <span>评分: {feedback.rating}</span>}
                                  {feedback.from_source && <span>来源: {feedback.from_source}</span>}
                                  {feedback.created_at && <span>时间: {formatTime(feedback.created_at)}</span>}
                                </div>
                                <pre className="max-h-40 overflow-auto whitespace-pre-wrap break-all text-gray-600">
                                  {formatJson(feedback)}
                                </pre>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}
                      {msg.workflow_run && (
                        <details className="mt-2 rounded bg-indigo-50 p-2 text-xs">
                          <summary className="flex cursor-pointer flex-wrap items-center gap-2 text-indigo-700">
                            <GitBranch size={14} />
                            <span className="font-medium">Workflow Run</span>
                            <span className={`px-2 py-0.5 rounded ${statusClass(msg.workflow_run.status)}`}>{msg.workflow_run.status || '-'}</span>
                            <span>{msg.workflow_run.elapsed_time.toFixed(2)}s</span>
                            <span>Steps: {msg.workflow_run.total_steps}</span>
                            <span>Tokens: {formatTokens(msg.workflow_run.total_tokens)}</span>
                          </summary>
                          <JsonBlock title="Workflow 图定义" value={msg.workflow_run.graph} compact />
                        </details>
                      )}
                      {msg.node_executions && msg.node_executions.length > 0 && (
                        <details className="mt-2 rounded bg-slate-50 p-2 text-xs">
                          <summary className="flex cursor-pointer items-center gap-1 font-medium text-slate-700">
                            <GitBranch size={14} />
                            节点执行过程 ({msg.node_executions.length})
                          </summary>
                          <div className="space-y-2 mt-2">
                            {msg.node_executions.map((node: NodeExecutionDetail, i: number) => (
                              <div key={node.id || i} className="rounded border border-slate-200 bg-white p-3">
                                <div className="mb-2 flex flex-wrap items-center gap-2">
                                  <span className="font-medium text-slate-800">#{i + 1} {node.title || node.node_id}</span>
                                  <span className="rounded bg-slate-100 px-2 py-0.5 text-slate-600">{node.node_type || '-'}</span>
                                  <span className={`rounded px-2 py-0.5 ${statusClass(node.status)}`}>{node.status || '-'}</span>
                                  {node.elapsed_time > 0 && <span className="text-slate-500">{node.elapsed_time.toFixed(2)}s</span>}
                                  {node.created_at > 0 && <span className="text-slate-400">{formatTime(node.created_at)}</span>}
                                </div>
                                <div className="grid grid-cols-1 gap-2 xl:grid-cols-2">
                                  <JsonBlock title="输入" value={node.inputs} compact />
                                  <JsonBlock title="过程数据" value={node.process_data} compact />
                                  <JsonBlock title="输出" value={node.outputs} compact />
                                  <JsonBlock title="元数据" value={node.execution_metadata} compact />
                                </div>
                                {hasJsonValue(node.error) && <JsonBlock title="错误" value={node.error} compact />}
                              </div>
                            ))}
                          </div>
                        </details>
                      )}
                      {/* Agent Thoughts */}
                      {msg.agent_thoughts && Array.isArray(msg.agent_thoughts) && msg.agent_thoughts.length > 0 && (
                        <div className="mt-2 p-2 bg-gray-50 rounded text-xs">
                          <p className="font-medium text-gray-500 mb-1">思维链 ({msg.agent_thoughts.length} 步)</p>
                          {msg.agent_thoughts.map((thought: any, i: number) => (
                            <p key={i} className="text-gray-500 my-1">
                              <span className="text-gray-400">Step {i + 1}:</span>{' '}
                              {thought.thought || thought.tool || JSON.stringify(thought)}
                            </p>
                          ))}
                        </div>
                      )}
                      {/* Retriever Resources */}
                      {msg.retriever_resources && Array.isArray(msg.retriever_resources) && msg.retriever_resources.length > 0 && (
                        <div className="mt-2 p-2 bg-green-50 rounded text-xs">
                          <p className="font-medium text-green-600 mb-1">
                            引用资源 ({msg.retriever_resources.length})
                          </p>
                          {msg.retriever_resources.map((res: any, i: number) => (
                            <p key={i} className="text-green-600 my-0.5 truncate">
                              [{i + 1}] {res.segment_id || res.datasource_id || JSON.stringify(res).slice(0, 80)}
                            </p>
                          ))}
                        </div>
                      )}
                      <JsonBlock title="原始消息 JSON" value={msg.raw_json} />
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function JsonBlock({ title, value, compact = false }: { title: string; value: any; compact?: boolean }) {
  if (value == null) return null;
  if (Array.isArray(value) && value.length === 0) return null;
  if (typeof value === 'object' && !Array.isArray(value) && Object.keys(value).length === 0) return null;

  return (
    <details className={`mt-2 rounded border border-gray-100 bg-white ${compact ? 'p-2' : 'p-3'}`}>
      <summary className="flex cursor-pointer items-center gap-1 text-xs font-medium text-gray-600">
        <FileJson size={13} />
        {title}
      </summary>
      <pre className={`${compact ? 'max-h-48' : 'max-h-80'} mt-2 overflow-auto whitespace-pre-wrap break-all rounded bg-gray-50 p-2 text-xs text-gray-600`}>
        {JSON.stringify(value, null, 2)}
      </pre>
    </details>
  );
}
