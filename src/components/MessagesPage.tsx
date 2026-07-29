import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { MessageSquare, Search, ChevronLeft, ChevronRight, User, Clock, ThumbsUp, ThumbsDown, Minus, Loader2, GitBranch, AlertCircle, FileJson } from 'lucide-react';
import type { MessageDetail, MessagesResult } from '../types';

export function MessagesPage() {
  const [messages, setMessages] = useState<MessageDetail[]>([]);
  const [apps, setApps] = useState<{ id: string; name: string }[]>([]);
  const [selectedApp, setSelectedApp] = useState<string>('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [totalMessages, setTotalMessages] = useState(0);
  const [loading, setLoading] = useState(false);
  const [searchTrigger, setSearchTrigger] = useState(0);
  const [expandedMessage, setExpandedMessage] = useState<string | null>(null);
  const pageSize = 20;

  useEffect(() => {
    loadApps();
  }, []);

  useEffect(() => {
    loadMessages();
  }, [page, selectedApp, searchTrigger]);

  const loadApps = async () => {
    try {
      const result = await invoke<{ id: string; name: string }[]>('get_local_apps');
      setApps((result || []).map((a) => ({ id: a.id, name: a.name })));
    } catch (e) {
      console.error(e);
    }
  };

  const loadMessages = async () => {
    setLoading(true);
    try {
      const result = await invoke<MessagesResult>('get_all_messages', {
        appId: selectedApp || null,
        keyword: searchKeyword || null,
        page,
        pageSize,
      });
      setMessages(result.data || []);
      setTotalMessages(result.total || 0);
      setTotalPages(Math.ceil((result.total || 0) / pageSize));
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
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

  const toggleExpand = (msgId: string) => {
    setExpandedMessage(expandedMessage === msgId ? null : msgId);
  };

  return (
    <div className="h-full flex flex-col">
      <div className="mb-4">
        <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
          <MessageSquare size={24} />
          消息浏览
        </h2>
        <p className="text-gray-500 mt-1">按时间顺序查看所有消息（最新消息在前）</p>
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
            placeholder="搜索消息内容..."
            className="flex-1 px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <button
            onClick={handleSearch}
            className="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 flex items-center gap-2"
          >
            <Search size={14} />
            搜索
          </button>
        </div>
      </div>

      {/* Message List */}
      <div className="flex-1 overflow-auto bg-white rounded-xl border border-gray-200">
        {loading ? (
          <div className="flex items-center justify-center py-10">
            <Loader2 size={24} className="animate-spin text-blue-500" />
          </div>
        ) : messages.length === 0 ? (
          <div className="p-6 text-center text-gray-400 text-sm">
            暂无消息数据
          </div>
        ) : (
          <div className="divide-y divide-gray-100">
            {messages.map((msg, idx) => (
              <div key={msg.id || idx} className="p-4 hover:bg-gray-50 transition-colors">
                <div className="flex items-start gap-3">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-2">
                      <span className="text-xs font-medium text-blue-600 bg-blue-50 px-2 py-0.5 rounded">
                        {msg.app_id}
                      </span>
                      <span className="text-xs text-gray-400">
                        <Clock size={12} className="inline mr-1" />
                        {formatTime(msg.created_at)}
                      </span>
                      {getFeedbackIcon(msg.feedback)}
                      {msg.status && (
                        <span className={`text-xs px-2 py-0.5 rounded ${statusClass(msg.status)}`}>
                          {msg.status}
                        </span>
                      )}
                    </div>
                    <div className="mb-2">
                      <p className="text-sm font-medium text-gray-900 mb-1">
                        <User size={14} className="inline mr-1 text-blue-500" />
                        用户：{msg.query || '(无内容)'}
                      </p>
                      <p className="text-sm text-gray-700">
                        <MessageSquare size={14} className="inline mr-1 text-gray-400" />
                        AI：{msg.answer ? (msg.answer.length > 200 ? msg.answer.substring(0, 200) + '...' : msg.answer) : '(无内容)'}
                      </p>
                    </div>
                    <div className="flex items-center gap-4 text-xs text-gray-500">
                      <span>Tokens: {formatTokens(msg.prompt_tokens)}+{formatTokens(msg.answer_tokens)}</span>
                      {msg.elapsed_time != null && msg.elapsed_time > 0 && (
                        <span>{msg.elapsed_time.toFixed(2)}s</span>
                      )}
                      {msg.workflow_run_id && (
                        <span className="text-indigo-600">
                          <GitBranch size={12} className="inline mr-1" />
                          Workflow
                        </span>
                      )}
                    </div>
                  </div>
                  <button
                    onClick={() => toggleExpand(msg.id)}
                    className="px-3 py-1 text-xs text-gray-500 hover:text-gray-700 border border-gray-200 rounded hover:bg-gray-100 transition-colors"
                  >
                    {expandedMessage === msg.id ? '收起' : '展开'}
                  </button>
                </div>

                {/* Expanded Detail */}
                {expandedMessage === msg.id && (
                  <div className="mt-3 pt-3 border-t border-gray-200">
                    <div className="space-y-2 text-sm">
                      {hasJsonValue(msg.error) && (
                        <div className="flex items-start gap-2 rounded bg-red-50 px-3 py-2 text-xs text-red-700">
                          <AlertCircle size={14} className="mt-0.5 shrink-0" />
                          <pre className="whitespace-pre-wrap break-all">{formatJson(msg.error)}</pre>
                        </div>
                      )}
                      {hasJsonValue(msg.inputs) && (
                        <JsonBlock title="消息输入" value={msg.inputs} />
                      )}
                      {feedbackItems(msg).length > 0 && (
                        <div className="rounded bg-amber-50 p-2 text-xs">
                          <p className="mb-2 font-medium text-amber-700">反馈详情 ({feedbackItems(msg).length})</p>
                          <div className="space-y-2">
                            {feedbackItems(msg).map((feedback: any, i: number) => (
                              <div key={i} className="rounded border border-amber-100 bg-white p-2">
                                <div className="mb-1 flex flex-wrap items-center gap-2 text-amber-700">
                                  <span>#{i + 1}</span>
                                  {feedback.rating && <span>评分：{feedback.rating}</span>}
                                  {feedback.from_source && <span>来源：{feedback.from_source}</span>}
                                  {feedback.created_at && <span>时间：{formatTime(feedback.created_at)}</span>}
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
                        <details className="rounded bg-indigo-50 p-2 text-xs">
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
                        <details className="rounded bg-slate-50 p-2 text-xs">
                          <summary className="flex cursor-pointer items-center gap-1 font-medium text-slate-700">
                            <GitBranch size={14} />
                            节点执行过程 ({msg.node_executions.length})
                          </summary>
                          <div className="space-y-2 mt-2">
                            {msg.node_executions.map((node: any, i: number) => (
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
                      {msg.agent_thoughts && Array.isArray(msg.agent_thoughts) && msg.agent_thoughts.length > 0 && (
                        <div className="rounded bg-gray-50 p-2 text-xs">
                          <p className="font-medium text-gray-500 mb-1">思维链 ({msg.agent_thoughts.length} 步)</p>
                          {msg.agent_thoughts.map((thought: any, i: number) => (
                            <p key={i} className="text-gray-500 my-1">
                              <span className="text-gray-400">Step {i + 1}:</span>{' '}
                              {thought.thought || thought.tool || JSON.stringify(thought)}
                            </p>
                          ))}
                        </div>
                      )}
                      {msg.retriever_resources && Array.isArray(msg.retriever_resources) && msg.retriever_resources.length > 0 && (
                        <div className="rounded bg-green-50 p-2 text-xs">
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
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="mt-4 flex items-center justify-between">
          <span className="text-sm text-gray-500">
            共 {totalMessages} 条消息，第 {page} / {totalPages} 页
          </span>
          <div className="flex gap-2">
            <button
              onClick={() => setPage(Math.max(1, page - 1))}
              disabled={page <= 1}
              className="px-3 py-1 text-sm text-gray-600 hover:text-gray-800 disabled:opacity-30 flex items-center gap-1"
            >
              <ChevronLeft size={18} />
              上一页
            </button>
            <button
              onClick={() => setPage(Math.min(totalPages, page + 1))}
              disabled={page >= totalPages}
              className="px-3 py-1 text-sm text-gray-600 hover:text-gray-800 disabled:opacity-30 flex items-center gap-1"
            >
              下一页
              <ChevronRight size={18} />
            </button>
          </div>
        </div>
      )}
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
