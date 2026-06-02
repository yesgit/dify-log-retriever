import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { PerformanceStats, DifyApp, AggregationStatus } from '../types';
import { Gauge, RefreshCw, CheckCircle, Clock, AlertTriangle } from 'lucide-react';

export function PerformancePage() {
  const [apps, setApps] = useState<DifyApp[]>([]);
  const [selectedApp, setSelectedApp] = useState<string>('');
  const [startDate, setStartDate] = useState<string>('');
  const [endDate, setEndDate] = useState<string>('');
  const [stats, setStats] = useState<PerformanceStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>('');
  const [aggStatus, setAggStatus] = useState<AggregationStatus | null>(null);
  const [rebuilding, setRebuilding] = useState(false);

  useEffect(() => {
    invoke<DifyApp[]>('get_local_apps').then(setApps).catch(console.error);
    loadAggStatus();
  }, []);

  const loadAggStatus = async () => {
    try {
      const status = await invoke<AggregationStatus>('get_aggregation_status');
      setAggStatus(status);
    } catch (e) {
      console.error(e);
    }
  };

  const loadData = async () => {
    setLoading(true);
    setError('');
    try {
      const startTs = startDate ? Math.floor(new Date(startDate).getTime() / 1000) : undefined;
      const endTs = endDate ? Math.floor(new Date(endDate + 'T23:59:59').getTime() / 1000) : undefined;
      const result = await invoke<PerformanceStats>('get_performance_stats', {
        appId: selectedApp || null,
        startTime: startTs || null,
        endTime: endTs || null,
      });
      setStats(result);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setLoading(false);
    }
  };

  const handleRebuild = async () => {
    setRebuilding(true);
    try {
      await invoke<string>('rebuild_dashboard_stats');
      await loadAggStatus();
      await loadData();
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setRebuilding(false);
    }
  };

  const formatNum = (n: number) => (n === null || n === undefined ? '-' : n.toFixed(2));
  const formatPct = (n: number) => (n === null || n === undefined ? '-' : n.toFixed(1) + '%');

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
          <Gauge size={28} />
          性能分析
        </h2>
        <div className="flex items-center gap-3">
          {aggStatus && (
            <div className="flex items-center gap-2 text-sm text-gray-500">
              {aggStatus.last_aggregated_at ? (
                <>
                  <CheckCircle size={14} className="text-green-500" />
                  <span>
                    已聚合 {aggStatus.total_days} 天数据 ·{' '}
                    {new Date(aggStatus.last_aggregated_at * 1000).toLocaleString('zh-CN')}
                  </span>
                </>
              ) : (
                <>
                  <AlertTriangle size={14} className="text-amber-500" />
                  <span>尚未聚合，请先构建聚合数据</span>
                </>
              )}
            </div>
          )}
          <button
            onClick={handleRebuild}
            disabled={rebuilding}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700 disabled:opacity-50"
          >
            <RefreshCw size={14} className={rebuilding ? 'animate-spin' : ''} />
            {rebuilding ? '构建中...' : '重新聚合'}
          </button>
        </div>
      </div>

      {/* Filters */}
      <div className="flex items-end gap-4 bg-white p-4 rounded-xl border border-gray-200">
        <div className="flex-1">
          <label className="block text-sm font-medium text-gray-700 mb-1">应用</label>
          <select
            value={selectedApp}
            onChange={(e) => setSelectedApp(e.target.value)}
            className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500"
          >
            <option value="">全部应用</option>
            {apps.map((app) => (
              <option key={app.id} value={app.id}>
                {app.name}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">开始日期</label>
          <input
            type="date"
            value={startDate}
            onChange={(e) => setStartDate(e.target.value)}
            className="border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500"
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">结束日期</label>
          <input
            type="date"
            value={endDate}
            onChange={(e) => setEndDate(e.target.value)}
            className="border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500"
          />
        </div>
        <button
          onClick={loadData}
          disabled={loading}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700 disabled:opacity-50 flex items-center gap-1.5"
        >
          {loading && <RefreshCw size={14} className="animate-spin" />}
          查询
        </button>
      </div>

      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm">{error}</div>
      )}

      {stats && (
        <>
          {/* Model Performance */}
          <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
            <div className="px-5 py-3 border-b border-gray-100 bg-gray-50">
              <h3 className="font-semibold text-gray-800">模型性能统计</h3>
            </div>
            {stats.model_performance.length > 0 ? (
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-gray-100 text-gray-500">
                      <th className="px-4 py-2.5 text-left font-medium">模型</th>
                      <th className="px-4 py-2.5 text-right font-medium">消息数</th>
                      <th className="px-4 py-2.5 text-right font-medium">总 Tokens</th>
                      <th className="px-4 py-2.5 text-right font-medium">平均耗时(s)</th>
                      <th className="px-4 py-2.5 text-right font-medium">平均 TTFT(s)</th>
                      <th className="px-4 py-2.5 text-right font-medium">速度(tokens/s)</th>
                      <th className="px-4 py-2.5 text-right font-medium">错误数</th>
                      <th className="px-4 py-2.5 text-right font-medium">错误率</th>
                    </tr>
                  </thead>
                  <tbody>
                    {stats.model_performance.map((m, i) => (
                      <tr key={i} className="border-b border-gray-50 hover:bg-gray-50">
                        <td className="px-4 py-2.5 font-mono text-xs">{m.model}</td>
                        <td className="px-4 py-2.5 text-right">{m.message_count.toLocaleString()}</td>
                        <td className="px-4 py-2.5 text-right">{m.total_tokens.toLocaleString()}</td>
                        <td className="px-4 py-2.5 text-right">{formatNum(m.avg_elapsed_time)}</td>
                        <td className="px-4 py-2.5 text-right">{formatNum(m.avg_ttft)}</td>
                        <td className="px-4 py-2.5 text-right">{formatNum(m.avg_token_speed)}</td>
                        <td className="px-4 py-2.5 text-right text-red-600">{m.error_count}</td>
                        <td className="px-4 py-2.5 text-right">{formatPct(m.error_rate)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="px-5 py-8 text-center text-gray-400">暂无模型性能数据</div>
            )}
          </div>

          {/* Model Token Speed Daily Trend */}
          {stats.model_token_speed_daily.length > 0 && (
            <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
              <div className="px-5 py-3 border-b border-gray-100 bg-gray-50">
                <h3 className="font-semibold text-gray-800">模型 Token 速度趋势</h3>
              </div>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-gray-100 text-gray-500">
                      <th className="px-4 py-2.5 text-left font-medium">模型</th>
                      <th className="px-4 py-2.5 text-left font-medium">日期</th>
                      <th className="px-4 py-2.5 text-right font-medium">平均速度(tokens/s)</th>
                      <th className="px-4 py-2.5 text-right font-medium">消息数</th>
                    </tr>
                  </thead>
                  <tbody>
                    {stats.model_token_speed_daily.map((d, i) => (
                      <tr key={i} className="border-b border-gray-50 hover:bg-gray-50">
                        <td className="px-4 py-2.5 font-mono text-xs">{d.model}</td>
                        <td className="px-4 py-2.5">{d.date}</td>
                        <td className="px-4 py-2.5 text-right">{formatNum(d.avg_token_speed)}</td>
                        <td className="px-4 py-2.5 text-right">{d.message_count.toLocaleString()}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Node Performance */}
          <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
            <div className="px-5 py-3 border-b border-gray-100 bg-gray-50">
              <h3 className="font-semibold text-gray-800">节点性能统计</h3>
            </div>
            {stats.node_performance.length > 0 ? (
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-gray-100 text-gray-500">
                      <th className="px-4 py-2.5 text-left font-medium">节点类型</th>
                      <th className="px-4 py-2.5 text-left font-medium">标题</th>
                      <th className="px-4 py-2.5 text-right font-medium">执行次数</th>
                      <th className="px-4 py-2.5 text-right font-medium">平均耗时(s)</th>
                      <th className="px-4 py-2.5 text-right font-medium">成功率</th>
                      <th className="px-4 py-2.5 text-right font-medium">错误数</th>
                    </tr>
                  </thead>
                  <tbody>
                    {stats.node_performance.map((n, i) => (
                      <tr key={i} className="border-b border-gray-50 hover:bg-gray-50">
                        <td className="px-4 py-2.5">
                          <span className="px-2 py-0.5 bg-blue-50 text-blue-700 rounded text-xs font-medium">{n.node_type}</span>
                        </td>
                        <td className="px-4 py-2.5">{n.title}</td>
                        <td className="px-4 py-2.5 text-right">{n.execution_count.toLocaleString()}</td>
                        <td className="px-4 py-2.5 text-right">{formatNum(n.avg_elapsed_time)}</td>
                        <td className="px-4 py-2.5 text-right">{formatPct(n.success_rate)}</td>
                        <td className="px-4 py-2.5 text-right text-red-600">{n.error_count}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="px-5 py-8 text-center text-gray-400">暂无节点性能数据</div>
            )}
          </div>

          {/* Node Daily Performance */}
          {stats.node_daily_performance.length > 0 && (
            <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
              <div className="px-5 py-3 border-b border-gray-100 bg-gray-50">
                <h3 className="font-semibold text-gray-800">节点每日性能趋势</h3>
              </div>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-gray-100 text-gray-500">
                      <th className="px-4 py-2.5 text-left font-medium">节点类型</th>
                      <th className="px-4 py-2.5 text-left font-medium">标题</th>
                      <th className="px-4 py-2.5 text-left font-medium">日期</th>
                      <th className="px-4 py-2.5 text-right font-medium">执行次数</th>
                      <th className="px-4 py-2.5 text-right font-medium">平均耗时(s)</th>
                      <th className="px-4 py-2.5 text-right font-medium">成功数</th>
                      <th className="px-4 py-2.5 text-right font-medium">错误数</th>
                    </tr>
                  </thead>
                  <tbody>
                    {stats.node_daily_performance.map((n, i) => (
                      <tr key={i} className="border-b border-gray-50 hover:bg-gray-50">
                        <td className="px-4 py-2.5">
                          <span className="px-2 py-0.5 bg-blue-50 text-blue-700 rounded text-xs font-medium">{n.node_type}</span>
                        </td>
                        <td className="px-4 py-2.5">{n.title}</td>
                        <td className="px-4 py-2.5">{n.date}</td>
                        <td className="px-4 py-2.5 text-right">{n.execution_count.toLocaleString()}</td>
                        <td className="px-4 py-2.5 text-right">{formatNum(n.avg_elapsed_time)}</td>
                        <td className="px-4 py-2.5 text-right text-green-600">{n.success_count}</td>
                        <td className="px-4 py-2.5 text-right text-red-600">{n.error_count}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </>
      )}

      {!stats && !loading && !error && (
        <div className="bg-white rounded-xl border border-gray-200 px-5 py-16 text-center text-gray-400">
          <Clock size={40} className="mx-auto mb-3 opacity-50" />
          <p className="text-lg">选择筛选条件后点击"查询"加载性能数据</p>
          <p className="text-sm mt-1">建议先点击"重新聚合"构建聚合数据以加速查询</p>
        </div>
      )}
    </div>
  );
}