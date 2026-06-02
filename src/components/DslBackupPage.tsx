import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { DslBackupSettings, DslBackupResult } from '../types';

export default function DslBackupPage() {
  const [settings, setSettings] = useState<DslBackupSettings>({
    enabled: false,
    interval_minutes: 1440,
    backup_dir: '',
    include_secret: false,
    last_backup_at: null,
  });
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [backing, setBacking] = useState(false);
  const [results, setResults] = useState<DslBackupResult[] | null>(null);
  const [error, setError] = useState('');
  const [saveMsg, setSaveMsg] = useState('');

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    setLoading(true);
    try {
      const s = await invoke<DslBackupSettings>('get_dsl_backup_settings');
      setSettings(s);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setLoading(false);
    }
  };

  const saveSettings = async () => {
    setSaving(true);
    setSaveMsg('');
    setError('');
    try {
      await invoke('save_dsl_backup_settings', { settings });
      setSaveMsg('设置已保存');
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setSaving(false);
    }
  };

  const selectBackupDir = async () => {
    try {
      const selected = await open({
        directory: true,
        title: '选择备份目录',
      });
      if (selected) {
        setSettings(prev => ({ ...prev, backup_dir: selected }));
      }
    } catch (_e) {
      // User cancelled
    }
  };

  const startBackup = async () => {
    setBacking(true);
    setResults(null);
    setError('');
    try {
      // Auto-save settings first so backend reads the latest backup_dir from DB
      await invoke('save_dsl_backup_settings', { settings });

      const r = await invoke<DslBackupResult[]>('backup_all_dsl', {
        includeSecret: settings.include_secret,
      });
      setResults(r);
      // Refresh settings to get updated last_backup_at
      const s = await invoke<DslBackupSettings>('get_dsl_backup_settings');
      setSettings(s);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setBacking(false);
    }
  };

  const formatTime = (ts: number | null) => {
    if (!ts) return '从未备份';
    return new Date(ts * 1000).toLocaleString('zh-CN');
  };

  const successCount = results ? results.filter(r => r.success).length : 0;
  const failCount = results ? results.filter(r => !r.success).length : 0;

  if (loading) {
    return <div className="text-gray-500 text-sm">加载中...</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-800">DSL 备份</h2>
        <div className="text-sm text-gray-500">
          上次备份: {formatTime(settings.last_backup_at)}
        </div>
      </div>

      {/* Settings */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-4">
        <h3 className="text-base font-medium text-gray-700">备份设置</h3>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Backup Directory */}
          <div className="md:col-span-2">
            <label className="block text-sm font-medium text-gray-600 mb-1">备份目录</label>
            <div className="flex gap-2">
              <input
                type="text"
                value={settings.backup_dir}
                onChange={(e) => setSettings({ ...settings, backup_dir: e.target.value })}
                placeholder="选择或输入备份目录路径"
                className="flex-1 px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              <button
                onClick={selectBackupDir}
                className="px-4 py-2 bg-gray-100 text-gray-700 rounded-md text-sm hover:bg-gray-200 transition-colors"
              >
                浏览
              </button>
            </div>
          </div>

          {/* Enable Auto Backup */}
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="auto_backup_enabled"
              checked={settings.enabled}
              onChange={(e) => setSettings({ ...settings, enabled: e.target.checked })}
              className="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
            />
            <label htmlFor="auto_backup_enabled" className="text-sm text-gray-600">
              启用定时自动备份
            </label>
          </div>

          {/* Interval */}
          <div className="flex items-center gap-2">
            <label htmlFor="backup_interval" className="text-sm text-gray-600 whitespace-nowrap">
              备份间隔：
            </label>
            <input
              type="number"
              id="backup_interval"
              value={settings.interval_minutes}
              onChange={(e) => setSettings({ ...settings, interval_minutes: parseInt(e.target.value) || 60 })}
              min={1}
              className="w-24 px-3 py-1.5 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <span className="text-sm text-gray-500">分钟</span>
          </div>

          {/* Include Secret */}
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="include_secret"
              checked={settings.include_secret}
              onChange={(e) => setSettings({ ...settings, include_secret: e.target.checked })}
              className="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
            />
            <label htmlFor="include_secret" className="text-sm text-gray-600">
              包含密钥信息（include_secret）
            </label>
          </div>
        </div>

        {/* Save Button */}
        <div className="flex items-center gap-3">
          <button
            onClick={saveSettings}
            disabled={saving}
            className="px-4 py-2 bg-blue-600 text-white rounded-md text-sm hover:bg-blue-700 disabled:opacity-50 transition-colors"
          >
            {saving ? '保存中...' : '保存设置'}
          </button>
          {saveMsg && <span className="text-sm text-green-600">{saveMsg}</span>}
        </div>
      </div>

      {/* Manual Backup */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-4">
        <h3 className="text-base font-medium text-gray-700">手动备份</h3>
        <p className="text-sm text-gray-500">
          导出所有已同步应用的 DSL（YAML 格式）到备份目录。每次备份会创建一个带时间戳的子目录。
        </p>
        <button
          onClick={startBackup}
          disabled={backing || !settings.backup_dir}
          className="px-4 py-2 bg-green-600 text-white rounded-md text-sm hover:bg-green-700 disabled:opacity-50 transition-colors"
        >
          {backing ? '备份中...' : '立即备份所有应用 DSL'}
        </button>
        {!settings.backup_dir && (
          <p className="text-sm text-amber-600">请先设置备份目录</p>
        )}
      </div>

      {/* Error */}
      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-sm text-red-700">{error}</p>
        </div>
      )}

      {/* Results */}
      {results && (
        <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-4">
          <h3 className="text-base font-medium text-gray-700">
            备份结果
            <span className="ml-3 text-sm font-normal text-gray-500">
              成功 {successCount} / 失败 {failCount} / 共 {results.length} 个应用
            </span>
          </h3>

          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200">
                  <th className="text-left py-2 px-3 font-medium text-gray-600">应用名称</th>
                  <th className="text-left py-2 px-3 font-medium text-gray-600">状态</th>
                  <th className="text-left py-2 px-3 font-medium text-gray-600">文件路径</th>
                  <th className="text-left py-2 px-3 font-medium text-gray-600">错误信息</th>
                </tr>
              </thead>
              <tbody>
                {results.map((r) => (
                  <tr key={r.app_id} className="border-b border-gray-100">
                    <td className="py-2 px-3">{r.app_name}</td>
                    <td className="py-2 px-3">
                      {r.success ? (
                        <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-green-100 text-green-800">
                          ✓ 成功
                        </span>
                      ) : (
                        <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-red-100 text-red-800">
                          ✗ 失败
                        </span>
                      )}
                    </td>
                    <td className="py-2 px-3 text-xs text-gray-500 break-all">{r.file_path || '-'}</td>
                    <td className="py-2 px-3 text-xs text-red-600">{r.error || '-'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}