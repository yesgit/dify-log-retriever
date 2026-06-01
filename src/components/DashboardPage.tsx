import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  BarChart3, MessageSquare, Users, Zap, ThumbsUp, ThumbsDown, Minus,
  AlertTriangle, Clock, Hash, Activity, Filter
} from 'lucide-react';
import type { DashboardStats, DailyStats, DifyApp, StatDistribution } from '../types';

// ===== Time Range Presets =====
interface TimePreset {
  label: string;
  getRange: () => [number, number]; // [start_ts, end_ts]
}

const now = () => Math.floor(Date.now() / 1000);

const startOfWeek = (d: Date) => {
  const r = new Date(d);
  const day = r.getDay();
  const diff = day === 0 ? 6 : day - 1; // Monday as start
  r.setDate(r.getDate() - diff);
  r.setHours(0, 0, 0, 0);
  return Math.floor(r.getTime() / 1000);
};

const startOfMonth = (d: Date) => {
  const r = new Date(d.getFullYear(), d.getMonth(), 1);
  return Math.floor(r.getTime() / 1000);
};

const startOfQuarter = (d: Date) => {
  const q = Math.floor(d.getMonth() / 3);
  const r = new Date(d.getFullYear(), q * 3, 1);
  return Math.floor(r.getTime() / 1000);
};

const startOfYear = (d: Date) => {
  const r = new Date(d.getFullYear(), 0, 1);
  return Math.floor(r.getTime() / 1000);
};

const TIME_PRESETS: TimePreset[] = [
  { label: '过去24小时', getRange: () => [now() - 86400, now()] },
  { label: '过去3天', getRange: () => [now() - 3 * 86400, now()] },
  { label: '过去7天', getRange: () => [now() - 7 * 86400, now()] },
  { label: '过去15天', getRange: () => [now() - 15 * 86400, now()] },
  { label: '过去30天', getRange: () => [now() - 30 * 86400, now()] },
  { label: '过去3个月', getRange: () => [now() - 90 * 86400, now()] },
  { label: '过去半年', getRange: () => [now() - 180 * 86400, now()] },
  { label: '过去1年', getRange: () => [now() - 365 * 86400, now()] },
  { label: '过去3年', getRange: () => [now() - 1095 * 86400, now()] },
  { label: '本周至今', getRange: () => [startOfWeek(new Date()), now()] },
  { label: '本月至今', getRange: () => [startOfMonth(new Date()), now()] },
  { label: '本季度至今', getRange: () => [startOfQuarter(new Date()), now()] },
  { label: '本年度至今', getRange: () => [startOfYear(new Date()), now()] },
];

