import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  ThumbsUp,
  ThumbsDown,
  Search,
  ChevronLeft,
  ChevronRight,
  User,
  MessageSquare,
  Clock,
  Loader2,
  FileSpreadsheet,
  FileJson,
  FileText,
} from 'lucide-react';
import type { FeedbackMessage, FeedbackResult } from '../types';

export function FeedbackPage() {
  const [feedbacks, setFeedbacks] = useState<FeedbackMessage[]>([]);
  const [selectedFeedback, setSelectedFeedback] = useState<FeedbackMessage | null>(null);
  const [apps, setApps] = useState<{ id: string; name: string }[]>([]);
  const [selectedApp, setSelectedApp] = useState<string>('');
  const [feedbackType, setFeedbackType] = useState<string>('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportMsg, setExportMsg] = useState<string | null>(null);
  const [searchTrigger, setSearchTrigger] = useState(0);
  const pageSize = 20;
  const exportTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    loadApps();
  }, []);

  useEffect(() => {
    setSelectedFeedback(null);
    loadFeedbacks();
  }, [page, selectedApp, feedbackType, searchTrigger]);

  useEffect(() => {
    return () => {
      if (exportTimerRef.current) {
        clearTimeout(exportTimerRef.current);
      }
    };
  }, []);

  const loadApps = async () => {
    try {
      const result = await invoke<{ id: string; name: string }[]>('get_local_apps');
      setApps((result || []).map((a) => ({ id: a.id, name: a.name })));
    } catch (e) {
      console.error(e);
    }
  };

  const loadFeedbacks = async () => {
    setLoading(true);
    try {
      const result = await invoke<FeedbackResult>('get_feedback_messages', {
        appId: selectedApp || null,
        feedbackType: feedbackType || null,
        keyword: searchKeyword || null,
        page,
        pageSize,
      });
      setFeedbacks(result.data || []);
      setTotal(result.total || 0);
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

  const handleExport = async (format: string) => {
    setExporting(true);
    setExportMsg(null);
    try {
      const msg = await invoke<string>('export_feedback_data', {
        format,
        appId: selectedApp || null,
        feedbackType: feedbackType || null,
        keyword: searchKeyword || null,
      });
      setExportMsg(msg);
    } catch (e: any) {
      setExportMsg(`导出失败: ${e}`);
    } finally {
      setExporting(false);
      if (exportTimerRef.current) {
        clearTimeout(exportTimerRef.current);
      }
      exportTimerRef.current = setTimeout(() => setExportMsg(null), 5000);
    }
  };

  const formatTime = (ts: number) => {
    if (!ts) return '-';
    return new Date(ts * 1000).toLocaleString('zh-CN');
  };

  const truncate = (s: string, maxLen: number) => {
    if (!s) return '-';
    return s.length > maxLen ? s.slice(0, maxLen) + '...' : s;
  };

  return (
    <div className="h-full flex flex-col">
      <div className="mb-4">
        <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
          <ThumbsUp size={24} />
          用户反馈
        </h2>
        <p className="text-gray-500 mt-1">查看和管理用户对 AI 回答的反馈评价</p>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-3 mb-4">
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
        <select
          value={feedbackType}
          onChange={(e) => { setFeedbackType(e.target.value); setPage(1); }}
          className="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">全部反馈</option>
          <option value="like">👍 赞</option>
          <option value="dislike">👎 踩</option>
        </select>
        <div className="flex-1 flex gap-2">
          <input
            type="text"
            value={searchKeyword}
            onChange={(e) => setSearchKeyword(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
            placeholder="搜索提问或回答内容..."
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

      {/* Export Bar */}
      <div className="flex items-center gap-3 mb-4">
        <span className="text-sm text-gray-500">共 {total} 条反馈</span>
        <div className="flex-1" />
        <span className="text-xs text-gray-400">导出：</span>
        <button
          onClick={() => handleExport('xlsx')}
          disabled={exporting || total === 0}
          className="px-3 py-1.5 bg-green-600 text-white text-xs rounded-lg hover:bg-green-700 disabled:opacity-40 flex items-center gap-1"
        >
          <FileSpreadsheet size={13} />
          Excel
        </button>
        <button
          onClick={() => handleExport('csv')}
          disabled={exporting || total === 0}
          className="px-3 py-1.5 bg-emerald-600 text-white text-xs rounded-lg hover:bg-emerald-700 disabled:opacity-40 flex items-center gap-1"
        >
          <FileText size={13} />
          CSV
        </button>
        <button
          onClick={() => handleExport('json')}
          disabled={exporting || total === 0}
          className="px-3 py-1.5 bg-amber-600 text-white text-xs rounded-lg hover:bg-amber-700 disabled:opacity-40 flex items-center gap-1"
        >
          <FileJson size={13} />
          JSON
        </button>
        {exporting && <Loader2 size={16} className="animate-spin text-blue-500" />}
        {exportMsg && (
          <span className={`text-xs ${exportMsg.startsWith('导出失败') ? 'text-red-500' : 'text-green-600'}`}>
            {exportMsg}
          </span>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 flex gap-4 min-h-0">
        {/* Feedback List */}
        <div className="w-[480px] flex flex-col bg-white rounded-xl border border-gray-200 overflow-hidden">
          <div className="px-4 py-3 border-b border-gray-100 bg-gray-50">
            <span className="text-sm font-medium text-gray-600">反馈列表</span>
          </div>
          <div className="flex-1 overflow-auto">
            {loading ? (
              <div className="flex items-center justify-center py-10">
                <Loader2 size={24} className="animate-spin text-blue-500" />
              </div>
            ) : feedbacks.length === 0 ? (
              <div className="p-6 text-center text-gray-400 text-sm">
                暂无反馈数据
              </div>
            ) : (
              feedbacks.map((fb) => (
                <div
                  key={fb.id}
                  onClick={() => setSelectedFeedback(fb)}
                  className={`px-4 py-3 border-b border-gray-50 cursor-pointer hover:bg-blue-50 transition-colors ${
                    selectedFeedback?.id === fb.id ? 'bg-blue-50 border-l-2 border-l-blue-500' : ''
                  }`}
                >
                  <div className="flex items-start gap-3">
                    <div className="mt-0.5 shrink-0">
                      {fb.feedback === 'like' ? (
                        <ThumbsUp size={18} className="text-green-500" />
                      ) : (
                        <ThumbsDown size={18} className="text-red-500" />
                      )}
                    </div>
                    <div className="min-w-0 flex-1">
                      <p className="text-sm text-gray-800 line-clamp-2">{truncate(fb.query, 80)}</p>
                      <div className="flex items-center gap-2 mt-1">
                        <span className="text-xs text-gray-400">{fb.app_name}</span>
                        <span className="text-xs text-gray-300">·</span>
                        <span className="text-xs text-gray-400">{formatTime(fb.created_at)}</span>
                      </div>
                    </div>
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

        {/* Feedback Detail */}
        <div className="flex-1 bg-white rounded-xl border border-gray-200 overflow-hidden flex flex-col">
          <div className="px-4 py-3 border-b border-gray-100 bg-gray-50">
            <span className="text-sm font-medium text-gray-600">反馈详情</span>
          </div>
          <div className="flex-1 overflow-auto p-4">
            {!selectedFeedback ? (
              <div className="flex items-center justify-center h-full text-gray-400 text-sm">
                请选择一条反馈查看详情
              </div>
            ) : (
              <div className="space-y-4">
                {/* Header */}
                <div className="flex items-center gap-3">
                  {selectedFeedback.feedback === 'like' ? (
                    <div className="flex items-center gap-1.5 px-3 py-1 rounded-full bg-green-50 text-green-700">
                      <ThumbsUp size={16} />
                      <span className="text-sm font-medium">赞</span>
                    </div>
                  ) : (
                    <div className="flex items-center gap-1.5 px-3 py-1 rounded-full bg-red-50 text-red-700">
                      <ThumbsDown size={16} />
                      <span className="text-sm font-medium">踩</span>
                    </div>
                  )}
                  <span className="text-sm text-gray-500">{selectedFeedback.app_name}</span>
                  <span className="text-xs text-gray-400 ml-auto">
                    <Clock size={12} className="inline mr-1" />
                    {formatTime(selectedFeedback.created_at)}
                  </span>
                </div>

                {/* User Query */}
                <div className="bg-blue-50 rounded-lg px-4 py-3">
                  <div className="flex items-center gap-2 mb-2">
                    <User size={14} className="text-blue-600" />
                    <span className="text-xs font-medium text-blue-600">用户提问</span>
                  </div>
                  <p className="text-sm text-gray-800 whitespace-pre-wrap">{selectedFeedback.query}</p>
                </div>

                {/* AI Answer */}
                <div className="rounded-lg border border-gray-100 px-4 py-3">
                  <div className="flex items-center gap-2 mb-2">
                    <MessageSquare size={14} className="text-gray-500" />
                    <span className="text-xs font-medium text-gray-500">AI 回答</span>
                    <div className="ml-auto flex items-center gap-2 text-xs text-gray-400">
                      <span>Tokens: {selectedFeedback.prompt_tokens}+{selectedFeedback.answer_tokens}</span>
                      {selectedFeedback.elapsed_time > 0 && (
                        <span>{selectedFeedback.elapsed_time.toFixed(2)}s</span>
                      )}
                    </div>
                  </div>
                  <p className="text-sm text-gray-800 whitespace-pre-wrap">{selectedFeedback.answer}</p>
                </div>

                {/* Feedback Details */}
                {Array.isArray(selectedFeedback.feedbacks) && selectedFeedback.feedbacks.length > 0 && (
                  <div className="rounded-lg bg-amber-50 p-4">
                    <p className="mb-3 text-sm font-medium text-amber-700">
                      反馈详情 ({selectedFeedback.feedbacks.length})
                    </p>
                    <div className="space-y-2">
                      {selectedFeedback.feedbacks.map((fb: any, i: number) => (
                        <div key={i} className="rounded border border-amber-100 bg-white p-3">
                          <div className="mb-2 flex flex-wrap items-center gap-2 text-xs text-amber-700">
                            <span>#{i + 1}</span>
                            {fb.rating && <span>评分: {fb.rating}</span>}
                            {fb.from_source && <span>来源: {fb.from_source}</span>}
                            {fb.created_at && <span>时间: {formatTime(fb.created_at)}</span>}
                          </div>
                          {fb.content && (
                            <p className="text-sm text-gray-700 whitespace-pre-wrap">{fb.content}</p>
                          )}
                          <details className="mt-2">
                            <summary className="cursor-pointer text-xs text-gray-400">查看原始数据</summary>
                            <pre className="mt-1 max-h-40 overflow-auto whitespace-pre-wrap break-all text-xs text-gray-500">
                              {JSON.stringify(fb, null, 2)}
                            </pre>
                          </details>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Metadata */}
                <div className="text-xs text-gray-400 flex items-center gap-4 pt-2 border-t border-gray-100">
                  <span>Message ID: {selectedFeedback.message_id}</span>
                  <span>Conversation ID: {selectedFeedback.conversation_id}</span>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}