import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AppWindow, RefreshCw, Loader2, CheckCircle, Trash2 } from 'lucide-react';
import type { DifyApp } from '../types';

export function AppsPage() {
  const [apps, setApps] = useState<DifyApp[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    loadApps();
  }, []);

  const loadApps = async () => {
    try {
      const result = await invoke<DifyApp[]>('get_local_apps');
      setApps(result || []);
    } catch (e: any) {
      setError(e.toString());
    }
  };

  const handleRefreshFromDify = async () => {
    setLoading(true);
    setError('');
    try {
      const result = await invoke<DifyApp[]>('fetch_apps_from_dify');
      setApps(result || []);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteApp = async (appId: string) => {
    if (!confirm('确认删除该应用的本地数据？此操作不可恢复。')) return;
    try {
      await invoke('delete_app_data', { appId });
      loadApps();
    } catch (e: any) {
      setError(e.toString());
    }
  };

  const getModeBadge = (mode: string) => {
    const colors: Record<string, string> = {
      chat: 'bg-blue-100 text-blue-700',
      completion: 'bg-purple-100 text-purple-700',
      workflow: 'bg-green-100 text-green-700',
      'agent-chat': 'bg-orange-100 text-orange-700',
    };
    return (
      <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${colors[mode] || 'bg-gray-100 text-gray-700'}`}>
        {mode}
      </span>
    );
  };

  return (
    <div>
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
            <AppWindow size={24} />
            应用管理
          </h2>
          <p className="text-gray-500 mt-1">从 Dify 同步应用列表，管理本地数据</p>
        </div>
        <button
          onClick={handleRefreshFromDify}
          disabled={loading}
          className="px-4 py-2.5 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 flex items-center gap-2"
        >
          {loading ? <Loader2 size={16} className="animate-spin" /> : <RefreshCw size={16} />}
          从 Dify 同步
        </button>
      </div>

      {error && (
        <div className="mb-4 px-4 py-3 bg-red-50 text-red-700 rounded-lg text-sm border border-red-200">
          {error}
        </div>
      )}

      {loading && apps.length === 0 ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 size={32} className="animate-spin text-blue-500" />
          <span className="ml-3 text-gray-500">正在从 Dify 获取应用列表...</span>
        </div>
      ) : apps.length === 0 ? (
        <div className="bg-white rounded-xl border border-gray-200 p-12 text-center">
          <AppWindow size={48} className="mx-auto text-gray-300 mb-3" />
          <p className="text-gray-500">暂无应用数据</p>
          <p className="text-sm text-gray-400 mt-1">点击"从 Dify 同步"按钮获取应用列表</p>
        </div>
      ) : (
        <div className="grid gap-4">
          {apps.map((app) => (
            <div
              key={app.id}
              className="bg-white rounded-xl border border-gray-200 p-5 hover:shadow-sm transition-shadow"
            >
              <div className="flex items-start justify-between">
                <div className="flex items-start gap-4">
                  <div
                    className="w-10 h-10 rounded-lg flex items-center justify-center text-lg"
                    style={{ backgroundColor: app.icon_background || '#f3f4f6' }}
                  >
                    {app.icon || '📱'}
                  </div>
                  <div>
                    <div className="flex items-center gap-2">
                      <h3 className="font-semibold text-gray-900">{app.name}</h3>
                      {getModeBadge(app.mode)}
                    </div>
                    <p className="text-sm text-gray-500 mt-1">
                      {app.description || '无描述'}
                    </p>
                    <p className="text-xs text-gray-400 mt-2">
                      ID: {app.id} · 创建于{' '}
                      {new Date(app.created_at * 1000).toLocaleString('zh-CN')}
                    </p>
                  </div>
                </div>
                <button
                  onClick={() => handleDeleteApp(app.id)}
                  className="p-2 text-gray-400 hover:text-red-500 transition-colors"
                  title="删除本地数据"
                >
                  <Trash2 size={16} />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}