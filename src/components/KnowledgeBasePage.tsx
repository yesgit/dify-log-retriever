import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Library, RefreshCw, Download, Loader2, FolderOpen } from 'lucide-react';
import type { DifyDataset, DifyDatasetDocument, DatasetDocDownloadResult } from '../types';

const DIR_STORAGE_KEY = 'knowledge-download-dir';

function loadSavedDir(): string {
  try {
    return localStorage.getItem(DIR_STORAGE_KEY) || '';
  } catch {
    return '';
  }
}

function saveDir(dir: string) {
  try {
    localStorage.setItem(DIR_STORAGE_KEY, dir);
  } catch {
    // storage unavailable; ignore
  }
}

const SOURCE_LABELS: Record<string, string> = {
  upload_file: '上传文件',
  notion_import: 'Notion',
  website_crawl: '网页',
};

const INDEX_STATUS_LABELS: Record<string, string> = {
  completed: '索引完成',
  waiting: '排队中',
  parsing: '解析中',
  cleaning: '清洗中',
  splitting: '分段中',
  indexing: '索引中',
  error: '索引失败',
  paused: '已暂停',
};

export default function KnowledgeBasePage() {
  const [datasets, setDatasets] = useState<DifyDataset[]>([]);
  const [datasetKeyword, setDatasetKeyword] = useState('');
  const [loadingDatasets, setLoadingDatasets] = useState(false);

  const [selectedDataset, setSelectedDataset] = useState<DifyDataset | null>(null);
  const [documents, setDocuments] = useState<DifyDatasetDocument[]>([]);
  const [loadingDocs, setLoadingDocs] = useState(false);
  const [docKeyword, setDocKeyword] = useState('');
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  const [targetDir, setTargetDir] = useState(loadSavedDir);
  const [withMarkers, setWithMarkers] = useState(true);
  const [downloading, setDownloading] = useState(false);
  const [results, setResults] = useState<DatasetDocDownloadResult[] | null>(null);
  const [error, setError] = useState('');

  useEffect(() => {
    loadDatasets();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const loadDatasets = async (keyword?: string) => {
    setLoadingDatasets(true);
    setError('');
    try {
      const list = await invoke<DifyDataset[]>('fetch_knowledge_datasets', {
        keyword: keyword?.trim() || null,
      });
      setDatasets(list);
    } catch (e: any) {
      setError(`获取知识库列表失败: ${e}`);
    } finally {
      setLoadingDatasets(false);
    }
  };

  const selectDataset = async (ds: DifyDataset) => {
    setSelectedDataset(ds);
    setDocuments([]);
    setSelectedIds(new Set());
    setResults(null);
    setDocKeyword('');
    setError('');
    setLoadingDocs(true);
    try {
      const docs = await invoke<DifyDatasetDocument[]>('fetch_knowledge_documents', {
        datasetId: ds.id,
        keyword: null,
      });
      setDocuments(docs);
    } catch (e: any) {
      setError(`获取文档列表失败: ${e}`);
    } finally {
      setLoadingDocs(false);
    }
  };

  const filteredDocs = useMemo(() => {
    const kw = docKeyword.trim().toLowerCase();
    if (!kw) return documents;
    return documents.filter((d) => d.name.toLowerCase().includes(kw));
  }, [documents, docKeyword]);

  const toggleDoc = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleAllVisible = () => {
    setSelectedIds((prev) => {
      const visibleIds = filteredDocs.map((d) => d.id);
      const allVisibleSelected =
        visibleIds.length > 0 && visibleIds.every((id) => prev.has(id));
      const next = new Set(prev);
      if (allVisibleSelected) {
        visibleIds.forEach((id) => next.delete(id));
      } else {
        visibleIds.forEach((id) => next.add(id));
      }
      return next;
    });
  };

  const browseDir = async () => {
    try {
      const selected = await open({ directory: true, title: '选择下载目录' });
      if (selected) {
        setTargetDir(selected);
        saveDir(selected);
      }
    } catch {
      // user cancelled
    }
  };

  const startDownload = async (docs: DifyDatasetDocument[]) => {
    if (docs.length === 0) return;
    if (!targetDir.trim()) {
      setError('请先选择下载目录');
      return;
    }
    saveDir(targetDir.trim());
    setDownloading(true);
    setResults(null);
    setError('');
    try {
      const refs = docs.map((d) => ({
        id: d.id,
        name: d.name,
        data_source_type: d.data_source_type,
      }));
      const r = await invoke<DatasetDocDownloadResult[]>('download_knowledge_documents', {
        datasetId: selectedDataset!.id,
        datasetName: selectedDataset!.name,
        documents: refs,
        targetDir: targetDir.trim(),
        withMarkers,
      });
      setResults(r);
      const failed = r.filter((x) => !x.success).length;
      if (failed > 0) {
        setError(`${failed} 个文档下载失败，详见下方结果`);
      }
    } catch (e: any) {
      setError(`下载失败: ${e}`);
    } finally {
      setDownloading(false);
    }
  };

  const selectedDocs = documents.filter((d) => selectedIds.has(d.id));
  const successCount = results ? results.filter((r) => r.success).length : 0;
  const failCount = results ? results.filter((r) => !r.success).length : 0;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-800">知识库文档下载</h2>
        <div className="text-sm text-gray-500">共 {datasets.length} 个知识库</div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Dataset list */}
        <div className="bg-white rounded-lg border border-gray-200 p-4 flex flex-col">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-base font-medium text-gray-700 flex items-center gap-2">
              <Library size={18} className="text-blue-600" />
              知识库
            </h3>
            <button
              onClick={() => loadDatasets(datasetKeyword)}
              disabled={loadingDatasets}
              className="p-1.5 text-gray-500 hover:text-blue-600 hover:bg-blue-50 rounded transition-colors"
              title="刷新列表"
            >
              <RefreshCw size={16} className={loadingDatasets ? 'animate-spin' : ''} />
            </button>
          </div>
          <input
            type="text"
            value={datasetKeyword}
            onChange={(e) => setDatasetKeyword(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') loadDatasets(datasetKeyword);
            }}
            placeholder="搜索知识库名称，回车确认"
            className="w-full px-3 py-2 mb-3 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <div className="flex-1 overflow-auto max-h-[420px] space-y-1">
            {loadingDatasets && datasets.length === 0 && (
              <div className="text-sm text-gray-400 py-4 text-center">加载中...</div>
            )}
            {!loadingDatasets && datasets.length === 0 && (
              <div className="text-sm text-gray-400 py-4 text-center">暂无知识库</div>
            )}
            {datasets.map((ds) => (
              <button
                key={ds.id}
                onClick={() => selectDataset(ds)}
                className={`w-full text-left px-3 py-2 rounded-md text-sm transition-colors ${
                  selectedDataset?.id === ds.id
                    ? 'bg-blue-50 text-blue-700 border border-blue-200'
                    : 'hover:bg-gray-50 border border-transparent'
                }`}
              >
                <div className="font-medium truncate">{ds.name || ds.id}</div>
                <div className="text-xs text-gray-500">
                  {ds.document_count} 个文档 · {ds.word_count} 词
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Document list */}
        <div className="lg:col-span-2 bg-white rounded-lg border border-gray-200 p-4 flex flex-col">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-base font-medium text-gray-700 truncate">
              {selectedDataset ? `文档：${selectedDataset.name}` : '文档列表'}
            </h3>
            {selectedDataset && (
              <div className="text-xs text-gray-500 shrink-0 ml-2">
                {documents.length} 个文档，已选 {selectedDocs.length}
              </div>
            )}
          </div>

          {!selectedDataset ? (
            <div className="text-sm text-gray-400 py-10 text-center">
              请先在左侧选择一个知识库
            </div>
          ) : (
            <>
              <input
                type="text"
                value={docKeyword}
                onChange={(e) => setDocKeyword(e.target.value)}
                placeholder="按名称筛选（筛选后全选仅作用于可见文档）"
                className="w-full px-3 py-2 mb-3 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />

              <div className="flex items-center gap-2 mb-3 flex-wrap">
                <button
                  onClick={toggleAllVisible}
                  disabled={filteredDocs.length === 0}
                  className="px-3 py-1.5 bg-gray-100 text-gray-700 rounded-md text-sm hover:bg-gray-200 disabled:opacity-50 transition-colors"
                >
                  全选 / 取消全选
                </button>
                <button
                  onClick={() => startDownload(selectedDocs)}
                  disabled={downloading || selectedDocs.length === 0 || !targetDir.trim()}
                  className="px-3 py-1.5 bg-blue-600 text-white rounded-md text-sm hover:bg-blue-700 disabled:opacity-50 transition-colors inline-flex items-center gap-1.5"
                >
                  {downloading ? (
                    <Loader2 size={14} className="animate-spin" />
                  ) : (
                    <Download size={14} />
                  )}
                  下载所选（{selectedDocs.length}）
                </button>
                <button
                  onClick={() => startDownload(documents)}
                  disabled={downloading || documents.length === 0 || !targetDir.trim()}
                  className="px-3 py-1.5 bg-green-600 text-white rounded-md text-sm hover:bg-green-700 disabled:opacity-50 transition-colors inline-flex items-center gap-1.5"
                  title="下载当前知识库的全部文档（无需勾选）"
                >
                  {downloading ? (
                    <Loader2 size={14} className="animate-spin" />
                  ) : (
                    <Download size={14} />
                  )}
                  下载全部（{documents.length}）
                </button>
              </div>

              <div className="flex items-center gap-2 mb-3">
                <FolderOpen size={16} className="text-gray-500 shrink-0" />
                <input
                  type="text"
                  value={targetDir}
                  onChange={(e) => setTargetDir(e.target.value)}
                  placeholder="下载目录（在其下按知识库名建子目录）"
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <button
                  onClick={browseDir}
                  className="px-4 py-2 bg-gray-100 text-gray-700 rounded-md text-sm hover:bg-gray-200 transition-colors shrink-0"
                >
                  浏览
                </button>
              </div>
              <label className="flex items-center gap-2 mb-2">
                <input
                  type="checkbox"
                  checked={withMarkers}
                  onChange={(e) => setWithMarkers(e.target.checked)}
                  className="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
                />
                <span className="text-sm text-gray-600">
                  分段文本重建时插入「======== 分段 N ========」标记
                  <span className="text-gray-400">（仅影响无原始文件、按分段重建为 TXT 的文档；Dify 切分时不会保留原始分隔符，只能显式插入）</span>
                </span>
              </label>
              {!targetDir.trim() && (
                <p className="text-sm text-amber-600 mb-2">请先设置下载目录</p>
              )}

              <div className="flex-1 overflow-auto max-h-[360px] border border-gray-100 rounded">
                {loadingDocs ? (
                  <div className="text-sm text-gray-400 py-8 text-center">加载文档列表...</div>
                ) : filteredDocs.length === 0 ? (
                  <div className="text-sm text-gray-400 py-8 text-center">
                    {documents.length === 0 ? '该知识库暂无文档' : '无匹配文档'}
                  </div>
                ) : (
                  <table className="w-full text-sm">
                    <thead className="sticky top-0 bg-gray-50">
                      <tr className="border-b border-gray-200 text-left">
                        <th className="py-2 px-3 w-10">
                          <input
                            type="checkbox"
                            checked={
                              filteredDocs.length > 0 &&
                              filteredDocs.every((d) => selectedIds.has(d.id))
                            }
                            onChange={toggleAllVisible}
                            className="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
                          />
                        </th>
                        <th className="py-2 px-3 font-medium text-gray-600">名称</th>
                        <th className="py-2 px-3 font-medium text-gray-600 w-24">来源</th>
                        <th className="py-2 px-3 font-medium text-gray-600 w-24">状态</th>
                        <th className="py-2 px-3 font-medium text-gray-600 w-20">字数</th>
                      </tr>
                    </thead>
                    <tbody>
                      {filteredDocs.map((doc) => (
                        <tr key={doc.id} className="border-b border-gray-100 hover:bg-gray-50">
                          <td className="py-2 px-3">
                            <input
                              type="checkbox"
                              checked={selectedIds.has(doc.id)}
                              onChange={() => toggleDoc(doc.id)}
                              className="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
                            />
                          </td>
                          <td className="py-2 px-3">
                            <div className="truncate max-w-[420px]" title={doc.name}>
                              {doc.name || doc.id}
                            </div>
                          </td>
                          <td className="py-2 px-3 text-gray-500">
                            {SOURCE_LABELS[doc.data_source_type] || doc.data_source_type || '-'}
                          </td>
                          <td className="py-2 px-3">
                            <span
                              className={`inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium ${
                                doc.indexing_status === 'completed'
                                  ? 'bg-green-100 text-green-800'
                                  : doc.indexing_status === 'error'
                                  ? 'bg-red-100 text-red-800'
                                  : 'bg-gray-100 text-gray-600'
                              }`}
                            >
                              {INDEX_STATUS_LABELS[doc.indexing_status] || doc.indexing_status}
                            </span>
                          </td>
                          <td className="py-2 px-3 text-gray-500">{doc.word_count}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            </>
          )}
        </div>
      </div>

      {/* Error */}
      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-sm text-red-700">{error}</p>
        </div>
      )}

      {/* Results */}
      {results && (
        <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-4">
          <h3 className="text-base font-medium text-gray-700">
            下载结果
            <span className="ml-3 text-sm font-normal text-gray-500">
              成功 {successCount} / 失败 {failCount} / 共 {results.length} 个文档
            </span>
          </h3>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200 text-left">
                  <th className="text-left py-2 px-3 font-medium text-gray-600">文档名称</th>
                  <th className="text-left py-2 px-3 font-medium text-gray-600 w-20">状态</th>
                  <th className="text-left py-2 px-3 font-medium text-gray-600 w-28">方式</th>
                  <th className="text-left py-2 px-3 font-medium text-gray-600">保存路径</th>
                  <th className="text-left py-2 px-3 font-medium text-gray-600">错误信息</th>
                </tr>
              </thead>
              <tbody>
                {results.map((r) => (
                  <tr key={r.document_id} className="border-b border-gray-100">
                    <td className="py-2 px-3">{r.document_name || r.document_id}</td>
                    <td className="py-2 px-3">
                      {r.success ? (
                        <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-green-100 text-green-800">
                          ✓ 成功
                        </span>
                      ) : (
                        <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-red-100 text-red-800">
                          ✗ 失败
                        </span>
                      )}
                    </td>
                    <td className="py-2 px-3 text-gray-500">
                      {r.saved_mode === 'original'
                        ? '原始文件'
                        : r.saved_mode === 'text'
                        ? '分段文本'
                        : '-'}
                    </td>
                    <td className="py-2 px-3 text-xs text-gray-500 break-all">
                      {r.file_path || '-'}
                    </td>
                    <td className="py-2 px-3 text-xs text-red-600 break-all">{r.error || '-'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