export function DashboardPage() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [apps, setApps] = useState<DifyApp[]>([]);
  const [selectedAppId, setSelectedAppId] = useState<string>('');
  const [selectedPresetIdx, setSelectedPresetIdx] = useState<number>(2); // default: 过去7天
  const [customStart, setCustomStart] = useState<string>('');
  const [customEnd, setCustomEnd] = useState<string>('');
  const [useCustom, setUseCustom] = useState(false);

  useEffect(() => {
    loadApps();
  }, []);

  const loadApps = async () => {
    try {
      const result = await invoke<DifyApp[]>('get_local_apps');
      setApps(result);
    } catch (e) {
      console.error(e);
    }
  };

  const loadStats = useCallback(async () => {
    setLoading(true);
    try {
      let startTime: number | undefined;
      let endTime: number | undefined;

      if (useCustom) {
        if (customStart) {
          startTime = Math.floor(new Date(customStart).getTime() / 1000);
        }
        if (customEnd) {
          const d = new Date(customEnd);
          d.setHours(23, 59, 59);
          endTime = Math.floor(d.getTime() / 1000);
        }
      } else {
        const [s, e] = TIME_PRESETS[selectedPresetIdx].getRange();
        startTime = s;
        endTime = e;
      }

      const result = await invoke<DashboardStats>('get_dashboard_stats', {
        appId: selectedAppId || null,
        startTime,
        endTime,
      });
      setStats(result);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, [selectedAppId, selectedPresetIdx, useCustom, customStart, customEnd]);

  useEffect(() => {
    loadStats();
  }, [loadStats]);

  const formatNumber = (n: number) => {
    if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`;
    if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
    return n.toLocaleString();
  };

  const formatPercent = (n: number) => `${n.toFixed(1)}%`;

  const formatTime = (seconds: number) => {
    if (seconds < 1) return `${(seconds * 1000).toFixed(0)}ms`;
    if (seconds < 60) return `${seconds.toFixed(2)}s`;
    return `${(seconds / 60).toFixed(1)}min`;
  };

  const formatTokensPerSec = (speed: number) => `${speed.toFixed(1)} t/s`;

  if (loading && !stats) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" />
      </div>
    );
  }

  return (
    <div>
      {/* Header with filters */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
              <BarChart3 size={24} />
              数据看板
            </h2>
            <p className="text-gray-500 mt-1">数据统计概览</p>
          </div>
          <button
            onClick={loadStats}
            className="px-3 py-1.5 text-sm bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors"
          >
            刷新
          </button>
        </div>

        {/* Filter bar */}
        <div className="bg-white rounded-xl border border-gray-200 p-4 space-y-3">
          <div className="flex items-center gap-2 text-sm font-medium text-gray-700">
            <Filter size={16} />
            <span>筛选条件</span>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            {/* App selector */}
            <select
              value={selectedAppId}
              onChange={(e) => setSelectedAppId(e.target.value)}
              className="px-3 py-2 border border-gray-300 rounded-lg text-sm bg-white min-w-[180px]"
            >
              <option value="">全部应用</option>
              {apps.map((app) => (
                <option key={app.id} value={app.id}>{app.name}</option>
              ))}
            </select>

            {/* Time preset buttons */}
            <div className="flex flex-wrap gap-1.5">
              {TIME_PRESETS.map((preset, idx) => (
                <button
                  key={idx}
                  onClick={() => { setSelectedPresetIdx(idx); setUseCustom(false); }}
                  className={`px-2.5 py-1.5 text-xs rounded-lg transition-colors ${
                    !useCustom && selectedPresetIdx === idx
                      ? 'bg-blue-500 text-white'
                      : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                  }`}
                >
                  {preset.label}
                </button>
              ))}
              <button
                onClick={() => setUseCustom(true)}
                className={`px-2.5 py-1.5 text-xs rounded-lg transition-colors ${
                  useCustom ? 'bg-blue-500 text-white' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                }`}
              >
                自定义
              </button>
            </div>
          </div>

          {/* Custom date range */}
          {useCustom && (
            <div className="flex items-center gap-3">
              <input
                type="date"
                value={customStart}
                onChange={(e) => setCustomStart(e.target.value)}
                className="px-3 py-2 border border-gray-300 rounded-lg text-sm"
              />
              <span className="text-gray-400">至</span>
              <input
                type="date"
                value={customEnd}
                onChange={(e) => setCustomEnd(e.target.value)}
                className="px-3 py-2 border border-gray-300 rounded-lg text-sm"
              />
            </div>
          )}
        </div>
      </div>

      {!stats ? (
        <div className="mt-10 text-center text-gray-500">暂无统计数据，请先同步数据</div>
      ) : (
        <>
          {/* ═══ Basic Count Cards ═══ */}
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-4 mb-6">
            <StatCard icon={<Users size={18} className="text-blue-500" />} label="用户数" value={stats.total_users} />
            <StatCard icon={<MessageSquare size={18} className="text-green-500" />} label="会话数" value={stats.total_conversations} />
            <StatCard icon={<Hash size={18} className="text-purple-500" />} label="问答数" value={stats.total_queries} />
            <StatCard icon={<Activity size={18} className="text-cyan-500" />} label="消息数" value={stats.total_messages} />
            <StatCard icon={<Zap size={18} className="text-orange-500" />} label="应用数" value={stats.total_apps} />
          </div>

          {/* ═══ Average Metrics ═══ */}
          <Section title="平均指标">
            <div className="grid grid-cols-3 gap-4">
              <AvgCard label="会话平均问答数" value={stats.avg_queries_per_conversation} decimals={1} />
              <AvgCard label="用户平均会话数" value={stats.avg_conversations_per_user} decimals={1} />
              <AvgCard label="用户平均问答数" value={stats.avg_queries_per_user} decimals={1} />
            </div>
          </Section>

          {/* ═══ Feedback Stats ═══ */}
          <Section title="用户反馈">
            <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-4 mb-4">
              <StatCard icon={<ThumbsUp size={16} className="text-green-500" />} label="赞数" value={stats.feedback_like} small />
              <StatCard icon={<ThumbsDown size={16} className="text-red-500" />} label="踩数" value={stats.feedback_dislike} small />
              <StatCard icon={<Minus size={16} className="text-gray-400" />} label="无反馈" value={stats.feedback_none} small />
              <SmallStat label="反馈总数" value={stats.feedback_total} />
              <SmallStat label="有内容反馈数" value={stats.feedback_with_content} />
            </div>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <SmallStat label="好评率" value={formatPercent(stats.feedback_like_rate)} />
              <SmallStat label="用户平均反馈数" value={stats.avg_feedback_per_user.toFixed(2)} />
              <SmallStat label="会话平均反馈数" value={stats.avg_feedback_per_conversation.toFixed(2)} />
              <SmallStat label="问答平均反馈数" value={stats.avg_feedback_per_query.toFixed(2)} />
            </div>
          </Section>

          {/* ═══ Error Stats ═══ */}
          <Section title="异常统计">
            <div className="grid grid-cols-2 gap-4">
              <StatCard icon={<AlertTriangle size={18} className="text-red-500" />} label="异常消息数" value={stats.error_count} small />
              <SmallStat label="异常率" value={formatPercent(stats.error_rate)} />
            </div>
          </Section>

          {/* ═══ Token Stats ═══ */}
          <Section title="Token 消耗">
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
              <SmallStat label="Prompt Tokens" value={formatNumber(stats.total_prompt_tokens)} />
              <SmallStat label="Answer Tokens" value={formatNumber(stats.total_answer_tokens)} />
              <SmallStat label="总 Token 量" value={formatNumber(stats.total_tokens)} />
              <SmallStat label="日均 Token 消耗" value={formatNumber(Math.round(stats.daily_avg_tokens))} />
            </div>
            {stats.token_per_message_distribution && (
              <DistributionTable
                title="每条消息 Token 消耗分布"
                dist={stats.token_per_message_distribution}
                format={(v) => Math.round(v).toLocaleString()}
              />
            )}
          </Section>

          {/* ═══ Response Time ═══ */}
          <Section title="响应时间">
            {stats.ttft_distribution && (
              <div className="mb-4">
                <h4 className="text-sm font-medium text-gray-700 mb-2 flex items-center gap-1.5">
                  <Clock size={14} /> 首 Token 时间 (TTFT)
                </h4>
                <DistributionTable
                  dist={stats.ttft_distribution}
                  format={formatTime}
                />
              </div>
            )}
            {stats.elapsed_time_distribution && (
              <div>
                <h4 className="text-sm font-medium text-gray-700 mb-2 flex items-center gap-1.5">
                  <Clock size={14} /> 总响应时间
                </h4>
                <DistributionTable
                  dist={stats.elapsed_time_distribution}
                  format={formatTime}
                />
              </div>
            )}
          </Section>

          {/* ═══ Token Speed ═══ */}
          {stats.token_speed_distribution && (
            <Section title="Token 生成速度">
              <DistributionTable
                dist={stats.token_speed_distribution}
                format={formatTokensPerSec}
              />
            </Section>
          )}

          {/* ═══ Feedback Distributions ═══ */}
          <Section title="反馈数分布">
            {stats.user_feedback_count_distribution && (
              <div className="mb-4">
                <h4 className="text-sm font-medium text-gray-700 mb-2">用户反馈数分布</h4>
                <DistributionTable
                  dist={stats.user_feedback_count_distribution}
                  format={(v) => v.toFixed(1)}
                />
              </div>
            )}
            {stats.conversation_feedback_count_distribution && (
              <div className="mb-4">
                <h4 className="text-sm font-medium text-gray-700 mb-2">会话反馈数分布</h4>
                <DistributionTable
                  dist={stats.conversation_feedback_count_distribution}
                  format={(v) => v.toFixed(1)}
                />
              </div>
            )}
            {stats.message_feedback_count_distribution && (
              <div>
                <h4 className="text-sm font-medium text-gray-700 mb-2">消息反馈数分布</h4>
                <DistributionTable
                  dist={stats.message_feedback_count_distribution}
                  format={(v) => v.toFixed(1)}
                />
              </div>
            )}
          </Section>

          {/* ═══ Daily Trend ═══ */}
          <Section title="趋势图">
            {stats.recent_daily && stats.recent_daily.length > 0 ? (
              <DailyTrendChart data={stats.recent_daily} />
            ) : (
              <p className="text-sm text-gray-400">暂无数据</p>
            )}
          </Section>

          {/* ═══ Top Apps ═══ */}
          {!selectedAppId && stats.top_apps && stats.top_apps.length > 0 && (
            <Section title="应用排名 (按会话数)">
              <div className="space-y-3">
                {stats.top_apps.map((app, idx) => (
                  <div key={app.app_id} className="flex items-center gap-3">
                    <span className="text-sm font-bold text-gray-400 w-6">#{idx + 1}</span>
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium text-gray-900 truncate">{app.app_name}</p>
                      <p className="text-xs text-gray-400">
                        {app.conversation_count} 会话 · {app.message_count} 问答
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </Section>
          )}
        </>
      )}
    </div>
  );
}

// ===== Sub-components =====

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="bg-white rounded-xl border border-gray-200 p-5 mb-6">
      <h3 className="font-semibold text-gray-900 mb-4 flex items-center gap-2">
        {title}
      </h3>
      {children}
    </div>
  );
}

function StatCard({
  icon, label, value, small,
}: {
  icon: React.ReactNode;
  label: string;
  value: number;
  small?: boolean;
}) {
  return (
    <div className={`${small ? 'p-3' : 'p-5'} bg-gray-50 rounded-lg`}>
      <div className={`flex items-center gap-2 ${small ? 'mb-1' : 'mb-3'}`}>
        <div className="p-1.5 rounded-lg bg-white">{icon}</div>
        <span className="text-xs text-gray-500">{label}</span>
      </div>
      <p className={`${small ? 'text-lg' : 'text-2xl'} font-bold text-gray-900`}>
        {value >= 1000 ? (value / 1000).toFixed(1) + 'K' : value.toLocaleString()}
      </p>
    </div>
  );
}

function SmallStat({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="p-3 bg-gray-50 rounded-lg">
      <p className="text-xs text-gray-500 mb-1">{label}</p>
      <p className="text-lg font-bold text-gray-900">{value}</p>
    </div>
  );
}

function AvgCard({ label, value, decimals = 2 }: { label: string; value: number; decimals?: number }) {
  return (
    <div className="p-4 bg-gray-50 rounded-lg text-center">
      <p className="text-xs text-gray-500 mb-1">{label}</p>
      <p className="text-2xl font-bold text-gray-900">{value.toFixed(decimals)}</p>
    </div>
  );
}

function DistributionTable({
  title,
  dist,
  format,
}: {
  title?: string;
  dist: StatDistribution;
  format: (v: number) => string;
}) {
  return (
    <div>
      {title && <h4 className="text-sm font-medium text-gray-700 mb-2">{title}</h4>}
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-200">
              <th className="text-left py-2 px-3 text-gray-500 font-medium">样本数</th>
              <th className="text-right py-2 px-3 text-gray-500 font-medium">最小</th>
              <th className="text-right py-2 px-3 text-gray-500 font-medium">最大</th>
              <th className="text-right py-2 px-3 text-gray-500 font-medium">平均</th>
              <th className="text-right py-2 px-3 text-blue-500 font-medium">P50</th>
              <th className="text-right py-2 px-3 text-yellow-500 font-medium">P80</th>
              <th className="text-right py-2 px-3 text-red-500 font-medium">P95</th>
            </tr>
          </thead>
          <tbody>
            <tr className="border-b border-gray-100">
              <td className="py-2 px-3 text-gray-600">{dist.count.toLocaleString()}</td>
              <td className="py-2 px-3 text-right font-mono text-gray-900">{format(dist.min)}</td>
              <td className="py-2 px-3 text-right font-mono text-gray-900">{format(dist.max)}</td>
              <td className="py-2 px-3 text-right font-mono text-gray-900">{format(dist.avg)}</td>
              <td className="py-2 px-3 text-right font-mono text-blue-600">{format(dist.p50)}</td>
              <td className="py-2 px-3 text-right font-mono text-yellow-600">{format(dist.p80)}</td>
              <td className="py-2 px-3 text-right font-mono text-red-600">{format(dist.p95)}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}

function DailyTrendChart({ data }: { data: DailyStats[] }) {
  const maxConv = Math.max(...data.map((d) => Math.max(d.conversations, d.queries)), 1);
  const maxTokens = Math.max(...data.map((d) => d.tokens), 1);

  return (
    <div className="space-y-6">
      {/* Conversations trend */}
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日会话数 & 问答数</h4>
        <div className="space-y-1.5">
          {data.map((day) => (
            <div key={day.date} className="flex items-center gap-3">
              <span className="text-xs text-gray-500 w-20 shrink-0">{day.date}</span>
              <div className="flex-1 bg-gray-100 rounded-full h-4 relative overflow-hidden">
                <div
                  className="bg-green-400 rounded-full h-full transition-all absolute left-0 top-0 opacity-50"
                  style={{ width: `${(day.queries / maxConv) * 100}%` }}
                />
                <div
                  className="bg-blue-400 rounded-full h-full transition-all absolute left-0 top-0"
                  style={{ width: `${(day.conversations / maxConv) * 100}%` }}
                />
              </div>
              <span className="text-xs text-gray-600 w-24 text-right shrink-0">
                {day.conversations} / {day.queries}
              </span>
            </div>
          ))}
        </div>
        <div className="flex gap-4 mt-2 text-xs text-gray-500">
          <span className="flex items-center gap-1"><span className="w-3 h-3 bg-blue-400 rounded-sm inline-block" /> 会话数</span>
          <span className="flex items-center gap-1"><span className="w-3 h-3 bg-green-400 opacity-50 rounded-sm inline-block" /> 问答数</span>
        </div>
      </div>

      {/* Token trend */}
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日 Token 消耗</h4>
        <div className="space-y-1.5">
          {data.map((day) => (
            <div key={day.date} className="flex items-center gap-3">
              <span className="text-xs text-gray-500 w-20 shrink-0">{day.date}</span>
              <div className="flex-1 bg-gray-100 rounded-full h-4 relative overflow-hidden">
                <div
                  className="bg-orange-400 rounded-full h-full transition-all"
                  style={{ width: `${(day.tokens / maxTokens) * 100}%` }}
                />
              </div>
              <span className="text-xs text-gray-600 w-20 text-right shrink-0">
                {day.tokens >= 1000 ? `${(day.tokens / 1000).toFixed(1)}K` : day.tokens}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}