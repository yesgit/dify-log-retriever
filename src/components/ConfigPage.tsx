import { useState, useEffect } from 'react';
import { Settings, CheckCircle, XCircle, Loader2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

export function ConfigPage() {
  const [apiBase, setApiBase] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [proxy, setProxy] = useState('');
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<'success' | 'error' | null>(null);
  const [testMessage, setTestMessage] = useState('');
  const [saved, setSaved] = useState(false);
  const [hasExistingKey, setHasExistingKey] = useState(false);

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      const config = await invoke<{ api_base: string; api_key_masked: string; proxy?: string; has_key: boolean }>('get_config');
      if (config) {
        setApiBase(config.api_base || '');
        setApiKey(config.api_key_masked || '');
        setProxy(config.proxy || '');
        setHasExistingKey(config.has_key);
      }
    } catch (e) {
      // Config not found yet, that's OK
    }
  };

  const validateInputs = (): string | null => {
    if (!apiBase.trim()) {
      return '请输入 Dify 实例地址';
    }
    const base = apiBase.trim();
    if (!base.startsWith('http://') && !base.startsWith('https://')) {
      return '实例地址必须以 http:// 或 https:// 开头';
    }
    try {
      new URL(base);
    } catch {
      return '实例地址格式无效';
    }
    if (!apiKey.trim()) {
      return '请输入 Console API Token';
    }
    // If user hasn't changed the masked key and we have an existing key, it's ok for saving
    if (hasExistingKey && apiKey.includes('****')) {
      return null;
    }
    if (apiKey.trim().length < 8) {
      return 'API Token 长度不足，请输入有效的 Console API Token';
    }
    return null;
  };

  const handleSave = async () => {
    const validationError = validateInputs();
    if (validationError) {
      setTestResult('error');
      setTestMessage(validationError);
      return;
    }

    try {
      // If the key is masked (unchanged), we need to keep the existing one
      // The backend save_config will handle the masked key case
      const keyToSend = hasExistingKey && apiKey.includes('****') ? '__KEEP_EXISTING__' : apiKey.trim();

      await invoke('save_config', {
        apiBase: apiBase.trim().replace(/\/+$/, ''),
        apiKey: keyToSend,
        proxy: proxy.trim() || null,
      });
      setSaved(true);
      setTestResult(null);
      setTimeout(() => setSaved(false), 2000);
    } catch (e: any) {
      setTestResult('error');
      setTestMessage(e.toString());
    }
  };

  const handleTest = async () => {
    const validationError = validateInputs();
    if (validationError) {
      setTestResult('error');
      setTestMessage(validationError);
      return;
    }

    // If key is masked, user must re-enter to test connection
    if (hasExistingKey && apiKey.includes('****')) {
      setTestResult('error');
      setTestMessage('请重新输入 API Token 后再测试连接（当前显示为掩码）');
      return;
    }

    setTesting(true);
    setTestResult(null);
    try {
      const result = await invoke<number>('test_connection', {
        apiBase: apiBase.trim().replace(/\/+$/, ''),
        apiKey: apiKey.trim(),
        proxy: proxy.trim() || null,
      });
      setTestResult('success');
      setTestMessage(`连接成功！发现 ${result} 个应用`);
    } catch (e: any) {
      setTestResult('error');
      setTestMessage(`连接失败: ${e}`);
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="max-w-2xl">
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
          <Settings size={24} />
          连接配置
        </h2>
        <p className="text-gray-500 mt-1">配置 Dify 实例的连接信息</p>
      </div>

      <div className="bg-white rounded-xl border border-gray-200 p-6 space-y-5">
        {/* API Base URL */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1.5">
            Dify 实例地址
          </label>
          <input
            type="text"
            value={apiBase}
            onChange={(e) => setApiBase(e.target.value)}
            placeholder="https://dify.example.com"
            className="w-full px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
          <p className="text-xs text-gray-400 mt-1">
            自托管版输入完整地址，如 https://dify.example.com
          </p>
        </div>

        {/* API Key */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1.5">
            Console API Token
          </label>
          <input
            type="password"
            value={apiKey}
            onChange={(e) => { setApiKey(e.target.value); setHasExistingKey(false); }}
            placeholder="输入 Console API Token"
            className="w-full px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
          <p className="text-xs text-gray-400 mt-1">
            {hasExistingKey
              ? '已保存 Token（显示为掩码）。如需修改请输入新的 Token'
              : '在 Dify 后台 → 设置 → API 扩展 中获取 Console API Token'}
          </p>
        </div>

        {/* Proxy */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1.5">
            网络代理（可选）
          </label>
          <input
            type="text"
            value={proxy}
            onChange={(e) => setProxy(e.target.value)}
            placeholder="http://127.0.0.1:7890 或 socks5://127.0.0.1:1080"
            className="w-full px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
          <p className="text-xs text-gray-400 mt-1">
            如需通过代理访问 Dify，请填写代理地址。支持 HTTP 和 SOCKS5 代理，留空则直连
          </p>
        </div>

        {/* Test Result */}
        {testResult && (
          <div
            className={`flex items-center gap-2 px-4 py-3 rounded-lg text-sm ${
              testResult === 'success'
                ? 'bg-green-50 text-green-700 border border-green-200'
                : 'bg-red-50 text-red-700 border border-red-200'
            }`}
          >
            {testResult === 'success' ? (
              <CheckCircle size={16} />
            ) : (
              <XCircle size={16} />
            )}
            {testMessage}
          </div>
        )}

        {/* Buttons */}
        <div className="flex gap-3 pt-2">
          <button
            onClick={handleSave}
            className="px-5 py-2.5 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 transition-colors"
          >
            {saved ? '✓ 已保存' : '保存配置'}
          </button>
          <button
            onClick={handleTest}
            disabled={testing || !apiBase || !apiKey}
            className="px-5 py-2.5 bg-white text-gray-700 text-sm font-medium rounded-lg border border-gray-300 hover:bg-gray-50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          >
            {testing && <Loader2 size={14} className="animate-spin" />}
            测试连接
          </button>
        </div>
      </div>
    </div>
  );
}