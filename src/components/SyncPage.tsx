import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RefreshCw, Loader2, CheckCircle, XCircle, Play, Clock, Zap, Database, Timer, ToggleLeft, ToggleRight, Trash2, AlertTriangle, Settings2 } from 'lucide-react';
import type { DifyApp, SyncResult, AutoSyncSettings, SyncConfig, AppSyncSetting, AppSyncDataInfo } from '../types';

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

const INTERVAL_OPTIONS = [
  { value: 5, label: '5 分钟' },
  { value: 15, label: '15 分钟' },
  { value: 30, label: '30 分钟' },
  { value: 60, label: '1 小时' },
  { value: 360, label: '6 小时' },
  { value: 720, label: '12 小时' },
  { value: 1440, label: '24 小时' },
];

export function SyncPage() {
  const [apps, setApps] = useState<DifyApp[]>([]);
  const [selectedApps, setSelectedApps] = useState<Set<string>>(new Set());
  const [syncStatuses, setSyncStatuses] = useState<Map<string, SyncStatus>>(new Map());
  const [syncing, setSyncing] = useState(false);
  const [loading, setLoading] = useState(true);
  const [syncMode, setSyncMode] = useState<'incremental' | 'full'>('incremental');

  // Per-app sync config
  const [syncConfig, setSyncConfig] = useState<SyncConfig>({ apps: [] });
  const [appDataInfo, setAppDataInfo] = useState<Map<string, AppSyncDataInfo>>(new Map());
  const [expandedApp, setExpandedApp] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{ appId: string; type: 'data' | 'workflow' } | null>(null);
  const [actionError, setActionError] = useState<string>('');

  // Auto sync state
  const [autoSyncSettings, setAutoSyncSettings] = useState<AutoSyncSettings>({
    enabled: false,
    interval_minutes: 30,
    mode: 'incremental',
    last_synced_at: null,
  });
  const [autoSyncSaving, setAutoSyncSaving] = useState(false);
  const [autoSyncError, setAutoSyncError] = useState<string>('');

  useEffect(() => {
    loadApps();
    loadAutoSyncSettings();
    loadSyncConfig();

    const interval = setInterval(loadAutoSyncSettings, 30000);
    return () => clearInterval(interval);
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

  const loadSyncConfig = async () => {
    try {
      const config = await invoke<SyncConfig>('get_sync_config');
      if (config) {
        setSyncConfig(config);
      }
    } catch (e) {
      console.error(e);
    }
  };

  const loadAppDataInfo = async (appId: string) => {
    try {
      const info = await invoke<AppSyncDataInfo>('get_app_sync_data_info', { appId });
      setAppDataInfo((prev) => {
        const next = new Map(prev);
        next.set(appId, info);
        return next;
      });
    } catch (e) {
      console.error(e);
    }
  };

  const loadAutoSyncSettings = async () => {
    try {
      const settings = await invoke<AutoSyncSettings>('get_auto_sync_settings');
      if (settings) {
        setAutoSyncSettings(settings);
      }
    } catch (e) {
      console.error(e);
    }
  };

  // Get sync setting for a specific app
  const getAppSyncSetting = (appId: string): AppSyncSetting => {
    const found = syncConfig.apps.find((a) => a.app_id === appId);
    return found || { app_id: appId, enabled: false, sync_workflow_details: false };
  };

  // Update sync config for a specific app
  const updateAppSyncSetting = async (appId: string, updates: Partial<AppSyncSetting>) => {
    const newConfig = { ...syncConfig, apps: [...syncConfig.apps] };
    const idx = newConfig.apps.findIndex((a) => a.app_id === appId);
    if (idx >= 0) {
      newConfig.apps[idx] = { ...newConfig.apps[idx], ...updates };
    } else {
      newConfig.apps.push({ app_id: appId, enabled: false, sync_workflow_details: false, ...updates });
    }
    setSyncConfig(newConfig);
    setActionError('');
    try {
      await invoke('save_sync_config', { config: newConfig });
    } catch (e: any) {
      console.error('保存同步配置失败:', e);
      setActionError(`保存配置失败: ${e}`);
      // Revert local state on failure
      loadSyncConfig();
    }
  };

  const handleAutoSyncToggle = async () => {
    const newSettings = {
      ...autoSyncSettings,
      enabled: !autoSyncSettings.enabled,
    };
    setAutoSyncSaving(true);
    setAutoSyncError('');
    try {
      await invoke('save_auto_sync_settings', { settings: newSettings });
      setAutoSyncSettings(newSettings);
    } catch (e: any) {
      setAutoSyncError(`保存失败: ${e}`);
    } finally {
      setAutoSyncSaving(false);
    }
  };

  const handleAutoSyncIntervalChange = async (interval: number) => {
    const newSettings = { ...autoSyncSettings, interval_minutes: interval };
    setAutoSyncSaving(true);
    setAutoSyncError('');
    try {
      await invoke('save_auto_sync_settings', { settings: newSettings });
      setAutoSyncSettings(newSettings);
    } catch (e: any) {
      setAutoSyncError(`保存失败: ${e}`);
    } finally {
      setAutoSyncSaving(false);
    }
  };

  const handleAutoSyncModeChange = async (mode: string) => {
    const newSettings = { ...autoSyncSettings, mode };
    setAutoSyncSaving(true);
    setAutoSyncError('');
    try {
      await invoke('save_auto_sync_settings', { settings: newSettings });
      setAutoSyncSettings(newSettings);
    } catch (e: any) {
      setAutoSyncError(`保存失败: ${e}`);
    } finally {
      setAutoSyncSaving(false);
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
    // Only select enabled apps
    const enabledApps = apps.filter((a) => getAppSyncSetting(a.id).enabled);
    if (selectedApps.size === enabledApps.length) {
      setSelectedApps(new Set());
    } else {
      setSelectedApps(new Set(enabledApps.map((a) => a.id)));
    }
  };

  const handleSync = async () => {
    if (selectedApps.size === 0) return;
    setSyncing(true);

    for (const appId of selectedApps) {
      const app = apps.find((a) => a.id === appId);
      const setting = getAppSyncSetting(appId);
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
          syncWorkflowDetails: setting.sync_workflow_details,
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
        // Refresh data info after sync
        await loadAppDataInfo(appId);
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

    try {
      await invoke<string>('rebuild_dashboard_stats');
    } catch (e) {
      console.error('Auto agg failed:', e);
    }

    setSyncing(false);
  };

  const handleDeleteConfirm = async () => {
    if (!deleteConfirm) return;
    setActionError('');
    try {
      if (deleteConfirm.type === 'data') {
        await invoke('delete_app_sync_data', { appId: deleteConfirm.appId });
      } else {
        await invoke('delete_app_workflow_details', { appId: deleteConfirm.appId });
      }
      await loadAppDataInfo(deleteConfirm.appId);
      setDeleteConfirm(null);
    } catch (e: any) {
      console.error('删除失败:', e);
      setActionError(`删除失败: ${e}`);
      setDeleteConfirm(null);
    }
  };

  const toggleExpand = async (appId: string) => {
    if (expandedApp === appId) {
      setExpandedApp(null);
    } else {
      setExpandedApp(appId);
      await loadAppDataInfo(appId);
    }
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

  const formatCount = (n: number) => n.toLocaleString('zh-CN');

  // Count enabled apps
  const enabledAppsCount = apps.filter((a) => getAppSyncSetting(a.id).enabled).length;

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

      {/* Auto Sync Configuration */}
      <div className="mb-4 bg-white rounded-xl border border-gray-200 p-5">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Timer size={20} className="text-blue-600" />
            <h3 className="font-semibold text-gray-900">自动同步</h3>
            <span className="text-xs text-gray-400">定期自动同步所有已启用的应用数据</span>
          </div>
          <button
            onClick={handleAutoSyncToggle}
            disabled={autoSyncSaving}
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium transition-all"
          >
            {autoSyncSettings.enabled ? (
              <span className="flex items-center gap-1.5 text-green-700 bg-green-50 border border-green-200 px-3 py-1.5 rounded-lg">
                <ToggleRight size={18} />
                已开启
              </span>
            ) : (
              <span className="flex items-center gap-1.5 text-gray-500 bg-gray-50 border border-gray-200 px-3 py-1.5 rounded-lg">
                <ToggleLeft size={18} />
                已关闭
              </span>
            )}
          </button>
        </div>

        <div className="flex items-center gap-6">
          {/* Interval */}
          <div className="flex items-center gap-2">
            <label className="text-sm text-gray-600">同步间隔:</label>
            <select
              value={autoSyncSettings.interval_minutes}
              onChange={(e) => handleAutoSyncIntervalChange(Number(e.target.value))}
              disabled={!autoSyncSettings.enabled || autoSyncSaving}
              className="px-3 py-1.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {INTERVAL_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </div>

          {/* Auto sync mode */}
          <div className="flex items-center gap-2">
            <label className="text-sm text-gray-600">同步模式:</label>
            <div className="flex items-center bg-gray-100 rounded-lg p-0.5">
              <button
                onClick={() => handleAutoSyncModeChange('incremental')}
                disabled={!autoSyncSettings.enabled || autoSyncSaving}
                className={`px-3 py-1 text-xs font-medium rounded-md transition-all flex items-center gap-1 ${
                  autoSyncSettings.mode === 'incremental'
                    ? 'bg-white text-blue-600 shadow-sm'
                    : 'text-gray-500 hover:text-gray-700'
                } disabled:opacity-50`}
              >
                <Zap size={12} />
                增量
              </button>
              <button
                onClick={() => handleAutoSyncModeChange('full')}
                disabled={!autoSyncSettings.enabled || autoSyncSaving}
                className={`px-3 py-1 text-xs font-medium rounded-md transition-all flex items-center gap-1 ${
                  autoSyncSettings.mode === 'full'
                    ? 'bg-white text-blue-600 shadow-sm'
                    : 'text-gray-500 hover:text-gray-700'
                } disabled:opacity-50`}
              >
                <Database size={12} />
                全量
              </button>
            </div>
          </div>

          {/* Last synced */}
          {autoSyncSettings.last_synced_at && (
            <div className="text-xs text-gray-400">
              上次自动同步: {new Date(autoSyncSettings.last_synced_at * 1000).toLocaleString('zh-CN')}
            </div>
          )}
        </div>

        {autoSyncError && (
          <div className="mt-3 p-2 bg-red-50 border border-red-100 rounded-lg flex items-center gap-2">
            <XCircle size={14} className="text-red-500 shrink-0" />
            <p className="text-xs text-red-700">{autoSyncError}</p>
          </div>
        )}

        {autoSyncSettings.enabled && (
          <div className="mt-3 p-2 bg-green-50 border border-green-100 rounded-lg">
            <p className="text-xs text-green-700">
              ⏱️ 自动同步已启用，将每隔 {INTERVAL_OPTIONS.find(o => o.value === autoSyncSettings.interval_minutes)?.label || autoSyncSettings.interval_minutes + ' 分钟'} 
              以{autoSyncSettings.mode === 'incremental' ? '增量' : '全量'}模式自动同步 {enabledAppsCount} 个已启用的应用。
            </p>
          </div>
        )}
      </div>

      {/* Sync Mode Description */}
      <div className="mb-4 p-3 bg-blue-50 border border-blue-100 rounded-lg">
        <p className="text-xs text-blue-700">
          {syncMode === 'incremental'
            ? '💡 增量同步模式：仅获取上次同步后有变化的对话（新增/更新），跳过未变化的对话，速度更快。'
            : '🔄 全量同步模式：重新获取所有对话和消息，确保数据完整，但耗时较长。'}
        </p>
      </div>

      {/* Action Error Banner */}
      {actionError && (
        <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg flex items-center justify-between">
          <div className="flex items-center gap-2">
            <XCircle size={16} className="text-red-500 shrink-0" />
            <p className="text-sm text-red-700">{actionError}</p>
          </div>
          <button onClick={() => setActionError('')} className="text-red-400 hover:text-red-600 text-sm">✕</button>
        </div>
      )}

      {/* Delete Confirmation Modal */}
      {deleteConfirm && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl p-6 max-w-md w-full mx-4 shadow-2xl">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 rounded-full bg-red-100 flex items-center justify-center">
                <AlertTriangle size={20} className="text-red-600" />
              </div>
              <div>
                <h3 className="font-semibold text-gray-900">
                  {deleteConfirm.type === 'data' ? '删除同步数据' : '删除工作流详情'}
                </h3>
                <p className="text-sm text-gray-500">
                  {apps.find(a => a.id === deleteConfirm.appId)?.name}
                </p>
              </div>
            </div>
            <p className="text-sm text-gray-600 mb-6">
              {deleteConfirm.type === 'data'
                ? '确定要删除该应用的同步数据吗？将删除所有对话、消息和工作流日志记录，但保留应用信息和已同步的工作流详情。此操作不可撤销。'
                : '确定要删除该应用的工作流详情吗？将删除所有工作流运行记录和节点执行记录。此操作不可撤销。'}
            </p>
            <div className="flex justify-end gap-3">
              <button
                onClick={() => setDeleteConfirm(null)}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors"
              >
                取消
              </button>
              <button
                onClick={handleDeleteConfirm}
                className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-lg hover:bg-red-700 transition-colors"
              >
                确认删除
              </button>
            </div>
          </div>
        </div>
      )}

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
                checked={selectedApps.size === enabledAppsCount && enabledAppsCount > 0}
                onChange={toggleAll}
                className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
              />
              全选 ({enabledAppsCount} 个已启用应用)
            </label>
          </div>

          {/* App List */}
          <div className="space-y-3">
            {apps.map((app) => {
              const status = syncStatuses.get(app.id);
              const setting = getAppSyncSetting(app.id);
              const isExpanded = expandedApp === app.id;
              const dataInfo = appDataInfo.get(app.id);

              return (
                <div
                  key={app.id}
                  className={`bg-white rounded-xl border transition-colors ${
                    !setting.enabled ? 'border-gray-100 opacity-60' : 'border-gray-200'
                  }`}
                >
                  <div className="p-4">
                    <div className="flex items-center gap-4">
                      <input
                        type="checkbox"
                        checked={selectedApps.has(app.id)}
                        onChange={() => toggleApp(app.id)}
                        disabled={syncing || !setting.enabled}
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
                          <span className={`px-1.5 py-0.5 text-[10px] font-medium rounded ${
                            app.mode === 'workflow'
                              ? 'bg-purple-100 text-purple-700'
                              : app.mode === 'completion'
                              ? 'bg-orange-100 text-orange-700'
                              : 'bg-blue-100 text-blue-700'
                          }`}>
                            {app.mode === 'workflow' ? 'Workflow' : app.mode === 'completion' ? 'Completion' : 'Chat'}
                          </span>
                          {getStatusIcon(status?.status || 'idle')}
                          {!setting.enabled && (
                            <span className="text-[10px] text-gray-400 bg-gray-100 px-1.5 py-0.5 rounded">已禁用</span>
                          )}
                        </div>
                        {status && status.status !== 'idle' && (
                          <div className="mt-2">
                            <div className="flex items-center gap-4 text-xs text-gray-500">
                              {app.mode === 'workflow' ? (
                                <span>日志: {status.synced_messages}/{status.total_messages}</span>
                              ) : (
                                <>
                                  <span>对话: {status.synced_conversations}/{status.total_conversations}</span>
                                  <span>消息: {status.synced_messages}/{status.total_messages}</span>
                                </>
                              )}
                              <span>Workflow: {status.synced_workflow_runs}</span>
                              <span>节点: {status.synced_node_executions}</span>
                              {status.failed_details > 0 && (
                                <span className="text-amber-600">详情失败: {status.failed_details}</span>
                              )}
                              {status.status === 'syncing' && (
                                <span className="text-blue-500">同步中，请稍候...</span>
                              )}
                            </div>
                            {status.skipped_conversations !== undefined && status.skipped_conversations > 0 && (
                              <div className="flex items-center gap-3 text-xs mt-1.5">
                                {status.new_conversations !== undefined && status.new_conversations > 0 && (
                                  <span className="text-green-600">✨ 新增: {status.new_conversations}</span>
                                )}
                                {status.updated_conversations !== undefined && status.updated_conversations > 0 && (
                                  <span className="text-blue-600">🔄 更新: {status.updated_conversations}</span>
                                )}
                                <span className="text-gray-400">⏭️ 跳过: {status.skipped_conversations}</span>
                              </div>
                            )}
                            {status.error_message && (
                              <p className="text-xs text-red-500 mt-1">{status.error_message}</p>
                            )}
                            {status.last_synced_at && (
                              <p className="text-xs text-gray-400 mt-1">
                                上次同步: {new Date(status.last_synced_at).toLocaleString('zh-CN')}
                              </p>
                            )}
                          </div>
                        )}
                      </div>

                      {/* Config & Actions */}
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => toggleExpand(app.id)}
                          className="p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
                          title="配置与数据管理"
                        >
                          <Settings2 size={16} />
                        </button>
                      </div>
                    </div>
                  </div>

                  {/* Expanded Config Panel */}
                  {isExpanded && (
                    <div className="border-t border-gray-100 px-4 py-3 bg-gray-50/50 rounded-b-xl">
                      <div className="grid grid-cols-2 gap-4 mb-3">
                        {/* Enable sync toggle */}
                        <div className="flex items-center justify-between bg-white rounded-lg p-3 border border-gray-200">
                          <div>
                            <p className="text-sm font-medium text-gray-900">同步数据</p>
                            <p className="text-xs text-gray-500">禁用后将跳过该应用的同步</p>
                          </div>
                          <button
                            onClick={() => updateAppSyncSetting(app.id, { enabled: !setting.enabled })}
                            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
                              setting.enabled
                                ? 'text-green-700 bg-green-50 border border-green-200'
                                : 'text-gray-500 bg-gray-50 border border-gray-200'
                            }`}
                          >
                            {setting.enabled ? <ToggleRight size={16} /> : <ToggleLeft size={16} />}
                            {setting.enabled ? '已启用' : '已禁用'}
                          </button>
                        </div>

                        {/* Workflow details toggle */}
                        <div className="flex items-center justify-between bg-white rounded-lg p-3 border border-gray-200">
                          <div>
                            <p className="text-sm font-medium text-gray-900">同步工作流详情</p>
                            <p className="text-xs text-gray-500">包含运行详情和节点执行记录</p>
                          </div>
                          <button
                            onClick={() => updateAppSyncSetting(app.id, { sync_workflow_details: !setting.sync_workflow_details })}
                            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
                              setting.sync_workflow_details
                                ? 'text-green-700 bg-green-50 border border-green-200'
                                : 'text-gray-500 bg-gray-50 border border-gray-200'
                            }`}
                          >
                            {setting.sync_workflow_details ? <ToggleRight size={16} /> : <ToggleLeft size={16} />}
                            {setting.sync_workflow_details ? '已启用' : '已禁用'}
                          </button>
                        </div>
                      </div>

                      {/* Data info & Delete actions */}
                      {dataInfo && (
                        <div className="flex items-center justify-between bg-white rounded-lg p-3 border border-gray-200">
                          <div className="flex items-center gap-4 text-xs text-gray-600">
                            <span>💬 对话: {formatCount(dataInfo.conversation_count)}</span>
                            <span>📝 消息: {formatCount(dataInfo.message_count)}</span>
                            <span>🔄 工作流: {formatCount(dataInfo.workflow_run_count)}</span>
                            <span>📦 节点: {formatCount(dataInfo.node_execution_count)}</span>
                            {dataInfo.workflow_app_log_count > 0 && (
                              <span>📋 日志: {formatCount(dataInfo.workflow_app_log_count)}</span>
                            )}
                          </div>
                          <div className="flex items-center gap-2">
                            <button
                              onClick={() => setDeleteConfirm({ appId: app.id, type: 'data' })}
                              className="flex items-center gap-1 px-2.5 py-1.5 text-xs font-medium text-amber-700 bg-amber-50 border border-amber-200 rounded-lg hover:bg-amber-100 transition-colors"
                            >
                              <Trash2 size={12} />
                              删除同步数据
                            </button>
                            <button
                              onClick={() => setDeleteConfirm({ appId: app.id, type: 'workflow' })}
                              className="flex items-center gap-1 px-2.5 py-1.5 text-xs font-medium text-red-700 bg-red-50 border border-red-200 rounded-lg hover:bg-red-100 transition-colors"
                            >
                              <Trash2 size={12} />
                              删除工作流详情
                            </button>
                          </div>
                        </div>
                      )}
                      {!dataInfo && (
                        <div className="flex justify-center">
                          <Loader2 size={16} className="animate-spin text-gray-400" />
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </>
      )}
    </div>
  );
}