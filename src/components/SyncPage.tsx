import { useState, useEffect } from 'react';
import { RefreshCw, Loader2, CheckCircle, XCircle, Play, Clock } from 'lucide-react';
import type { DifyApp } from '../types';

interface SyncStatus {
  app_id: string;
  app_name: string;
  status: 'idle' | 'syncing' | 'completed' | 'error';
  total_conversations: number;
  synced_conversations: number;
  total_messages: number;
  synced_messages: number;
  error_message?: string;
  last_synced_at?: number;
}

export function SyncPage() {
  const [apps, setApps] = useState<DifyApp[]>([]);
  const [selectedApps, setSelectedApps] = useState<Set<string>>(new Set());
  const [syncStatuses, setSyncStatuses] = useState<Map<string, SyncStatus>>(new Map());
  const [syncing, setSyncing] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadApps();
  }, []);

  const loadApps = async () => {
    try {
      const result = await (window as any).__TAURI__.invoke('get_local_apps');
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
        });
        return next;
      });

      try {
        const result = await (window as any).__TAURI__.invoke('sync_app_data', { appId });
        setSyncStatuses((prev) => {
          const next = new Map(prev);
          next.set(appId, {
            ...(prev.get(appId) || {}),
            status: 'completed',
            total_conversations: result.total_conversations,
            synced_conversations: result.synced_conversations,
            total_messages: result.total_messages,
            synced_messages: result.synced_messages,
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

  const getProgressBar = (current: number, total: number) => {
    if (total === 0) return 0;
    return Math.round((current / total) * 100);
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
              开始同步 ({selectedApps.size} 个应用)
            </>
          )}
        </button>
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
                            {status.status === 'syncing' && (
                              <div className="flex-1 bg-gray-200 rounded-full h-1.5 max-w-xs">
                                <div
                                  className="bg-blue-500 rounded-full h-1.5 transition-all"
                                  style={{
                                    width: `${getProgressBar(status.synced_conversations, status.total_conversations)}%`,
                                  }}
                                />
                              </div>
                            )}
                          </div>
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