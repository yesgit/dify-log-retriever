import { useState, useEffect } from 'react';
import { Settings, CheckCircle, XCircle, Loader2, LogIn, Key } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

type AuthMode = 'login' | 'token';

export function ConfigPage() {
  const [authMode, setAuthMode] = useState<AuthMode>('login');
  const [apiBase, setApiBase] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [proxy, setProxy] = useState('');
  const [testing, setTesting] = useState(false);
  const [logging, setLogging] = useState(false);
  const [testResult, setTestResult] = useState<'success' | 'error' | null>(null);
  const [testMessage, setTestMessage] = useState('');
  const [saved, setSaved] = useState(false);
  const [hasExistingKey, setHasExistingKey] = useState(false);

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      const config = await invoke<{
        api_base: string;
        api_key_masked: string;
        proxy?: string;
        has_key: boolean;
        auth_mode: string;
        auth_email?: string;
      }>('get_config');
      if (config) {
        setApiBase(config.api_base || '');
        setApiKey(config.api_key_masked || '');
        setProxy(config.proxy || '');
        setHasExistingKey(config.has_key);
        const mode = config.auth_mode === 'login' ? 'login' : 'token';
        setAuthMode(mode);
        if (config.auth_email) {
          setEmail(config.auth_email);
        }
      }
    } catch (e) {
      // Config not found yet, that's OK
    }
  };

  const validateBase = (): string | null => {
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
    return null;
  };

  const handleLogin = async () => {
    const baseError = validateBase();
    if (baseError) {
      setTestResult('error');
      setTestMessage(baseError);
      return;
    }
    if (!email.trim()) {
      setTestResult('error');
      setTestMessage('请输入邮箱');
      return;
    }
    if (!password.trim()) {
      setTestResult('error');
      setTestMessage('请输入密码');
      return;
    }

    setLogging(true);
    setTestResult(null);
    try {
      await invoke<string>('login_to_dify', {
        apiBase: apiBase.trim().replace(/\/+$/, ''),
        email: email.trim(),
        password: password.trim(),
        proxy: proxy.trim() || null,
      });
      setTestResult('success');
      setTestMessage('登录成功！Token 已自动获取并保存');
      setHasExistingKey(true);
      // Reload to get masked key
      const config = await invoke<{
        api_base: string;
        api_key_masked: string;
        proxy?: string;
        has_key: boolean;
      }>('get_config');
      if (config) {
        setApiKey(config.api_key_masked);
      }
    } catch (e: any) {
      setTestResult('error');
      setTestMessage(`登录失败: ${e}`);
    } finally {
      setLogging(false);
    }
  };

  const handleSave = async () => {
    if (authMode === 'token') {
      if (!apiBase.trim()) {
        setTestResult('error');
        setTestMessage('请输入 Dify 实例地址');
        return;
      }
      const base = apiBase.trim();
      if (!base.startsWith('http://') && !base.startsWith('https://')) {
        setTestResult('error');
        setTestMessage('实例地址必须以 http:// 或 https:// 开头');
        return;
      }
      if (!apiKey.trim()) {
        setTestResult('error');
        setTestMessage('请输入 Console API Token');
        return;
      }
      if (hasExistingKey && apiKey.includes('****')) {
        // OK, keep existing
      } else if (apiKey.trim().length < 8) {
        setTestResult('error');
        setTestMessage('API Token 长度不足，请输入有效的 Console API Token');
        return;
      }
    } else {
      const baseError = validateBase();
      if (baseError) {
        setTestResult('error');
        setTestMessage(baseError);
        return;
      }
      // In login mode, must have logged in first
      if (!hasExistingKey) {
        setTestResult('error');
        setTestMessage('请先点击「登录获取 Token」完成登录');
        return;
      }
    }

    try {
      const keyToSend = hasExistingKey && apiKey.includes('****') ? '__KEEP_EXISTING__' : apiKey.trim();

      await invoke('save_config', {
        apiBase: apiBase.trim().replace(/\/+$/, ''),
        apiKey: keyToSend,
        proxy: proxy.trim() || null,
        authMode: authMode,
        authEmail: authMode === 'login' ? email.trim() : null,
        authPassword: authMode === 'login' ? password.trim() : null,
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
    if (authMode === 'login') {
      // In login mode, test by logging in
      return handleLogin();
    }

    if (!apiBase.trim()) {
      setTestResult('error');
      setTestMessage('请输入 Dify 实例地址');
      return;
    }
    if (!apiKey.trim()) {
      setTestResult('error');
      setTestMessage('请输入 Console API Token');
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
        {/* Auth Mode Selector */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            认证方式
          </label>
          <div className="flex gap-2">
            <button
              onClick={() => { setAuthMode('login'); setTestResult(null); }}
              className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                authMode === 'login'
                  ? 'bg-blue-100 text-blue-700 border border-blue-300'
                  : 'bg-gray-50 text-gray-600 border border-gray-200 hover:bg-gray-100'
              }`}
            >
              <LogIn size={16} />
              账号登录
            </button>
            <button
              onClick={() => { setAuthMode('token'); setTestResult(null); }}
              className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                authMode === 'token'
                  ? 'bg-blue-100 text-blue-700 border border-blue-300'
                  : 'bg-gray-50 text-gray-600 border border-gray-200 hover:bg-gray-100'
              }`}
            >
              <Key size={16} />
              手动输入 Token
            </button>
          </div>
          <p className="text-xs text-gray-400 mt-1">
            {authMode === 'login'
              ? '使用 Dify 账号密码登录，Token 会自动获取和刷新'
              : '手动输入 Console API Token（Token 过期后需重新配置）'}
          </p>
        </div>

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

        {/* Login Mode */}
        {authMode === 'login' && (
          <>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1.5">
                邮箱
              </label>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="admin@example.com"
                className="w-full px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
              <p className="text-xs text-gray-400 mt-1">
                Dify 管理员账号邮箱
              </p>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1.5">
                密码
              </label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="输入密码"
                className="w-full px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
              {hasExistingKey && (
                <p className="text-xs text-green-600 mt-1">
                  ✓ 已登录（Token 已自动获取）。如需更换账号请输入新的邮箱和密码
                </p>
              )}
              {!hasExistingKey && (
                <p className="text-xs text-gray-400 mt-1">
                  密码仅保存在本地，用于自动刷新 Token
                </p>
              )}
            </div>
          </>
        )}

        {/* Token Mode */}
        {authMode === 'token' && (
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
        )}

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
          {authMode === 'login' ? (
            <button
              onClick={handleLogin}
              disabled={logging || !apiBase || !email || !password}
              className="px-5 py-2.5 bg-green-600 text-white text-sm font-medium rounded-lg hover:bg-green-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              {logging && <Loader2 size={14} className="animate-spin" />}
              <LogIn size={14} />
              登录获取 Token
            </button>
          ) : (
            <button
              onClick={handleTest}
              disabled={testing || !apiBase || !apiKey}
              className="px-5 py-2.5 bg-white text-gray-700 text-sm font-medium rounded-lg border border-gray-300 hover:bg-gray-50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              {testing && <Loader2 size={14} className="animate-spin" />}
              测试连接
            </button>
          )}
        </div>
      </div>
    </div>
  );
}