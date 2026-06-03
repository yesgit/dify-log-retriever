import { useState, useEffect, useRef, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { revealItemInDir, openPath } from '@tauri-apps/plugin-opener';
import html2canvas from 'html2canvas';
import {
  Gauge, RefreshCw, Clock,
  Download, Camera, Loader2, ExternalLink, FolderOpen, ChevronDown, ChevronRight,
} from 'lucide-react';
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip as RechartsTooltip,
  ResponsiveContainer, Legend,
} from 'recharts';
import type { PerformanceStats, DifyApp, NodeDailyPerformance, ModelDailyTokenSpeed, AgentDailyPerformance } from '../types';

const CHART_COLORS = ['#3b82f6', '#ef4444', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1'];

export function PerformancePage() {
  const [apps, setApps] = useState<DifyApp[]>([]);
  const [selectedApp, setSelectedApp] = useState<string>('');
  const [selectedNodeType, setSelectedNodeType] = useState<string>('');
  const [startDate, setStartDate] = useState<string>('');
  const [endDate, setEndDate] = useState<string>('');
  const [stats, setStats] = useState<PerformanceStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>('');
  const [exportingExcel, setExportingExcel] = useState(false);
  const [exportingScreenshot, setExportingScreenshot] = useState(false);
  const [exportMsg, setExportMsg] = useState<string | null>(null);
  const [exportMsgIsError, setExportMsgIsError] = useState(false);
  const [exportFilePath, setExportFilePath] = useState<string | null>(null);
  const [showNodeTable, setShowNodeTable] = useState(false);
  const [showModelTable, setShowModelTable] = useState(false);
  const [selectedNodeKey, setSelectedNodeKey] = useState<string | null>(null);
  const performanceRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<DifyApp[]>('get_local_apps').then(setApps).catch(console.error);
  }, []);

  const nodeTypes = useMemo(() => {
    if (!stats) return [];
    const types = new Set<string>();
    stats.node_performance.forEach(n => types.add(n.node_type));
    stats.node_daily_performance.forEach(n => types.add(n.node_type));
    return [...types].sort();
  }, [stats]);

  const availableNodeKeys = useMemo(() => {
    if (!stats) return [];
    const keySet = new Set<string>();
    const labelMap = new Map<string, string>();
    const source = selectedNodeType
      ? stats.node_daily_performance.filter(n => n.node_type === selectedNodeType)
      : stats.node_daily_performance;
    for (const d of source) {
      const key = `${d.node_type}::${d.title}`;
      keySet.add(key);
      labelMap.set(key, `${d.node_type} - ${d.title}`);
    }
    return [...keySet].sort().map(key => ({ key, label: labelMap.get(key) || key }));
  }, [stats, selectedNodeType]);

  const filteredNodePerformance = useMemo(() => {
    if (!stats) return [];
    if (!selectedNodeType) return stats.node_performance;
    return stats.node_performance.filter(n => n.node_type === selectedNodeType);
  }, [stats, selectedNodeType]);

  const filteredNodeDaily = useMemo(() => {
    if (!stats) return [];
    let data = selectedNodeType
      ? stats.node_daily_performance.filter(n => n.node_type === selectedNodeType)
      : stats.node_daily_performance;
    if (selectedNodeKey) {
      data = data.filter(d => `${d.node_type}::${d.title}` === selectedNodeKey);
    }
    return data;
  }, [stats, selectedNodeType, selectedNodeKey]);

  useEffect(() => {
    setSelectedNodeKey(null);
  }, [stats, selectedNodeType]);

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
      setSelectedNodeType('');
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setLoading(false);
    }
  };

  const handleExportExcel = async () => {
    setExportingExcel(true);
    setExportMsg(null);
    try {
      const startTs = startDate ? Math.floor(new Date(startDate).getTime() / 1000) : undefined;
      const endTs = endDate ? Math.floor(new Date(endDate + 'T23:59:59').getTime() / 1000) : undefined;
      const defaultName = `performance_export_${new Date().toISOString().slice(0, 19).replace(/[:-]/g, '')}.xlsx`;
      const filePath = await save({
        defaultPath: defaultName,
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
      });
      if (!filePath) { setExportingExcel(false); return; }
      const result = await invoke<string>('export_performance_excel', {
        appId: selectedApp || null,
        startTime: startTs || null,
        endTime: endTs || null,
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
    if (!performanceRef.current) return;
    setExportingScreenshot(true);
    setExportMsg(null);
    const filePath = await save({
      defaultPath: `performance_screenshot_${new Date().toISOString().slice(0, 19).replace(/[:-]/g, '')}.png`,
      filters: [{ name: 'PNG Image', extensions: ['png'] }],
    });
    if (!filePath) { setExportingScreenshot(false); return; }
    try {
      const canvas = await html2canvas(performanceRef.current, {
        backgroundColor: '#f9fafb', scale: 2, useCORS: true, logging: false,
        windowWidth: performanceRef.current.scrollWidth, windowHeight: performanceRef.current.scrollHeight,
      });
      const dataUrl = canvas.toDataURL('image/png');
      const base64Data = dataUrl.replace(/^data:image\/png;base64,/, '');
      const binaryStr = atob(base64Data);
      const bytes = new Uint8Array(binaryStr.length);
      for (let i = 0; i < binaryStr.length; i++) { bytes[i] = binaryStr.charCodeAt(i); }
      const { writeFile } = await import('@tauri-apps/plugin-fs');
      await writeFile(filePath, bytes);
      setExportFilePath(filePath);
      setExportMsg(`截图已保存到: ${filePath}`);
      setExportMsgIsError(false);
    } catch (e: any) {
      try {
        if (performanceRef.current) {
          const canvas = await html2canvas(performanceRef.current, { backgroundColor: '#f9fafb', scale: 2, useCORS: true, logging: false });
          const link = document.createElement('a');
          link.download = `performance_screenshot_${Date.now()}.png`;
          link.href = canvas.toDataURL('image/png');
          link.click();
          setExportMsg('截图已下载（浏览器下载）');
          setExportMsgIsError(false);
        }
      } catch (innerError: any) {
        setExportMsg(`截图失败: ${innerError ?? e}`);
        setExportMsgIsError(true);
      }
    } finally {
      setExportingScreenshot(false);
    }
  };

  const formatNum = (n: number) => (n === null || n === undefined ? '-' : n.toFixed(2));
  const formatPct = (n: number) => (n === null || n === undefined ? '-' : n.toFixed(1) + '%');

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
          <Gauge size={28} /> 性能分析
        </h2>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap items-end gap-4 bg-white p-4 rounded-xl border border-gray-200">
        <div className="min-w-[160px]">
          <label className="block text-sm font-medium text-gray-700 mb-1">应用</label>
          <select value={selectedApp} onChange={(e) => setSelectedApp(e.target.value)}
            className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500">
            <option value="">全部应用</option>
            {apps.map((app) => (<option key={app.id} value={app.id}>{app.name}</option>))}
          </select>
        </div>
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">开始日期</label>
          <input type="date" value={startDate} onChange={(e) => setStartDate(e.target.value)}
            className="border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500" />
        </div>
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">结束日期</label>
          <input type="date" value={endDate} onChange={(e) => setEndDate(e.target.value)}
            className="border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500" />
        </div>
        <button onClick={loadData} disabled={loading}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700 disabled:opacity-50 flex items-center gap-1.5">
          {loading && <RefreshCw size={14} className="animate-spin" />} 查询
        </button>
      </div>

      {/* Export buttons */}
      {stats && (
        <div className="flex items-center gap-2">
          <button onClick={handleExportExcel} disabled={exportingExcel}
            className="px-3 py-1.5 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors disabled:opacity-50 flex items-center gap-1.5"
            title="导出 Excel 报表">
            {exportingExcel ? <Loader2 size={14} className="animate-spin" /> : <Download size={14} />} 导出 Excel
          </button>
          <button onClick={handleExportScreenshot} disabled={exportingScreenshot}
            className="px-3 py-1.5 text-sm bg-purple-600 text-white rounded-lg hover:bg-purple-700 transition-colors disabled:opacity-50 flex items-center gap-1.5"
            title="导出截图">
            {exportingScreenshot ? <Loader2 size={14} className="animate-spin" /> : <Camera size={14} />} 导出截图
          </button>
        </div>
      )}

      {/* Export message */}
      {exportMsg && (
        <div className={`flex items-center gap-2 px-4 py-3 rounded-lg text-sm ${
          exportMsgIsError ? 'bg-red-50 text-red-700 border border-red-200' : 'bg-green-50 text-green-700 border border-green-200'}`}>
          <span className="flex-1">{exportMsg}</span>
          {!exportMsgIsError && exportFilePath && (
            <div className="flex items-center gap-1.5 ml-2">
              <button onClick={async () => { try { await openPath(exportFilePath); } catch(e) { console.error(e); } }}
                className="px-2 py-1 text-xs bg-green-600 text-white rounded hover:bg-green-700 flex items-center gap-1">
                <ExternalLink size={12} /> 打开文件
              </button>
              <button onClick={async () => { try { await revealItemInDir(exportFilePath); } catch(e) { console.error(e); } }}
                className="px-2 py-1 text-xs bg-green-600 text-white rounded hover:bg-green-700 flex items-center gap-1">
                <FolderOpen size={12} /> 打开文件夹
              </button>
            </div>
          )}
          <button onClick={() => { setExportMsg(null); setExportMsgIsError(false); setExportFilePath(null); }}
            className="ml-1 text-current opacity-60 hover:opacity-100">✕</button>
        </div>
      )}

      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm">{error}</div>
      )}

      {stats && (
        <div ref={performanceRef}>
          {/* Screenshot Header */}
          <div className="bg-white rounded-xl border border-gray-200 p-4 mb-6 flex items-center justify-between">
            <div>
              <h2 className="text-lg font-bold text-gray-900 flex items-center gap-2">
                <Gauge size={20} />
                性能分析{selectedApp ? ` - ${apps.find(a => a.id === selectedApp)?.name || selectedApp}` : ' - 全部应用'}
              </h2>
            </div>
            <div className="text-sm text-gray-500">
              {startDate || endDate ? (<>{startDate || '起始'} ~ {endDate || '至今'}</>) : '全部时间'}
            </div>
          </div>

          {/* Node Type Filter */}
          {nodeTypes.length > 1 && (
            <div className="mb-4 bg-white p-3 rounded-xl border border-gray-200 flex items-center gap-3">
              <span className="text-sm font-medium text-gray-700">节点类型筛选:</span>
              <div className="flex flex-wrap gap-1.5">
                <button onClick={() => setSelectedNodeType('')}
                  className={`px-2.5 py-1 text-xs rounded-lg transition-colors ${!selectedNodeType ? 'bg-blue-500 text-white' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}`}>
                  全部
                </button>
                {nodeTypes.map((type) => (
                  <button key={type} onClick={() => setSelectedNodeType(type)}
                    className={`px-2.5 py-1 text-xs rounded-lg transition-colors ${selectedNodeType === type ? 'bg-blue-500 text-white' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}`}>
                    {type}
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* Model Performance */}
          <div className="bg-white rounded-xl border border-gray-200 overflow-hidden mb-6">
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

          {/* Model Token Speed Trend Chart */}
          {stats.model_token_speed_daily.length > 0 && (
            <div className="bg-white rounded-xl border border-gray-200 overflow-hidden mb-6">
              <div className="px-5 py-3 border-b border-gray-100 bg-gray-50 flex items-center justify-between">
                <h3 className="font-semibold text-gray-800">模型 Token 速度趋势</h3>
                <button onClick={() => setShowModelTable(!showModelTable)}
                  className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700">
                  {showModelTable ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                  {showModelTable ? '隐藏表格' : '显示表格'}
                </button>
              </div>
              <div className="p-4">
                <ModelTokenSpeedChart data={stats.model_token_speed_daily} />
              </div>
              {showModelTable && (
                <div className="overflow-x-auto border-t border-gray-100">
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
              )}
            </div>
          )}

          {/* Node Performance */}
          <div className="bg-white rounded-xl border border-gray-200 overflow-hidden mb-6">
            <div className="px-5 py-3 border-b border-gray-100 bg-gray-50">
              <h3 className="font-semibold text-gray-800">
                节点性能统计
                {selectedNodeType && <span className="ml-2 text-sm font-normal text-blue-600">筛选: {selectedNodeType}</span>}
              </h3>
            </div>
            {filteredNodePerformance.length > 0 ? (
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
                    {filteredNodePerformance.map((n, i) => (
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

          {/* Node Daily Performance Trend Charts */}
          {availableNodeKeys.length > 0 && (
            <div className="bg-white rounded-xl border border-gray-200 overflow-hidden mb-6">
              <div className="px-5 py-3 border-b border-gray-100 bg-gray-50 flex items-center justify-between">
                <h3 className="font-semibold text-gray-800">
                  节点每日性能趋势
                  {selectedNodeType && <span className="ml-2 text-sm font-normal text-blue-600">筛选: {selectedNodeType}</span>}
                </h3>
                <button onClick={() => setShowNodeTable(!showNodeTable)}
                  className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700">
                  {showNodeTable ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                  {showNodeTable ? '隐藏表格' : '显示表格'}
                </button>
              </div>
              <div className="px-5 py-3 border-b border-gray-100 bg-gray-50/50">
                <div className="flex items-center gap-3">
                  <span className="text-sm font-medium text-gray-600 whitespace-nowrap">趋势图节点:</span>
                  <select value={selectedNodeKey || ''} onChange={(e) => setSelectedNodeKey(e.target.value || null)}
                    className="flex-1 min-w-[200px] border border-gray-300 rounded-lg px-3 py-2 text-sm bg-white focus:ring-2 focus:ring-blue-500 focus:border-blue-400">
                    <option value="">请选择节点</option>
                    {availableNodeKeys.map(({ key, label }) => (<option key={key} value={key}>{label}</option>))}
                  </select>
                </div>
              </div>
              {selectedNodeKey && filteredNodeDaily.length > 0 ? (
                <div className="p-4"><NodeDailyCharts data={filteredNodeDaily} /></div>
              ) : (
                <div className="px-5 py-8 text-center text-gray-400">
                  {selectedNodeKey ? '所选节点无每日趋势数据' : '请在上方选择一个节点以显示趋势图'}
                </div>
              )}
              {showNodeTable && (
                <div className="overflow-x-auto border-t border-gray-100">
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
                      {filteredNodeDaily.map((n, i) => (
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
              )}
            </div>
          )}
        </div>
      )}

      {/* Agent (per-app) Performance Section */}
      {stats && stats.agent_performance && stats.agent_performance.length > 0 && (
        <div className="bg-white rounded-xl border border-gray-200 p-5">
          <h3 className="text-base font-semibold text-gray-700 mb-4 flex items-center gap-2">
            <Gauge size={16} className="text-purple-500" /> Agent 级别性能统计
          </h3>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200 text-left text-xs text-gray-500">
                  <th className="pb-2 pr-3 font-medium">应用名称</th>
                  <th className="pb-2 pr-3 font-medium text-right">消息数</th>
                  <th className="pb-2 pr-3 font-medium text-right">Token 速度</th>
                  <th className="pb-2 pr-3 font-medium text-right">TTFT</th>
                  <th className="pb-2 pr-3 font-medium text-right">平均耗时</th>
                  <th className="pb-2 font-medium text-right">输出 Tokens</th>
                </tr>
              </thead>
              <tbody>
                {stats.agent_performance.map((a) => (
                  <tr key={a.app_id} className="border-b border-gray-100 hover:bg-gray-50">
                    <td className="py-2 pr-3 font-medium text-gray-700 max-w-[200px] truncate" title={a.app_name}>{a.app_name}</td>
                    <td className="py-2 pr-3 text-right text-gray-600">{a.message_count.toLocaleString()}</td>
                    <td className="py-2 pr-3 text-right">
                      <span className={`font-mono ${a.avg_token_speed > 0 ? 'text-green-600' : 'text-gray-300'}`}>
                        {a.avg_token_speed > 0 ? a.avg_token_speed.toFixed(1) : '-'}
                      </span>
                      {a.avg_token_speed > 0 && <span className="text-gray-400 text-xs ml-1">t/s</span>}
                    </td>
                    <td className="py-2 pr-3 text-right">
                      <span className={`font-mono ${a.avg_ttft > 0 ? 'text-blue-600' : 'text-gray-300'}`}>
                        {a.avg_ttft > 0 ? a.avg_ttft.toFixed(2) : '-'}
                      </span>
                      {a.avg_ttft > 0 && <span className="text-gray-400 text-xs ml-1">s</span>}
                    </td>
                    <td className="py-2 pr-3 text-right">
                      <span className={`font-mono ${a.avg_elapsed_time > 0 ? 'text-orange-600' : 'text-gray-300'}`}>
                        {a.avg_elapsed_time > 0 ? a.avg_elapsed_time.toFixed(2) : '-'}
                      </span>
                      {a.avg_elapsed_time > 0 && <span className="text-gray-400 text-xs ml-1">s</span>}
                    </td>
                    <td className="py-2 text-right font-mono text-gray-600">{a.total_answer_tokens.toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {stats.agent_daily_performance && stats.agent_daily_performance.length > 0 && (
            <AgentDailyCharts data={stats.agent_daily_performance} />
          )}
        </div>
      )}

      {!stats && !loading && !error && (
        <div className="bg-white rounded-xl border border-gray-200 px-5 py-16 text-center text-gray-400">
          <Clock size={40} className="mx-auto mb-3 opacity-50" />
          <p className="text-lg">选择筛选条件后点击"查询"加载性能数据</p>
        </div>
      )}
    </div>
  );
}

// ===== Shared XAxis props for 45° angled date labels =====
const angledXAxisProps = {
  dataKey: 'date',
  tick: { fontSize: 10, angle: -45, textAnchor: 'end' as const },
  stroke: '#9ca3af',
  tickMargin: 8,
  height: 60,
  interval: 0 as const,
};

// ===== Model Token Speed Trend Chart =====
function ModelTokenSpeedChart({ data }: { data: ModelDailyTokenSpeed[] }) {
  const { models, pivot } = useMemo(() => {
    const models = [...new Set(data.map(d => d.model))];
    const dates = [...new Set(data.map(d => d.date))].sort();
    const dataMap = new Map<string, ModelDailyTokenSpeed>();
    // Sanitize model name to avoid recharts dot-path interpretation (e.g. "gpt-4.1" → "gpt-4_1")
    const safeKey = (m: string) => m.replace(/\./g, '_');
    for (const d of data) { dataMap.set(`${d.date}::${d.model}`, d); }
    const pivot = dates.map(date => {
      const row: Record<string, any> = { date };
      for (const model of models) {
        const item = dataMap.get(`${date}::${model}`);
        row[safeKey(model)] = item ? Number(item.avg_token_speed.toFixed(1)) : null;
      }
      return row;
    });
    return { models, pivot };
  }, [data]);

  const safeKey = (m: string) => m.replace(/\./g, '_');

  return (
    <div>
      <h4 className="text-sm font-medium text-gray-700 mb-3">各模型 Token 生成速度趋势</h4>
      <ResponsiveContainer width="100%" height={320}>
        <LineChart data={pivot} margin={{ left: 10, right: 10, bottom: 30 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
          <XAxis {...angledXAxisProps} />
          <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" unit=" t/s" />
          <RechartsTooltip contentStyle={{ fontSize: 12, borderRadius: 8 }}
            formatter={(v: any, name: any) => [v !== null && v !== undefined ? `${Number(v).toFixed(1)} t/s` : '无数据', name]} />
          <Legend wrapperStyle={{ fontSize: 12 }} />
          {models.map((model, idx) => (
            <Line key={model} type="linear" dataKey={safeKey(model)} name={model}
              stroke={CHART_COLORS[idx % CHART_COLORS.length]} strokeWidth={2} dot={false} connectNulls={false} />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

// ===== Node Daily Performance Charts =====
function NodeDailyCharts({ data }: { data: NodeDailyPerformance[] }) {
  const seriesInfo = useMemo(() => {
    const keySet = new Set<string>();
    const labelMap = new Map<string, string>();
    for (const d of data) {
      const key = `${d.node_type}::${d.title}`;
      keySet.add(key);
      labelMap.set(key, d.title || d.node_type);
    }
    return { keys: [...keySet], labelMap };
  }, [data]);

  const { keys, labelMap } = seriesInfo;

  const { timePivot, countPivot, successPivot, successErrorKeys } = useMemo(() => {
    const dates = [...new Set(data.map(d => d.date))].sort();
    const dataMap = new Map<string, NodeDailyPerformance>();
    for (const d of data) { dataMap.set(`${d.date}::${d.node_type}::${d.title}`, d); }

    const timePivot = dates.map(date => {
      const row: Record<string, any> = { date };
      for (const key of keys) {
        const item = dataMap.get(`${date}::${key}`);
        row[key] = item ? Number(item.avg_elapsed_time.toFixed(2)) : null;
      }
      return row;
    });

    const countPivot = dates.map(date => {
      const row: Record<string, any> = { date };
      for (const key of keys) {
        const item = dataMap.get(`${date}::${key}`);
        row[key] = item ? item.execution_count : null;
      }
      return row;
    });

    const successPivot = dates.map(date => {
      const row: Record<string, any> = { date };
      for (const key of keys) {
        const item = dataMap.get(`${date}::${key}`);
        row[`${key}_success`] = item ? item.success_count : null;
        row[`${key}_error`] = item ? item.error_count : null;
      }
      return row;
    });

    const successErrorKeys = keys.flatMap(key => {
      const label = labelMap.get(key) || key;
      return [
        { key: `${key}_success`, name: `${label} 成功`, color: '#10b981' },
        { key: `${key}_error`, name: `${label} 错误`, color: '#ef4444' },
      ];
    });

    return { timePivot, countPivot, successPivot, successErrorKeys };
  }, [data, keys, labelMap]);

  return (
    <div className="space-y-6">
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日平均耗时趋势 (秒)</h4>
        <ResponsiveContainer width="100%" height={300}>
          <LineChart data={timePivot} margin={{ left: 10, right: 10, bottom: 30 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis {...angledXAxisProps} />
            <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" unit="s" />
            <RechartsTooltip contentStyle={{ fontSize: 12, borderRadius: 8 }}
              formatter={(v: any, name: any) => [v !== null && v !== undefined ? `${Number(v).toFixed(2)}s` : '无数据', name]} />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            {keys.map((key, idx) => (
              <Line key={key} type="linear" dataKey={key} name={labelMap.get(key) || key}
                stroke={CHART_COLORS[idx % CHART_COLORS.length]} strokeWidth={2} dot={false} connectNulls={false} />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </div>
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日执行次数趋势</h4>
        <ResponsiveContainer width="100%" height={300}>
          <LineChart data={countPivot} margin={{ left: 10, right: 10, bottom: 30 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis {...angledXAxisProps} />
            <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" />
            <RechartsTooltip contentStyle={{ fontSize: 12, borderRadius: 8 }}
              formatter={(v: any, name: any) => [v !== null && v !== undefined ? Number(v).toLocaleString() : '无数据', name]} />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            {keys.map((key, idx) => (
              <Line key={key} type="linear" dataKey={key} name={labelMap.get(key) || key}
                stroke={CHART_COLORS[idx % CHART_COLORS.length]} strokeWidth={2} dot={false} connectNulls={false} />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </div>
      <div>
        <h4 className="text-sm font-medium text-gray-700 mb-3">每日成功/错误数趋势</h4>
        <ResponsiveContainer width="100%" height={300}>
          <LineChart data={successPivot} margin={{ left: 10, right: 10, bottom: 30 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis {...angledXAxisProps} />
            <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" />
            <RechartsTooltip contentStyle={{ fontSize: 12, borderRadius: 8 }}
              formatter={(v: any, name: any) => [v !== null && v !== undefined ? Number(v).toLocaleString() : '无数据', name]} />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            {successErrorKeys.map((se) => (
              <Line key={se.key} type="linear" dataKey={se.key} name={se.name}
                stroke={se.color} strokeWidth={1.5}
                strokeDasharray={se.key.endsWith('_error') ? '5 3' : undefined}
                dot={false} connectNulls={false} />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

// ===== Agent Daily Performance Charts =====
function AgentDailyCharts({ data }: { data: AgentDailyPerformance[] }) {
  const { apps, ttftPivot, speedPivot } = useMemo(() => {
    const appIds: string[] = [];
    const appNameMap = new Map<string, string>();
    for (const d of data) {
      if (!appNameMap.has(d.app_id)) {
        appIds.push(d.app_id);
        appNameMap.set(d.app_id, d.app_name);
      }
    }
    const dates = [...new Set(data.map(d => d.date))].sort();
    const dataMap = new Map<string, AgentDailyPerformance>();
    for (const d of data) { dataMap.set(`${d.date}::${d.app_id}`, d); }

    const ttftPivot = dates.map(date => {
      const row: Record<string, any> = { date };
      for (const appId of appIds) {
        const item = dataMap.get(`${date}::${appId}`);
        row[`${appId}_ttft`] = item && item.avg_ttft > 0 ? Number(item.avg_ttft.toFixed(3)) : null;
      }
      return row;
    });

    const speedPivot = dates.map(date => {
      const row: Record<string, any> = { date };
      for (const appId of appIds) {
        const item = dataMap.get(`${date}::${appId}`);
        row[`${appId}_speed`] = item && item.avg_token_speed > 0 ? Number(item.avg_token_speed.toFixed(1)) : null;
      }
      return row;
    });

    return { apps: appIds.map(id => ({ id, name: appNameMap.get(id) || id })), ttftPivot, speedPivot };
  }, [data]);

  return (
    <div className="mt-6 space-y-6">
      <div>
        <h4 className="text-sm font-medium text-gray-600 mb-3">Agent 每日 TTFT 趋势</h4>
        <ResponsiveContainer width="100%" height={280}>
          <LineChart data={ttftPivot} margin={{ left: 10, right: 10, bottom: 30 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis {...angledXAxisProps} />
            <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" unit="s" />
            <RechartsTooltip contentStyle={{ fontSize: 12, borderRadius: 8 }}
              formatter={(v: any, name: any) => [v !== null && v !== undefined ? `${Number(v).toFixed(3)}s` : '无数据', name]} />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            {apps.map((app, idx) => (
              <Line key={app.id} type="linear" dataKey={`${app.id}_ttft`} name={app.name}
                stroke={CHART_COLORS[idx % CHART_COLORS.length]} strokeWidth={1.5} dot={false} connectNulls={false} />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </div>
      <div>
        <h4 className="text-sm font-medium text-gray-600 mb-3">Agent 每日 Token 速度趋势</h4>
        <ResponsiveContainer width="100%" height={280}>
          <LineChart data={speedPivot} margin={{ left: 10, right: 10, bottom: 30 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis {...angledXAxisProps} />
            <YAxis tick={{ fontSize: 11 }} stroke="#9ca3af" unit=" t/s" />
            <RechartsTooltip contentStyle={{ fontSize: 12, borderRadius: 8 }}
              formatter={(v: any, name: any) => [v !== null && v !== undefined ? `${Number(v).toFixed(1)} t/s` : '无数据', name]} />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            {apps.map((app, idx) => (
              <Line key={app.id} type="linear" dataKey={`${app.id}_speed`} name={app.name}
                stroke={CHART_COLORS[idx % CHART_COLORS.length]} strokeWidth={1.5} dot={false} connectNulls={false} />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}