import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Download, FileJson, FileSpreadsheet, FileText, Loader2, CheckCircle, Cpu, MessageSquare, Brain } from 'lucide-react';
import type { DifyApp } from '../types';

interface NodeTypeSummary {
  node_type: string;
  node_id: string;
  node_title: string;
  count: number;
}

export function ExportPage() {
  const [activeTab, setActiveTab] = useState<'conversation' | 'node-eval'>('conversation');

  // Shared state
  const [apps, setApps] = useState<{ id: string; name: string }[]>([]);
  const [loading, setLoading] = useState(true);

  // Conversation export state
  const [selectedApp, setSelectedApp] = useState<string>('');
  const [format, setFormat] = useState<'json' | 'csv' | 'jsonl'>('json');
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');
  const [keyword, setKeyword] = useState('');
  const [includeMetadata, setIncludeMetadata] = useState(true);
  const [includeAgentThoughts, setIncludeAgentThoughts] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [exportResult, setExportResult] = useState<string | null>(null);

  // Node eval export state
  const [nodeAppId, setNodeAppId] = useState<string>('');
  const [nodeTypes, setNodeTypes] = useState<NodeTypeSummary[]>([]);
  const [selectedNodeType, setSelectedNodeType] = useState<string>('');
  const [selectedNodeId, setSelectedNodeId] = useState<string>('');
  const [nodeEvalFormat, setNodeEvalFormat] = useState<string>('openai-eval');
  const [nodeStartDate, setNodeStartDate] = useState('');
  const [nodeEndDate, setNodeEndDate] = useState('');
  const [nodeExporting, setNodeExporting] = useState(false);
  const [nodeExportResult, setNodeExportResult] = useState<string | null>(null);
  const [nodeTypesLoading, setNodeTypesLoading] = useState(false);

  useEffect(() => {
    loadApps();
  }, []);

  const loadApps = async () => {
    try {
      const result = await invoke<DifyApp[]>('get_local_apps');
      setApps((result || []).map((a) => ({ id: a.id, name: a.name })));
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const loadNodeTypes = async (appId: string) => {
    if (!appId) {
      setNodeTypes([]);
      setSelectedNodeType('');
      setSelectedNodeId('');
      return;
    }
    setNodeTypesLoading(true);
    try {
      const result = await invoke<NodeTypeSummary[]>('get_app_node_types', { appId });
      setNodeTypes(result || []);
      setSelectedNodeType('');
      setSelectedNodeId('');
    } catch (e) {
      console.error(e);
      setNodeTypes([]);
    } finally {
      setNodeTypesLoading(false);
    }
  };

  const handleExport = async () => {
    setExporting(true);
    setExportResult(null);
    try {
      const result = await invoke<string>('export_data', {
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

  const handleNodeEvalExport = async () => {
    if (!nodeAppId) {
      setNodeExportResult('导出失败: 请先选择一个应用');
      return;
    }
    setNodeExporting(true);
    setNodeExportResult(null);
    try {
      const result = await invoke<string>('export_node_eval_data', {
        format: nodeEvalFormat,
        appId: nodeAppId,
        nodeType: selectedNodeType || null,
        nodeId: selectedNodeId || null,
        startDate: nodeStartDate || null,
        endDate: nodeEndDate || null,
      });
      setNodeExportResult(result);
    } catch (e: any) {
      setNodeExportResult(`导出失败: ${e}`);
    } finally {
      setNodeExporting(false);
    }
  };

  const formats = [
    { value: 'json' as const, label: 'JSON', desc: '完整结构化数据', icon: <FileJson size={20} /> },
    { value: 'csv' as const, label: 'CSV', desc: '扁平化表格，适合 Excel', icon: <FileSpreadsheet size={20} /> },
    { value: 'jsonl' as const, label: 'JSONL', desc: '每行一条问答，适合评测框架', icon: <FileText size={20} /> },
  ];

  const nodeEvalFormats = [
    { value: 'openai-eval', label: 'OpenAI Evals', desc: 'messages + ideal 格式', icon: <Brain size={20} /> },
    { value: 'openai-finetune', label: 'OpenAI Fine-tune', desc: '含 assistant 回复的训练格式', icon: <Cpu size={20} /> },
    { value: 'alpaca', label: 'AlpacaEval', desc: 'instruction + output 格式', icon: <FileJson size={20} /> },
    { value: 'qa', label: 'QA 评测', desc: 'query + expected_output 格式', icon: <MessageSquare size={20} /> },
    { value: 'raw', label: '完整原始数据', desc: '包含所有节点执行详情', icon: <FileText size={20} /> },
  ];

  // Group node types for the dropdown
  const nodeTypeGroups = nodeTypes.reduce((acc, item) => {
    if (!acc[item.node_type]) {
      acc[item.node_type] = [];
    }
    acc[item.node_type].push(item);
    return acc;
  }, {} as Record<string, NodeTypeSummary[]>);

  const selectedNodeInfo = nodeTypes.find(n => n.node_id === selectedNodeId);

  return (
    <div className="max-w-3xl">
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-gray-900 flex items-center gap-2">
          <Download size={24} />
          数据导出
        </h2>
        <p className="text-gray-500 mt-1">将对话数据导出为不同格式，用于分析和评测</p>
      </div>

      {/* Tab Navigation */}
      <div className="flex border-b border-gray-200 mb-6">
        <button
          onClick={() => setActiveTab('conversation')}
          className={`px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
            activeTab === 'conversation'
              ? 'border-blue-500 text-blue-600'
              : 'border-transparent text-gray-500 hover:text-gray-700'
          }`}
        >
          <span className="flex items-center gap-1.5">
            <MessageSquare size={16} />
            对话数据导出
          </span>
        </button>
        <button
          onClick={() => setActiveTab('node-eval')}
          className={`px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
            activeTab === 'node-eval'
              ? 'border-blue-500 text-blue-600'
              : 'border-transparent text-gray-500 hover:text-gray-700'
          }`}
        >
          <span className="flex items-center gap-1.5">
            <Cpu size={16} />
            节点评测数据导出
            <span className="text-xs bg-green-100 text-green-700 px-1.5 py-0.5 rounded-full ml-1">新</span>
          </span>
        </button>
      </div>

      {activeTab === 'conversation' ? (
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

          <button
            onClick={handleExport}
            disabled={exporting || loading}
            className="w-full py-3 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          >
            {exporting ? (
              <><Loader2 size={18} className="animate-spin" />正在导出...</>
            ) : (
              <><Download size={18} />导出数据</>
            )}
          </button>
        </div>
      ) : (
        /* ===== Node Eval Export Tab ===== */
        <div className="space-y-6">
          {/* Info Banner */}
          <div className="bg-blue-50 border border-blue-200 rounded-xl p-4">
            <div className="flex items-start gap-2">
              <Cpu size={18} className="text-blue-600 mt-0.5 shrink-0" />
              <div className="text-sm text-blue-800">
                <p className="font-medium mb-1">节点评测数据导出</p>
                <p className="text-blue-600">
                  导出工作流中 LLM/Agent 节点的所有输入（Prompt 组装）和输出（回复内容），
                  自动组装为评测框架所需的格式，用于模型质量评测和 Fine-tuning 数据准备。
                </p>
              </div>
            </div>
          </div>

          {/* App Selection */}
          <div className="bg-white rounded-xl border border-gray-200 p-5">
            <h3 className="font-semibold text-gray-900 mb-3">选择应用</h3>
            <select
              value={nodeAppId}
              onChange={(e) => {
                setNodeAppId(e.target.value);
                loadNodeTypes(e.target.value);
                setNodeExportResult(null);
              }}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="">请选择一个应用</option>
              {apps.map((app) => (
                <option key={app.id} value={app.id}>{app.name}</option>
              ))}
            </select>
          </div>

          {/* Node Type Selection - shown after app is selected */}
          {nodeAppId && (
            <div className="bg-white rounded-xl border border-gray-200 p-5">
              <h3 className="font-semibold text-gray-900 mb-3">选择工作流节点</h3>
              {nodeTypesLoading ? (
                <div className="flex items-center gap-2 text-sm text-gray-500">
                  <Loader2 size={16} className="animate-spin" />
                  正在加载节点列表...
                </div>
              ) : nodeTypes.length === 0 ? (
                <div className="text-sm text-amber-600 bg-amber-50 p-3 rounded-lg">
                  该应用没有找到已执行的工作流节点。请先在"数据同步"页面同步该应用的数据。
                </div>
              ) : (
                <div className="space-y-4">
                  {/* Node type filter */}
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">节点类型筛选</label>
                    <select
                      value={selectedNodeType}
                      onChange={(e) => {
                        setSelectedNodeType(e.target.value);
                        setSelectedNodeId('');
                      }}
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                    >
                      <option value="">全部节点类型</option>
                      {Object.keys(nodeTypeGroups).map((type) => (
                        <option key={type} value={type}>
                          {type} ({nodeTypeGroups[type].reduce((sum, n) => sum + n.count, 0)} 条)
                        </option>
                      ))}
                    </select>
                  </div>

                  {/* Specific node selection */}
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">指定节点（可选）</label>
                    <select
                      value={selectedNodeId}
                      onChange={(e) => setSelectedNodeId(e.target.value)}
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                    >
                      <option value="">全部节点</option>
                      {nodeTypes
                        .filter(n => !selectedNodeType || n.node_type === selectedNodeType)
                        .map((n) => (
                          <option key={n.node_id} value={n.node_id}>
                            {n.node_title || n.node_id} ({n.node_type}) — {n.count} 条记录
                          </option>
                        ))}
                    </select>
                  </div>

                  {/* Node summary */}
                  {selectedNodeInfo && (
                    <div className="bg-gray-50 p-3 rounded-lg text-sm">
                      <span className="text-gray-600">已选择: </span>
                      <span className="font-medium text-gray-900">{selectedNodeInfo.node_title || selectedNodeInfo.node_id}</span>
                      <span className="text-gray-500 ml-2">({selectedNodeInfo.node_type})</span>
                      <span className="text-blue-600 ml-2">共 {selectedNodeInfo.count} 条成功执行记录</span>
                    </div>
                  )}

                  {!selectedNodeId && (
                    <div className="bg-gray-50 p-3 rounded-lg text-sm">
                      <span className="text-gray-600">将导出: </span>
                      <span className="font-medium text-gray-900">
                        {selectedNodeType
                          ? `${selectedNodeType} 类型的所有节点`
                          : '所有节点'
                        }
                      </span>
                      <span className="text-blue-600 ml-2">
                        共 {nodeTypes
                          .filter(n => !selectedNodeType || n.node_type === selectedNodeType)
                          .reduce((sum, n) => sum + n.count, 0)} 条记录
                      </span>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}

          {/* Eval Format Selection */}
          {nodeAppId && nodeTypes.length > 0 && (
            <>
              <div className="bg-white rounded-xl border border-gray-200 p-5">
                <h3 className="font-semibold text-gray-900 mb-3">选择评测数据格式</h3>
                <div className="grid grid-cols-1 gap-2">
                  {nodeEvalFormats.map((f) => (
                    <button
                      key={f.value}
                      onClick={() => setNodeEvalFormat(f.value)}
                      className={`p-3 rounded-lg border-2 text-left transition-colors flex items-center gap-3 ${
                        nodeEvalFormat === f.value
                          ? 'border-blue-500 bg-blue-50'
                          : 'border-gray-200 hover:border-gray-300'
                      }`}
                    >
                      <div className={`shrink-0 ${nodeEvalFormat === f.value ? 'text-blue-600' : 'text-gray-400'}`}>
                        {f.icon}
                      </div>
                      <div className="flex-1">
                        <p className="font-medium text-sm text-gray-900">{f.label}</p>
                        <p className="text-xs text-gray-500">{f.desc}</p>
                      </div>
                      {nodeEvalFormat === f.value && (
                        <CheckCircle size={16} className="text-blue-600 shrink-0" />
                      )}
                    </button>
                  ))}
                </div>

                {/* Format description */}
                <div className="mt-3 p-3 bg-gray-50 rounded-lg text-xs text-gray-600 font-mono whitespace-pre-wrap">
                  {nodeEvalFormat === 'openai-eval' && '{"messages": [{"role": "system", "content": "..."}, {"role": "user", "content": "..."}], "ideal": "期望输出"}'}
                  {nodeEvalFormat === 'openai-finetune' && '{"messages": [{"role": "system", ...}, {"role": "user", ...}, {"role": "assistant", "content": "实际输出"}]}'}
                  {nodeEvalFormat === 'alpaca' && '{"instruction": "用户输入", "input": "上下文", "output": "模型输出", "system": "系统提示词"}'}
                  {nodeEvalFormat === 'qa' && '{"query": "用户输入", "context": "检索上下文", "expected_output": "期望输出", "system_prompt": "..."}'}
                  {nodeEvalFormat === 'raw' && '{"execution_id": "...", "prompt_messages": [...], "output": "...", "inputs": {...}, "outputs": {...}, ...}'}
                </div>
              </div>

              {/* Date Range for Node Eval */}
              <div className="bg-white rounded-xl border border-gray-200 p-5">
                <h3 className="font-semibold text-gray-900 mb-3">时间范围（可选）</h3>
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">开始日期</label>
                    <input
                      type="date"
                      value={nodeStartDate}
                      onChange={(e) => setNodeStartDate(e.target.value)}
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">结束日期</label>
                    <input
                      type="date"
                      value={nodeEndDate}
                      onChange={(e) => setNodeEndDate(e.target.value)}
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                </div>
              </div>
            </>
          )}

          {/* Node Eval Export Result */}
          {nodeExportResult && (
            <div className={`flex items-center gap-2 px-4 py-3 rounded-lg text-sm ${
              nodeExportResult.startsWith('导出失败')
                ? 'bg-red-50 text-red-700 border border-red-200'
                : 'bg-green-50 text-green-700 border border-green-200'
            }`}>
              {nodeExportResult.startsWith('导出失败') ? null : <CheckCircle size={16} />}
              {nodeExportResult}
            </div>
          )}

          {/* Node Eval Export Button */}
          {nodeAppId && nodeTypes.length > 0 && (
            <button
              onClick={handleNodeEvalExport}
              disabled={nodeExporting}
              className="w-full py-3 bg-green-600 text-white font-medium rounded-lg hover:bg-green-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {nodeExporting ? (
                <><Loader2 size={18} className="animate-spin" />正在导出评测数据...</>
              ) : (
                <><Download size={18} />导出评测数据 (JSONL)</>
              )}
            </button>
          )}
        </div>
      )}
    </div>
  );
}