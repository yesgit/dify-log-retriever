import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { revealItemInDir, openPath } from '@tauri-apps/plugin-opener';
import html2canvas from 'html2canvas';
import {
  BarChart3, MessageSquare, Users, Zap, ThumbsUp, ThumbsDown, Minus,
  AlertTriangle, Clock, Activity, Filter, Info, Download, Camera, Loader2,
  FolderOpen, ExternalLink
} from 'lucide-react';
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip as RechartsTooltip,
  ResponsiveContainer, Legend
} from 'recharts';
import type { DashboardStats, DailyStats, DifyApp, StatDistribution, FeedbackLabelStat, ModelDailyTokenSpeed } from '../types';

// ===== Time Range Presets =====
interface TimePreset {
  label: string;
  getRange: () => [number | undefined, number | undefined];
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
  { label: '全部时间', getRange: () => [undefined, undefined] },
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
  const [selectedPresetIdx, setSelectedPresetIdx] = useState<number>(3);
  const [customStart, setCustomStart] = useState<string>('');
  const [customEnd, setCustomEnd] = useState<string>('');
  const [useCustom, setUseCustom] = useState(false);
  const [exportingExcel, setExportingExcel] = useState(false);
  const [exportingScreenshot, setExportingScreenshot] = useState(false);
  const [exportMsg, setExportMsg] = useState<string | null>(null);
  const [exportMsgIsError, setExportMsgIsError] = useState(false);
  const [exportFilePath, setExportFilePath] = useState<string | null>(null);
  const dashboardRef = useRef<HTMLDivElement>(null);

  useEffect(() => { loadApps(); }, []);

  const loadApps = async () => {
    try {
      const result = await invoke<DifyApp[]>('get_local_apps');
      setApps(result);
    } catch (e) { console.error(e); }
  };

  /** Compute the current time range based on filter state */
  const getTimeRange = useCallback((): { startTime: number | undefined; endTime: number | undefined } => {
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
    return { startTime, endTime };
  }, [useCustom, customStart, customEnd, selectedPresetIdx]);

  const loadStats = useCallback(async () => {
    setLoading(true);
    try {
      const { startTime, endTime } = getTimeRange();
      const result = await invoke<DashboardStats>('get_dashboard_stats', {
        appId: selectedAppId || null,
        startTime,
        endTime,
      });
      setStats(result);
    } catch (e) { console.error(e); } finally { setLoading(false); }
  }, [selectedAppId, getTimeRange]);

  useEffect(() => { loadStats(); }, [loadStats]);

  const handleExportExcel = async () => {
    setExportingExcel(true);
    setExportMsg(null);
    try {
      const { startTime, endTime } = getTimeRange();
      const defaultName = `dashboard_export_${new Date().toISOString().slice(0, 19).replace(/[:-]/g, '')}.xlsx`;
      const filePath = await save({
        defaultPath: defaultName,
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
      });
      if (!filePath) { setExportingExcel(false); return; }
      const result = await invoke<string>('export_dashboard_excel', {
        appId: selectedAppId || null,
        startTime,
        endTime,
        savePath: filePath,
      });
      setExportFilePath(filePath);
      setExportMsg(result);
      setExportMsgIsError(false);
    } catch (e: any) {
      setExportMsg(`导出失败: ${e}`);
      setExportMsgIsError(true);
    } finally {
      setExportingExcel(false);
    }
  };

