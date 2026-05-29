import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { MessageSquare, Search, ChevronLeft, ChevronRight, User, Clock, ThumbsUp, ThumbsDown, Minus, Loader2 } from 'lucide-react';
import type { ConversationSummary, MessageDetail, ConversationsResult } from '../types';

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

  const loadMessages = async (conversationId: string) => {
    setSelectedConversation(conversationId);
    setMsgLoading(true);
    try {
      const result = await invoke<MessageDetail[]>('get_messages', {
        conversationId,
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
                  onClick={() => loadMessages(conv.conversation_id)}
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