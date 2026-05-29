import { useState, useEffect } from 'react';
import { Download, FileJson, FileSpreadsheet, FileText, Loader2, CheckCircle } from 'lucide-react';

export function ExportPage() {
  const [apps, setApps] = useState<{ id: string; name: string }[]>([]);
  const [selectedApp, setSelectedApp] = useState<string>('');
  const [format, setFormat] = useState<'json' | 'csv' | 'jsonl'>('json');
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');
  const [keyword, setKeyword] = useState('');
  const [includeMetadata, setIncludeMetadata] = useState(true);
  const [includeAgentThoughts, setIncludeAgentThoughts] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [exportResult, setExportResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadApps();
  }, []);

  const loadApps = async () => {
    try {
      const result = await (window as any).__TAURI__.invoke('get_local_apps');
      setApps((result || []).map((a: any) => ({ id: a.id, name: a.name })));
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const handleExport = async () => {
    setExporting(true);
    setExportResult(null);
    try {
      const result = await (window as any).__TAURI__.invoke('export_data', {
        format,
        appId: selectedApp || null,
        startDate: startDate || null,
        endDate: endDate || null,
        keyword: keyword || null,
        includeMetadata,
        includeAgentThoughts,
      });
      setExportResult(result);
    } catch (e: any) {
      setExportResult(`导出失败: ${e}`);
    } finally {
      setExporting(false);
    }
  };

  const formats = [
    { value: 'json' as const, label: 'JSON', desc: '完整结构化数据', icon: <FileJson size={20} /> },
    { value: 'csv' as const, label: 'CSV', desc: '扁平化表格，适合 Excel', icon: <FileSpreadsheet size={20} /> },
    { value: 'jsonl' as const, label: 'JSONL', desc: '每行一条问答，适合评测框架', icon: <FileText size={20} /> },
  ];

  return (
    <div className="max-w-3xl">
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
          <Download size={24} />
          数据导出
        </h2>
        <p className="text-gray-500 mt-1">将对话数据导出为不同格式，用于分析和评测</p>
      </div>

      <div className="space-y-6">
        {/* Format Selection */}
        <div className="bg-white rounded-xl border border-gray-200 p-5">
          <h3 className="font-semibold text-gray-900 mb-3">选择导出格式</h3>
          <div className="grid grid-cols-3 gap-3">
            {formats.map((f) => (
              <button
                key={f.value}
                onClick={() => setFormat(f.value)}
                className={`p-4 rounded-lg border-2 text-left transition-colors ${
                  format === f.value
                    ? 'border-blue-500 bg-blue-50'
                    : 'border-gray-200 hover:border-gray-300'
                }`}
              >
                <div className={`mb-2 ${format === f.value ? 'text-blue-600' : 'text-gray-400'}`}>
                  {f.icon}
                </div>
                <p className="font-medium text-sm text-gray-900">{f.label}</p>
                <p className="text-xs text-gray-500 mt-0.5">{f.desc}</p>
              </button>
            ))}
          </div>
        </div>

        {/* Filters */}
        <div className="bg-white rounded-xl border border-gray-200 p-5">
          <h3 className="font-semibold text-gray-900 mb-3">筛选条件</h3>
          <div className="space-y-4">
            {/* App Selection */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">应用</label>
              <select
                value={selectedApp}
                onChange={(e) => setSelectedApp(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="">全部应用</option>
                {apps.map((app) => (
                  <option key={app.id} value={app.id}>{app.name}</option>
                ))}
              </select>
            </div>

            {/* Date Range */}
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">开始日期</label>
                <input
                  type="date"
                  value={startDate}
                  onChange={(e) => setStartDate(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">结束日期</label>
                <input
                  type="date"
                  value={endDate}
                  onChange={(e) => setEndDate(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            </div>

            {/* Keyword */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">关键词过滤</label>
              <input
                type="text"
                value={keyword}
                onChange={(e) => setKeyword(e.target.value)}
                placeholder="输入关键词筛选对话内容..."
                className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>

            {/* Options */}
            <div className="flex gap-6">
              <label className="flex items-center gap-2 text-sm text-gray-700 cursor-pointer">
                <input
                  type="checkbox"
                  checked={includeMetadata}
                  onChange={(e) => setIncludeMetadata(e.target.checked)}
                  className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                />
                包含元数据 (tokens, 耗时等)
              </label>
              <label className="flex items-center gap-2 text-sm text-gray-700 cursor-pointer">
                <input
                  type="checkbox"
                  checked={includeAgentThoughts}
                  onChange={(e) => setIncludeAgentThoughts(e.target.checked)}
                  className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                />
                包含 Agent 思维链
              </label>
            </div>
          </div>
        </div>

        {/* Export Result */}
        {exportResult && (
          <div className={`flex items-center gap-2 px-4 py-3 rounded-lg text-sm ${
            exportResult.startsWith('导出失败')
              ? 'bg-red-50 text-red-700 border border-red-200'
              : 'bg-green-50 text-green-700 border border-green-200'
          }`}>
            {exportResult.startsWith('导出失败') ? null : <CheckCircle size={16} />}
            {exportResult}
          </div>
        )}

        {/* Export Button */}
        <button
          onClick={handleExport}
          disabled={exporting || loading}
          className="w-full py-3 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        >
          {exporting ? (
            <>
              <Loader2 size={18} className="animate-spin" />
              正在导出...
            </>
          ) : (
            <>
              <Download size={18} />
              导出数据
            </>
          )}
        </button>
      </div>
    </div>
  );
}