  const handleExportScreenshot = async () => {
    if (!dashboardRef.current) return;
    setExportingScreenshot(true);
    setExportMsg(null);
    try {
      const canvas = await html2canvas(dashboardRef.current, {
        backgroundColor: '#f9fafb',
        scale: 2,
        useCORS: true,
        logging: false,
        windowWidth: dashboardRef.current.scrollWidth,
        windowHeight: dashboardRef.current.scrollHeight,
      });
      const dataUrl = canvas.toDataURL('image/png');
      const base64Data = dataUrl.replace(/^data:image\/png;base64,/, '');
      const binaryStr = atob(base64Data);
      const bytes = new Uint8Array(binaryStr.length);
      for (let i = 0; i < binaryStr.length; i++) {
        bytes[i] = binaryStr.charCodeAt(i);
      }
      const filePath = await save({
        defaultPath: `dashboard_screenshot_${new Date().toISOString().slice(0, 19).replace(/[:-]/g, '')}.png`,
        filters: [{ name: 'PNG Image', extensions: ['png'] }],
      });
      if (filePath) {
        const { writeFile } = await import('@tauri-apps/plugin-fs');
        await writeFile(filePath, bytes);
        setExportFilePath(filePath);
        setExportMsg(`截图已保存到: ${filePath}`);
        setExportMsgIsError(false);
      }
    } catch (outerError: any) {
      // Fallback: download via browser <a> tag
      try {
        if (dashboardRef.current) {
          const canvas = await html2canvas(dashboardRef.current, {
            backgroundColor: '#f9fafb',
            scale: 2,
            useCORS: true,
            logging: false,
          });
          const link = document.createElement('a');
          link.download = `dashboard_screenshot_${Date.now()}.png`;
          link.href = canvas.toDataURL('image/png');
          link.click();
          setExportMsg('截图已下载');
          setExportMsgIsError(false);
        }
      } catch (innerError: any) {
        setExportMsg(`截图失败: ${innerError ?? outerError}`);
        setExportMsgIsError(true);
      }
    } finally {
      setExportingScreenshot(false);
    }
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
          <div className="flex items-center gap-2">
            <button
              onClick={loadStats}
              className="px-3 py-1.5 text-sm bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors"
            >
              刷新
            </button>
            <button
              onClick={handleExportExcel}
              disabled={exportingExcel || !stats}
              className="px-3 py-1.5 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1.5"
              title="导出 Excel 报表"
            >
              {exportingExcel ? <Loader2 size={14} className="animate-spin" /> : <Download size={14} />}
              <span className="hidden sm:inline">导出 Excel</span>
            </button>
            <button
              onClick={handleExportScreenshot}
              disabled={exportingScreenshot || !stats}
              className="px-3 py-1.5 text-sm bg-purple-600 text-white rounded-lg hover:bg-purple-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1.5"
              title="导出长截图"
            >
              {exportingScreenshot ? <Loader2 size={14} className="animate-spin" /> : <Camera size={14} />}
              <span className="hidden sm:inline">导出截图</span>
            </button>
          </div>
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

      {/* Export message */}
      {exportMsg && (
        <div className={`mb-4 flex items-center gap-2 px-4 py-3 rounded-lg text-sm ${
          exportMsgIsError
            ? 'bg-red-50 text-red-700 border border-red-200'
            : 'bg-green-50 text-green-700 border border-green-200'
        }`}>
          <span className="flex-1">{exportMsg}</span>
          {!exportMsgIsError && exportFilePath && (
            <div className="flex items-center gap-1.5 ml-2">
              <button
                onClick={async () => { try { await openPath(exportFilePath); } catch(e) { console.error(e); } }}
                className="px-2 py-1 text-xs bg-green-600 text-white rounded hover:bg-green-700 flex items-center gap-1"
                title="打开文件"
              >
                <ExternalLink size={12} /> 打开文件
              </button>
              <button
                onClick={async () => { try { await revealItemInDir(exportFilePath); } catch(e) { console.error(e); } }}
                className="px-2 py-1 text-xs bg-green-600 text-white rounded hover:bg-green-700 flex items-center gap-1"
                title="打开文件夹"
              >
                <FolderOpen size={12} /> 打开文件夹
              </button>
            </div>
          )}
          <button onClick={() => { setExportMsg(null); setExportMsgIsError(false); setExportFilePath(null); }} className="ml-1 text-current opacity-60 hover:opacity-100">✕</button>
        </div>
      )}

      {!stats ? (
        <div className="mt-10 text-center text-gray-500">暂无统计数据，请先同步数据</div>
      ) : (
        <div ref={dashboardRef}>
          {/* Basic Count Cards */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
            <StatCard icon={<Users size={18} className="text-blue-500" />} label="活跃用户数" value={stats.total_users}
              tooltip="与 AI 有效互动的唯一用户数（基于 from_end_user_id 去重统计）" />
            <StatCard icon={<MessageSquare size={18} className="text-green-500" />} label="全部会话数" value={stats.total_conversations}
              tooltip="会话（Conversation）总数，提示词编排和调试的消息不计入" />
            <StatCard icon={<Activity size={18} className="text-cyan-500" />} label="全部消息数" value={stats.total_messages}
              tooltip="AI 每天的互动总次数，每回答用户一个问题算一条 Message" />
            <StatCard icon={<Zap size={18} className="text-orange-500" />} label="应用数" value={stats.total_apps}
              tooltip="筛选范围内的应用数量" />
          </div>

          {/* Dify-aligned Key Metrics */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
            <SmallStat label="平均会话互动数" value={stats.avg_conversation_interactions.toFixed(1)}
              tooltip="每个会话用户的持续沟通次数，反映用户粘性（消息数 ÷ 会话数）" />
            <SmallStat label="用户满意度 (‰)" value={stats.satisfaction_rate.toFixed(1)}
              tooltip="每 1000 条消息的点赞数，反映用户对回答十分满意的比例" />
            <SmallStat label="好评率" value={formatPercent(stats.feedback_like_rate)}
              tooltip="好评率 = 赞数 / (赞数 + 踩数) × 100%" />
            <SmallStat label="异常率" value={formatPercent(stats.error_rate)}
              tooltip="异常率 = 异常消息数 / 消息总数 × 100%" />
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
                tooltip="总 Token = 消息有效 Token（优先 message_tokens，否则用 Prompt + Answer）+ workflow 额外消耗（去重后）" />
              <SmallStat label="日均 Token 消耗" value={formatStatNumber(Math.round(stats.daily_avg_tokens))}
                tooltip="日均消耗 = 总 Token 量 / 时间范围内的天数" />
            </div>
            {stats.total_tokens !== (stats.total_prompt_tokens + stats.total_answer_tokens) && (
              <p className="mb-3 text-xs text-amber-700 bg-amber-50 border border-amber-200 rounded px-3 py-2">
                💡 总 Token 量包含了 workflow 运行中未被消息直接覆盖的额外消耗，因此可能大于 Prompt + Answer 之和。
              </p>
            )}
            {stats.token_per_message_distribution && (
              <DistributionTable
                title="每条消息 Token 消耗分布"
                dist={stats.token_per_message_distribution}
                format={(v) => Math.round(v).toLocaleString()}
                tooltip="统计每条消息的有效 Token 消耗（优先 message_tokens，否则用 Prompt + Answer），样本为消耗大于 0 的消息"
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

          {/* Per-Model Token Speed Trend */}
          {stats.model_token_speed_daily && stats.model_token_speed_daily.length > 0 && (
            <Section title="各模型 Token 速度趋势">
              <ModelTokenSpeedChart data={stats.model_token_speed_daily} />
            </Section>
          )}

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
        </div>
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
    if (v >= 1000000) return `${(v / 1000000).toFixed(1)}M`;
    if (v >= 1000) return `${(v / 1000).toFixed(1)}K`;
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

      {/* Token trend — split by input/output */}
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日 Token 消耗（输入/输出）</h4>
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
            <Line type="monotone" dataKey="total_prompt_tokens" name="输入 Token" stroke="#f97316" strokeWidth={2} dot={false} />
            <Line type="monotone" dataKey="total_answer_tokens" name="输出 Token" stroke="#22c55e" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* Error trend */}
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日异常数</h4>
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
            <Line type="monotone" dataKey="errors" name="异常数" stroke="#ef4444" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* Feedback trend */}
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日反馈（赞/踩）</h4>
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
            <Line type="monotone" dataKey="likes" name="赞数" stroke="#22c55e" strokeWidth={2} dot={false} />
            <Line type="monotone" dataKey="dislikes" name="踩数" stroke="#ef4444" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* Response time trend (seconds scale) */}
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日响应时间</h4>
        <ResponsiveContainer width="100%" height={240}>
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis dataKey="date" tick={{ fontSize: 11 }} stroke="#9ca3af" />
            <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" unit="s" />
            <RechartsTooltip
              contentStyle={{ fontSize: 12, borderRadius: 8 }}
              formatter={(v: any, name: any) => [`${Number(v).toFixed(2)}s`, name]}
            />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            <Line type="monotone" dataKey="avg_elapsed_time" name="平均响应时间" stroke="#3b82f6" strokeWidth={2} dot={false} />
            <Line type="monotone" dataKey="avg_ttft" name="平均 TTFT" stroke="#8b5cf6" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* Token speed trend (tokens/s scale) */}
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日 Token 生成速度</h4>
        <ResponsiveContainer width="100%" height={240}>
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis dataKey="date" tick={{ fontSize: 11 }} stroke="#9ca3af" />
            <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" unit=" t/s" />
            <RechartsTooltip
              contentStyle={{ fontSize: 12, borderRadius: 8 }}
              formatter={(v: any, name: any) => [`${Number(v).toFixed(1)} t/s`, name]}
            />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            <Line type="monotone" dataKey="avg_token_speed" name="Token 速度" stroke="#f97316" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

// ── Per-Model Token Speed Trend Chart ──
const MODEL_COLORS = ['#3b82f6', '#ef4444', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1'];

function ModelTokenSpeedChart({ data }: { data: ModelDailyTokenSpeed[] }) {
  if (!data || data.length === 0) return null;

  // Get unique models and all dates
  const models = [...new Set(data.map(d => d.model))];
  const dates = [...new Set(data.map(d => d.date))].sort();

  // Build pivot: date -> { model -> speed }
  const pivot = dates.map(date => {
    const row: Record<string, any> = { date };
    for (const model of models) {
      const item = data.find(d => d.date === date && d.model === model);
      row[model] = item ? Number(item.avg_token_speed.toFixed(1)) : null;
    }
    return row;
  });

  return (
    <div>
      <h4 className="text-sm font-medium text-gray-700 mb-3">各模型 Token 生成速度趋势</h4>
      <ResponsiveContainer width="100%" height={300}>
        <LineChart data={pivot}>
          <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
          <XAxis dataKey="date" tick={{ fontSize: 11 }} stroke="#9ca3af" />
          <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" unit=" t/s" />
          <RechartsTooltip
            contentStyle={{ fontSize: 12, borderRadius: 8 }}
            formatter={(v: any, name: any) => [v !== null && v !== undefined ? `${Number(v).toFixed(1)} t/s` : '无数据', name]}
          />
          <Legend wrapperStyle={{ fontSize: 12 }} />
          {models.map((model, idx) => (
            <Line
              key={model}
              type="monotone"
              dataKey={model}
              name={model}
              stroke={MODEL_COLORS[idx % MODEL_COLORS.length]}
              strokeWidth={2}
              dot={false}
              connectNulls={false}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
