import { useState, useEffect } from 'react';
import { type Page } from '../types';
import { invoke } from '@tauri-apps/api/core';
import {
  Settings,
  AppWindow,
  RefreshCw,
  MessageSquare,
  BarChart3,
  Download,
  FileText,
} from 'lucide-react';

interface LayoutProps {
  currentPage: Page;
  onNavigate: (page: Page) => void;
  children: React.ReactNode;
}

const navItems: { page: Page; label: string; icon: React.ReactNode }[] = [
  { page: 'config', label: '连接配置', icon: <Settings size={20} /> },
  { page: 'apps', label: '应用管理', icon: <AppWindow size={20} /> },
  { page: 'sync', label: '数据同步', icon: <RefreshCw size={20} /> },
  { page: 'conversations', label: '对话浏览', icon: <MessageSquare size={20} /> },
  { page: 'dashboard', label: '数据看板', icon: <BarChart3 size={20} /> },
  { page: 'export', label: '数据导出', icon: <Download size={20} /> },
];

export function Layout({ currentPage, onNavigate, children }: LayoutProps) {
  const [version, setVersion] = useState('...');

  useEffect(() => {
    invoke<string>('get_app_version').then(setVersion).catch(() => setVersion(''));
  }, []);

  return (
    <div className="flex h-screen bg-gray-50">
      {/* Sidebar */}
      <aside className="w-60 bg-white border-r border-gray-200 flex flex-col shadow-sm">
        {/* Logo */}
        <div className="px-5 py-4 border-b border-gray-200">
          <div className="flex items-center gap-2">
            <FileText size={24} className="text-blue-600" />
            <div>
              <h1 className="text-base font-bold text-gray-900">Dify Log Retriever</h1>
              <p className="text-xs text-gray-500">对话记录检索工具</p>
            </div>
          </div>
        </div>

        {/* Navigation */}
        <nav className="flex-1 px-3 py-4 space-y-1">
          {navItems.map((item) => (
            <button
              key={item.page}
              onClick={() => onNavigate(item.page)}
              className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors ${
                currentPage === item.page
                  ? 'bg-blue-50 text-blue-700'
                  : 'text-gray-600 hover:bg-gray-100 hover:text-gray-900'
              }`}
            >
              {item.icon}
              {item.label}
            </button>
          ))}
        </nav>

        {/* Footer */}
        <div className="px-5 py-3 border-t border-gray-200">
          <p className="text-xs text-gray-400">v{version}</p>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 overflow-auto">
        <div className="p-6">{children}</div>
      </main>
    </div>
  );
}