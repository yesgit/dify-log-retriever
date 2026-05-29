import { useState, useEffect } from 'react';
import { BarChart3, MessageSquare, Users, Zap, ThumbsUp, ThumbsDown, Minus, TrendingUp } from 'lucide-react';

interface DashboardStats {
  total_apps: number;
  total_conversations: number;
  total_messages: number;
  total_answer_tokens: number;
  total_prompt_tokens: number;
  feedback_like: number;
  feedback_dislike: number;
  feedback_none: number;
  top_apps: { app_id: string; app_name: string; conversation_count: number; message_count: number }[];
  recent_daily: { date: string; conversations: number; messages: number }[];
}

export function DashboardPage() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadStats();
  }, []);

  const loadStats = async () => {
    try {
      const result = await (window as any).__TAURI__.invoke('get_dashboard_stats');
      setStats(result);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const formatNumber = (n: number) => {
    if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`;
    if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
    return n.toString();
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" />
      </div>
    );
  }

  if (!stats) {
    return (
      <div>
        <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
          <BarChart3 size={24} />
          数据看板
        </h2>
        <div className="mt-10 text-center text-gray-500">
          暂无统计数据，请先同步数据
        </div>
      </div>
    );
  }

  const maxConv = Math.max(...(stats.recent_daily || []).map((d) => d.conversations), 1);

  return (
    <div>
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
          <BarChart3 size={24} />
          数据看板
        </h2>
        <p className="text-gray-500 mt-1">数据统计概览</p>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-4 gap-4 mb-6">
        <StatCard
          icon={<Users size={20} className="text-blue-500" />}
          label="应用数"
          value={stats.total_apps}
          color="blue"
        />
        <StatCard
          icon={<MessageSquare size={20} className="text-green-500" />}
          label="对话数"
          value={stats.total_conversations}
          color="green"
        />
        <StatCard
          icon={<TrendingUp size={20} className="text-purple-500" />}
          label="消息数"
          value={stats.total_messages}
          color="purple"
        />
        <StatCard
          icon={<Zap size={20} className="text-orange-500" />}
          label="总 Tokens"
          value={stats.total_prompt_tokens + stats.total_answer_tokens}
          color="orange"
        />
      </div>

      <div className="grid grid-cols-2 gap-6">
        {/* Token Usage */}
        <div className="bg-white rounded-xl border border-gray-200 p-5">
          <h3 className="font-semibold text-gray-900 mb-4">Token 用量</h3>
          <div className="space-y-3">
            <div className="flex justify-between items-center">
              <span className="text-sm text-gray-600">Prompt Tokens</span>
              <span className="text-sm font-medium text-gray-900">{formatNumber(stats.total_prompt_tokens)}</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-sm text-gray-600">Answer Tokens</span>
              <span className="text-sm font-medium text-gray-900">{formatNumber(stats.total_answer_tokens)}</span>
            </div>
            <div className="border-t pt-3 flex justify-between items-center">
              <span className="text-sm font-medium text-gray-900">合计</span>
              <span className="text-sm font-bold text-blue-600">
                {formatNumber(stats.total_prompt_tokens + stats.total_answer_tokens)}
              </span>
            </div>
          </div>
        </div>

        {/* Feedback Stats */}
        <div className="bg-white rounded-xl border border-gray-200 p-5">
          <h3 className="font-semibold text-gray-900 mb-4">用户反馈</h3>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <ThumbsUp size={16} className="text-green-500" />
                <span className="text-sm text-gray-600">好评</span>
              </div>
              <span className="text-sm font-medium text-green-600">{stats.feedback_like}</span>
            </div>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <ThumbsDown size={16} className="text-red-500" />
                <span className="text-sm text-gray-600">差评</span>
              </div>
              <span className="text-sm font-medium text-red-600">{stats.feedback_dislike}</span>
            </div>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Minus size={16} className="text-gray-400" />
                <span className="text-sm text-gray-600">无反馈</span>
              </div>
              <span className="text-sm font-medium text-gray-500">{stats.feedback_none}</span>
            </div>
          </div>
        </div>

        {/* Daily Trend */}
        <div className="bg-white rounded-xl border border-gray-200 p-5">
          <h3 className="font-semibold text-gray-900 mb-4">近 7 天对话趋势</h3>
          {stats.recent_daily && stats.recent_daily.length > 0 ? (
            <div className="space-y-2">
              {stats.recent_daily.map((day) => (
                <div key={day.date} className="flex items-center gap-3">
                  <span className="text-xs text-gray-500 w-20">{day.date}</span>
                  <div className="flex-1 bg-gray-100 rounded-full h-5 relative overflow-hidden">
                    <div
                      className="bg-blue-500 rounded-full h-full transition-all"
                      style={{ width: `${(day.conversations / maxConv) * 100}%` }}
                    />
                  </div>
                  <span className="text-xs text-gray-600 w-12 text-right">{day.conversations}</span>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-gray-400">暂无数据</p>
          )}
        </div>

        {/* Top Apps */}
        <div className="bg-white rounded-xl border border-gray-200 p-5">
          <h3 className="font-semibold text-gray-900 mb-4">应用排名 (按对话数)</h3>
          {stats.top_apps && stats.top_apps.length > 0 ? (
            <div className="space-y-3">
              {stats.top_apps.map((app, idx) => (
                <div key={app.app_id} className="flex items-center gap-3">
                  <span className="text-sm font-bold text-gray-400 w-6">#{idx + 1}</span>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-gray-900 truncate">{app.app_name}</p>
                    <p className="text-xs text-gray-400">
                      {app.conversation_count} 对话 · {app.message_count} 消息
                    </p>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-gray-400">暂无数据</p>
          )}
        </div>
      </div>
    </div>
  );
}

function StatCard({ icon, label, value, color }: { icon: React.ReactNode; label: string; value: number; color: string }) {
  const bgColors: Record<string, string> = {
    blue: 'bg-blue-50',
    green: 'bg-green-50',
    purple: 'bg-purple-50',
    orange: 'bg-orange-50',
  };
  return (
    <div className="bg-white rounded-xl border border-gray-200 p-5">
      <div className="flex items-center gap-3 mb-3">
        <div className={`p-2 rounded-lg ${bgColors[color] || 'bg-gray-50'}`}>{icon}</div>
        <span className="text-sm text-gray-500">{label}</span>
      </div>
      <p className="text-2xl font-bold text-gray-900">
        {value >= 1000 ? (value / 1000).toFixed(1) + 'K' : value.toLocaleString()}
      </p>
    </div>
  );
}