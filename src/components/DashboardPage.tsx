import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  BarChart3, MessageSquare, Users, Zap, ThumbsUp, ThumbsDown, Minus,
  AlertTriangle, Clock, Activity, Filter, Info
} from 'lucide-react';
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip as RechartsTooltip,
  ResponsiveContainer, Legend
} from 'recharts';
import type { DashboardStats, DailyStats, DifyApp, StatDistribution, FeedbackLabelStat } from '../types';

// ===== Time Range Presets =====
interface TimePreset {
  label: string;
  getRange: () => [number, number];
}

const now = () => Math.floor(Date.now() / 1000);

const startOfWeek = (d: Date) => {
  const r = new Date(d);
  const day = r.getDay();
  const diff = day === 0 ? 6 : day - 1;
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
  const [selectedPresetIdx, setSelectedPresetIdx] = useState<number>(2);
  const [customStart, setCustomStart] = useState<string>('');
  const [customEnd, setCustomEnd] = useState<string>('');
  const [useCustom, setUseCustom] = useState(false);

  useEffect(() => { loadApps(); }, []);

  const loadApps = async () => {
    try {
      const result = await invoke<DifyApp[]>('get_local_apps');
      setApps(result);
    } catch (e) { console.error(e); }
  };

  const loadStats = useCallback(async () => {
    setLoading(true);
    try {
      let startTime: number | undefined;
      let endTime: number | undefined;
      if (useCustom) {
        if (customStart) startTime = Math.floor(new Date(customStart).getTime() / 1000);
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
    } catch (e) { console.error(e); } finally { setLoading(false); }
  }, [selectedAppId, selectedPresetIdx, useCustom, customStart, customEnd]);

  useEffect(() => { loadStats(); }, [loadStats]);

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

        <div className="bg-white rounded-xl border border-gray-200 p-4 space-y-3">
          <div className="flex items-center gap-2 text-sm font-medium text-gray-700">
            <Filter size={16} />
            <span>筛选条件</span>
          </div>
          <div className="flex flex-wrap items-center gap-3">
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
          {useCustom && (
            <div className="flex items-center gap-3">
              <input type="date" value={customStart} onChange={(e) => setCustomStart(e.target.value)} className="px-3 py-2 border border-gray-300 rounded-lg text-sm" />
              <span className="text-gray-400">至</span>
              <input type="date" value={customEnd} onChange={(e) => setCustomEnd(e.target.value)} className="px-3 py-2 border border-gray-300 rounded-lg text-sm" />
            </div>
          )}
        </div>
      </div>

      {!stats ? (
        <div className="mt-10 text-center text-gray-500">暂无统计数据，请先同步数据</div>
      ) : (
        <>
          {/* Basic Count Cards */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
            <StatCard icon={<Users size={18} className="text-blue-500" />} label="用户数" value={stats.total_users}
              tooltip="独立终端用户数（基于 from_end_user_id 去重统计）" />
            <StatCard icon={<MessageSquare size={18} className="text-green-500" />} label="会话数" value={stats.total_conversations}
              tooltip="会话（Conversation）总数，每个会话包含多条消息" />
            <StatCard icon={<Activity size={18} className="text-cyan-500" />} label="消息数" value={stats.total_messages}
              tooltip="用户提问消息数量，即 query 非空的消息数" />
            <StatCard icon={<Zap size={18} className="text-orange-500" />} label="应用数" value={stats.total_apps}
              tooltip="筛选范围内的应用数量" />
          </div>

          {/* Average Distributions */}
          <Section title="平均指标分布">
            {stats.messages_per_conversation_distribution && (
              <div className="mb-4">
                <DistributionTable
                  title="会话消息数分布"
                  dist={stats.messages_per_conversation_distribution}
                  format={(v) => v.toFixed(1)}
                  tooltip="每个会话包含的用户提问消息数分布。样本为筛选范围内的每个会话，统计其消息数量。"
                />
              </div>
            )}
            {stats.conversations_per_user_distribution && (
              <div className="mb-4">
                <DistributionTable
                  title="用户会话数分布"
                  dist={stats.conversations_per_user_distribution}
                  format={(v) => v.toFixed(1)}
                  tooltip="每个终端用户发起的会话数分布。样本为筛选范围内的每个独立用户，统计其会话数量。"
                />
              </div>
            )}
            {stats.messages_per_user_distribution && (
              <DistributionTable
                title="用户消息数分布"
                dist={stats.messages_per_user_distribution}
                format={(v) => v.toFixed(1)}
                tooltip="每个终端用户的提问消息数分布。样本为筛选范围内的每个独立用户，统计其消息数量。"
              />
            )}
          </Section>

          {/* Feedback Stats */}
          <Section title="用户反馈">
            <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-4 mb-4">
              <StatCard icon={<ThumbsUp size={16} className="text-green-500" />} label="赞数" value={stats.feedback_like} small
                tooltip="反馈为 like 的消息数" />
              <StatCard icon={<ThumbsDown size={16} className="text-red-500" />} label="踩数" value={stats.feedback_dislike} small
                tooltip="反馈为 dislike 的消息数" />
              <StatCard icon={<Minus size={16} className="text-gray-400" />} label="无反馈" value={stats.feedback_none} small
                tooltip="没有任何反馈的消息数" />
              <SmallStat label="反馈总数" value={stats.feedback_total}
                tooltip="有反馈的消息总数 = 赞数 + 踩数" />
              <SmallStat label="有内容反馈数" value={stats.feedback_with_content}
                tooltip="反馈中至少包含 label（rating）或 content 不为空的记录数" />
            </div>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <SmallStat label="好评率" value={formatPercent(stats.feedback_like_rate)}
                tooltip="好评率 = 赞数 / (赞数 + 踩数) × 100%" />
              <SmallStat label="用户平均反馈数" value={stats.avg_feedback_per_user.toFixed(2)}
                tooltip="平均每个用户提交的反馈数 = 反馈总数 / 用户数" />
              <SmallStat label="会话平均反馈数" value={stats.avg_feedback_per_conversation.toFixed(2)}
                tooltip="平均每个会话的反馈数 = 反馈总数 / 会话数" />
              <SmallStat label="消息平均反馈数" value={stats.avg_feedback_per_message.toFixed(2)}
                tooltip="平均每条消息的反馈数 = 反馈总数 / 消息数" />
            </div>

            {/* Feedback label stats */}
            {stats.feedback_label_stats && stats.feedback_label_stats.length > 0 && (
              <div className="mt-4">
                <h4 className="text-sm font-medium text-gray-700 mb-2">反馈分类统计</h4>
                <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-3">
                  {stats.feedback_label_stats.map((item) => (
                    <FeedbackLabelCard key={item.feedback} item={item} />
                  ))}
                </div>
              </div>
            )}
          </Section>

          {/* Error Stats */}
          <Section title="异常统计">
            <div className="grid grid-cols-2 gap-4">
              <StatCard icon={<AlertTriangle size={18} className="text-red-500" />} label="异常消息数" value={stats.error_count} small
                tooltip="标记为异常的消息数（error 字段非空或 status 为 error）" />
              <SmallStat label="异常率" value={formatPercent(stats.error_rate)}
                tooltip="异常率 = 异常消息数 / 消息总数 × 100%" />
            </div>
          </Section>

          {/* Token Stats */}
          <Section title="Token 消耗">
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
              <SmallStat label="Prompt Tokens" value={formatStatNumber(stats.total_prompt_tokens)}
                tooltip="所有消息的 Prompt Token 用量（输入部分）" />
              <SmallStat label="Answer Tokens" value={formatStatNumber(stats.total_answer_tokens)}
                tooltip="所有消息的 Answer Token 用量（输出部分）" />
              <SmallStat label="总 Token 量" value={formatStatNumber(stats.total_tokens)}
                tooltip="总 Token = Prompt Tokens + Answer Tokens，单位为 Dify 报告的 Token 数" />
              <SmallStat label="日均 Token 消耗" value={formatStatNumber(Math.round(stats.daily_avg_tokens))}
                tooltip="日均消耗 = 总 Token 量 / 时间范围内的天数" />
            </div>
            {stats.token_per_message_distribution && (
              <DistributionTable
                title="每条消息 Token 消耗分布"
                dist={stats.token_per_message_distribution}
                format={(v) => Math.round(v).toLocaleString()}
                tooltip="统计每条消息的 Token 消耗（Prompt + Answer），样本为 Token 消耗大于 0 的消息。单位为 Dify 报告的 Token 数。"
              />
            )}
          </Section>

          {/* Response Time */}
          <Section title="响应时间">
            {stats.ttft_distribution && (
              <div className="mb-4">
                <h4 className="text-sm font-medium text-gray-700 mb-2 flex items-center gap-1.5">
                  <Clock size={14} /> 首 Token 时间 (TTFT)
                </h4>
                <DistributionTable
                  dist={stats.ttft_distribution}
                  format={formatTime}
                  tooltip="首 Token 时间（Time To First Token），从请求发出到收到第一个 Token 的延迟。样本仅包含有延迟数据的消息。"
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
                  tooltip="从请求发出到响应完成的总时间。样本仅包含有时间记录的消息。"
                />
              </div>
            )}
          </Section>

          {/* Token Speed */}
          {stats.token_speed_distribution && (
            <Section title="Token 生成速度">
              <DistributionTable
                dist={stats.token_speed_distribution}
                format={formatTokensPerSec}
                tooltip="Token 生成速度 = Answer Tokens / 响应时间。样本仅包含有有效时间和 Token 的消息。"
              />
            </Section>
          )}

          {/* Feedback Distributions */}
          <Section title="反馈数分布">
            {stats.user_feedback_count_distribution && (
              <div className="mb-4">
                <DistributionTable
                  title="用户反馈数分布"
                  dist={stats.user_feedback_count_distribution}
                  format={(v) => v.toFixed(1)}
                  tooltip="每个终端用户的反馈数量分布（包含无反馈的用户，反馈数为 0）"
                />
              </div>
            )}
            {stats.conversation_feedback_count_distribution && (
              <div className="mb-4">
                <DistributionTable
                  title="会话反馈数分布"
                  dist={stats.conversation_feedback_count_distribution}
                  format={(v) => v.toFixed(1)}
                  tooltip="每个会话收到的反馈数量分布（包含无反馈的会话，反馈数为 0）"
                />
              </div>
            )}
            {stats.message_feedback_count_distribution && (
              <div>
                <DistributionTable
                  title="消息反馈数分布"
                  dist={stats.message_feedback_count_distribution}
                  format={(v) => v.toFixed(1)}
                  tooltip="每条消息收到的反馈数量分布"
                />
              </div>
            )}
          </Section>

          {/* Daily Trend */}
          <Section title="趋势图">
            {stats.recent_daily && stats.recent_daily.length > 0 ? (
              <DailyTrendChart data={stats.recent_daily} />
            ) : (
              <p className="text-sm text-gray-400">暂无数据</p>
            )}
          </Section>

          {/* Top Apps */}
          {!selectedAppId && stats.top_apps && stats.top_apps.length > 0 && (
            <Section title="应用排名 (按会话数)">
              <div className="space-y-3">
                {stats.top_apps.map((app, idx) => (
                  <div key={app.app_id} className="flex items-center gap-3">
                    <span className="text-sm font-bold text-gray-400 w-6">#{idx + 1}</span>
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium text-gray-900 truncate">{app.app_name}</p>
                      <p className="text-xs text-gray-400">
                        {app.conversation_count} 会话 · {app.message_count} 消息
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

// ===== Shared formatting =====
function formatStatNumber(n: number): string {
  if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`;
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
  return n.toLocaleString();
}

// ===== Sub-components =====

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="bg-white rounded-xl border border-gray-200 p-5 mb-6">
      <h3 className="font-semibold text-gray-900 mb-4 flex items-center gap-2">{title}</h3>
      {children}
    </div>
  );
}

function TooltipIcon({ text }: { text: string }) {
  return (
    <span className="group relative inline-flex items-center ml-1 cursor-help">
      <Info size={12} className="text-gray-400 hover:text-gray-600" />
      <span className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-2 text-xs text-white bg-gray-800 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity whitespace-normal max-w-[260px] min-w-[120px] text-center pointer-events-none z-50 shadow-lg">
        {text}
        <span className="absolute top-full left-1/2 -translate-x-1/2 -mt-1 border-4 border-transparent border-t-gray-800" />
      </span>
    </span>
  );
}

function StatCard({
  icon, label, value, small, tooltip,
}: {
  icon: React.ReactNode;
  label: string;
  value: number;
  small?: boolean;
  tooltip?: string;
}) {
  return (
    <div className={`${small ? 'p-3' : 'p-5'} bg-gray-50 rounded-lg`}>
      <div className={`flex items-center gap-2 ${small ? 'mb-1' : 'mb-3'}`}>
        <div className="p-1.5 rounded-lg bg-white">{icon}</div>
        <span className="text-xs text-gray-500 flex items-center">
          {label}
          {tooltip && <TooltipIcon text={tooltip} />}
        </span>
      </div>
      <p className={`${small ? 'text-lg' : 'text-2xl'} font-bold text-gray-900`}>
        {formatStatNumber(value)}
      </p>
    </div>
  );
}

function SmallStat({ label, value, tooltip }: { label: string; value: string | number; tooltip?: string }) {
  return (
    <div className="p-3 bg-gray-50 rounded-lg">
      <p className="text-xs text-gray-500 mb-1 flex items-center">
        {label}
        {tooltip && <TooltipIcon text={tooltip} />}
      </p>
      <p className="text-lg font-bold text-gray-900">{value}</p>
    </div>
  );
}

function FeedbackLabelCard({ item }: { item: FeedbackLabelStat }) {
  const labelMap: Record<string, { bg: string; text: string; icon: string; label: string }> = {
    like: { bg: 'bg-green-50 border-green-200', text: 'text-green-700', icon: '👍', label: '赞' },
    dislike: { bg: 'bg-red-50 border-red-200', text: 'text-red-700', icon: '👎', label: '踩' },
    none: { bg: 'bg-gray-50 border-gray-200', text: 'text-gray-600', icon: '➖', label: '无反馈' },
  };
  const c = labelMap[item.feedback] || { bg: 'bg-blue-50 border-blue-200', text: 'text-blue-700', icon: '📌', label: item.feedback };
  return (
    <div className={`p-3 rounded-lg border ${c.bg}`}>
      <p className={`text-xs font-medium ${c.text}`}>
        {c.icon} {c.label}
      </p>
      <p className="text-lg font-bold text-gray-900 mt-1">{item.count.toLocaleString()}</p>
    </div>
  );
}

function DistributionTable({
  title,
  dist,
  format,
  tooltip,
}: {
  title?: string;
  dist: StatDistribution;
  format: (v: number) => string;
  tooltip?: string;
}) {
  return (
    <div>
      {title && (
        <h4 className="text-sm font-medium text-gray-700 mb-2 flex items-center">
          {title}
          {tooltip && <TooltipIcon text={tooltip} />}
        </h4>
      )}
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
  const formatTickNumber = (v: number) => {
    if (v >= 1000000) return `${(v / 1000000).toFixed(0)}M`;
    if (v >= 1000) return `${(v / 1000).toFixed(0)}K`;
    return v.toString();
  };

  return (
    <div className="space-y-6">
      {/* Conversations & Queries */}
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日会话数 & 消息数</h4>
        <ResponsiveContainer width="100%" height={240}>
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis dataKey="date" tick={{ fontSize: 11 }} stroke="#9ca3af" />
            <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" tickFormatter={formatTickNumber} />
            <RechartsTooltip
              contentStyle={{ fontSize: 12, borderRadius: 8 }}
              formatter={(v: any, name: any) => [Number(v).toLocaleString(), String(name)]}
            />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            <Line type="monotone" dataKey="conversations" name="会话数" stroke="#3b82f6" strokeWidth={2} dot={false} />
            <Line type="monotone" dataKey="messages" name="消息数" stroke="#22c55e" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* Daily Users */}
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日用户数</h4>
        <ResponsiveContainer width="100%" height={240}>
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis dataKey="date" tick={{ fontSize: 11 }} stroke="#9ca3af" />
            <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" tickFormatter={formatTickNumber} />
            <RechartsTooltip
              contentStyle={{ fontSize: 12, borderRadius: 8 }}
              formatter={(v: any, name: any) => [Number(v).toLocaleString(), String(name)]}
            />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            <Line type="monotone" dataKey="users" name="用户数" stroke="#8b5cf6" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* Token trend */}
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日 Token 消耗</h4>
        <ResponsiveContainer width="100%" height={240}>
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis dataKey="date" tick={{ fontSize: 11 }} stroke="#9ca3af" />
            <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" tickFormatter={formatTickNumber} />
            <RechartsTooltip
              contentStyle={{ fontSize: 12, borderRadius: 8 }}
              formatter={(v: any, name: any) => [Number(v).toLocaleString(), String(name)]}
            />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            <Line type="monotone" dataKey="tokens" name="Token 消耗" stroke="#f97316" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}