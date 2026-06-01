import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RefreshCw, Loader2, CheckCircle, XCircle, Play, Clock, Zap, Database } from 'lucide-react';
import type { DifyApp, SyncResult } from '../types';

interface SyncStatus {
  app_id: string;
  app_name: string;
  status: 'idle' | 'syncing' | 'completed' | 'error';
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
  error_message?: string;
  last_synced_at?: number;
}

export function SyncPage() {
  const [apps, setApps] = useState<DifyApp[]>([]);
  const [selectedApps, setSelectedApps] = useState<Set<string>>(new Set());
  const [syncStatuses, setSyncStatuses] = useState<Map<string, SyncStatus>>(new Map());
  const [syncing, setSyncing] = useState(false);
  const [loading, setLoading] = useState(true);
  const [syncMode, setSyncMode] = useState<'incremental' | 'full'>('incremental');

  useEffect(() => {
    loadApps();
  }, []);

  const loadApps = async () => {
    try {
      const result = await invoke<DifyApp[]>('get_local_apps');
      setApps(result || []);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const toggleApp = (appId: string) => {
    setSelectedApps((prev) => {
      const next = new Set(prev);
      if (next.has(appId)) {
        next.delete(appId);
      } else {
        next.add(appId);
      }
      return next;
    });
  };

  const toggleAll = () => {
    if (selectedApps.size === apps.length) {
      setSelectedApps(new Set());
    } else {
      setSelectedApps(new Set(apps.map((a) => a.id)));
    }
  };

  const handleSync = async () => {
    if (selectedApps.size === 0) return;
    setSyncing(true);

    for (const appId of selectedApps) {
      const app = apps.find((a) => a.id === appId);
      setSyncStatuses((prev) => {
        const next = new Map(prev);
        next.set(appId, {
          app_id: appId,
          app_name: app?.name || appId,
          status: 'syncing',
          total_conversations: 0,
          synced_conversations: 0,
          total_messages: 0,
          synced_messages: 0,
          synced_workflow_runs: 0,
          synced_node_executions: 0,
          failed_details: 0,
        });
        return next;
      });

      try {
        const result = await invoke<SyncResult>('sync_app_data', {
          appId,
          incremental: syncMode === 'incremental',
        });
        setSyncStatuses((prev) => {
          const next = new Map(prev);
          next.set(appId, {
            ...(prev.get(appId) || {}),
            status: 'completed',
            total_conversations: result.total_conversations,
            synced_conversations: result.synced_conversations,
            total_messages: result.total_messages,
            synced_messages: result.synced_messages,
            synced_workflow_runs: result.synced_workflow_runs,
            synced_node_executions: result.synced_node_executions,
            failed_details: result.failed_details,
            new_conversations: result.new_conversations,
            updated_conversations: result.updated_conversations,
            skipped_conversations: result.skipped_conversations,
            last_synced_at: Date.now(),
          } as SyncStatus);
          return next;
        });
      } catch (e: any) {
        setSyncStatuses((prev) => {
          const next = new Map(prev);
          next.set(appId, {
            ...(prev.get(appId) || {}),
            status: 'error',
            error_message: e.toString(),
          } as SyncStatus);
          return next;
        });
      }
    }
    setSyncing(false);
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'syncing':
        return <Loader2 size={16} className="animate-spin text-blue-500" />;
      case 'completed':
        return <CheckCircle size={16} className="text-green-500" />;
      case 'error':
        return <XCircle size={16} className="text-red-500" />;
      default:
        return <Clock size={16} className="text-gray-300" />;
    }
  };

  return (
    <div>
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
            <RefreshCw size={24} />
            数据同步
          </h2>
          <p className="text-gray-500 mt-1">选择应用并同步对话记录到本地</p>
        </div>
        <div className="flex items-center gap-3">
          {/* Sync Mode Toggle */}
          <div className="flex items-center bg-gray-100 rounded-lg p-0.5">
            <button
              onClick={() => setSyncMode('incremental')}
              disabled={syncing}
              className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all flex items-center gap-1.5 ${
                syncMode === 'incremental'
                  ? 'bg-white text-blue-600 shadow-sm'
                  : 'text-gray-500 hover:text-gray-700'
              }`}
            >
              <Zap size={12} />
              增量同步
            </button>
            <button
              onClick={() => setSyncMode('full')}
              disabled={syncing}
              className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all flex items-center gap-1.5 ${
                syncMode === 'full'
                  ? 'bg-white text-blue-600 shadow-sm'
                  : 'text-gray-500 hover:text-gray-700'
              }`}
            >
              <Database size={12} />
              全量同步
            </button>
          </div>
          <button
            onClick={handleSync}
            disabled={syncing || selectedApps.size === 0}
            className="px-5 py-2.5 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          >
            {syncing ? (
              <>
                <Loader2 size={16} className="animate-spin" />
                同步中...
              </>
            ) : (
              <>
                <Play size={16} />
                {syncMode === 'incremental' ? '增量同步' : '全量同步'} ({selectedApps.size} 个应用)
              </>
            )}
          </button>
        </div>
      </div>

      {/* Sync Mode Description */}
      <div className="mb-4 p-3 bg-blue-50 border border-blue-100 rounded-lg">
        <p className="text-xs text-blue-700">
          {syncMode === 'incremental'
            ? '💡 增量同步模式：仅获取上次同步后有变化的对话（新增/更新），跳过未变化的对话，速度更快。'
            : '🔄 全量同步模式：重新获取所有对话和消息，确保数据完整，但耗时较长。'}
        </p>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 size={32} className="animate-spin text-blue-500" />
        </div>
      ) : apps.length === 0 ? (
        <div className="bg-white rounded-xl border border-gray-200 p-12 text-center">
          <p className="text-gray-500">请先在"应用管理"中同步应用列表</p>
        </div>
      ) : (
        <>
          {/* Select All */}
          <div className="mb-4">
            <label className="flex items-center gap-2 text-sm text-gray-600 cursor-pointer">
              <input
                type="checkbox"
                checked={selectedApps.size === apps.length && apps.length > 0}
                onChange={toggleAll}
                className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
              />
              全选 ({apps.length} 个应用)
            </label>
          </div>

          {/* App List */}
          <div className="space-y-3">
            {apps.map((app) => {
              const status = syncStatuses.get(app.id);
              return (
                <div
                  key={app.id}
                  className="bg-white rounded-xl border border-gray-200 p-4"
                >
                  <div className="flex items-center gap-4">
                    <input
                      type="checkbox"
                      checked={selectedApps.has(app.id)}
                      onChange={() => toggleApp(app.id)}
                      disabled={syncing}
                      className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                    />
                    <div
                      className="w-8 h-8 rounded-lg flex items-center justify-center text-sm"
                      style={{ backgroundColor: app.icon_background || '#f3f4f6' }}
                    >
                      {app.icon || '📱'}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <h3 className="font-medium text-gray-900 text-sm">{app.name}</h3>
                        {getStatusIcon(status?.status || 'idle')}
                      </div>
                      {status && status.status !== 'idle' && (
                        <div className="mt-2">
                          <div className="flex items-center gap-4 text-xs text-gray-500">
                            <span>
                              对话: {status.synced_conversations}/{status.total_conversations}
                            </span>
                            <span>
                              消息: {status.synced_messages}/{status.total_messages}
                            </span>
                            <span>
                              Workflow: {status.synced_workflow_runs}
                            </span>
                            <span>
                              节点: {status.synced_node_executions}
                            </span>
                            {status.failed_details > 0 && (
                              <span className="text-amber-600">
                                详情失败: {status.failed_details}
                              </span>
                            )}
                            {status.status === 'syncing' && (
                              <span className="text-blue-500">同步中，请稍候...</span>
                            )}
                          </div>
                          {/* Incremental sync stats (only meaningful when skipped > 0) */}
                          {status.skipped_conversations !== undefined && status.skipped_conversations > 0 && (
                            <div className="flex items-center gap-3 text-xs mt-1.5">
                              {status.new_conversations !== undefined && status.new_conversations > 0 && (
                                <span className="text-green-600">
                                  ✨ 新增: {status.new_conversations}
                                </span>
                              )}
                              {status.updated_conversations !== undefined && status.updated_conversations > 0 && (
                                <span className="text-blue-600">
                                  🔄 更新: {status.updated_conversations}
                                </span>
                              )}
                              <span className="text-gray-400">
                                ⏭️ 跳过: {status.skipped_conversations}
                              </span>
                            </div>
                          )}
                          {status.error_message && (
                            <p className="text-xs text-red-500 mt-1">
                              {status.error_message}
                            </p>
                          )}
                          {status.last_synced_at && (
                            <p className="text-xs text-gray-400 mt-1">
                              上次同步: {new Date(status.last_synced_at).toLocaleString('zh-CN')}
                            </p>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </>
      )}
    </div>
  );
}
