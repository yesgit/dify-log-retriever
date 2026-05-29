import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import {
  Activity,
  Archive,
  CheckCircle2,
  Database,
  Download,
  RefreshCw,
  Search,
  Settings,
} from "lucide-react";
import "./styles.css";

type AppConfig = {
  id: number;
  name: string;
  baseUrl: string;
  difyAppId: string;
  tokenConfigured: boolean;
  updatedAt: string;
};

type Conversation = {
  id: string;
  appConfigId: number;
  difyConversationId: string;
  name?: string;
  summary?: string;
  status?: string;
  messageCount?: number;
  createdAt?: number;
  updatedAt?: number;
  syncedAt: string;
};

type SyncSummary = {
  syncRunId: string;
  status: string;
  conversations: number;
  messages: number;
  workflowRuns: number;
  nodeExecutions: number;
  error?: string;
};

type ExportSummary = {
  outputPath: string;
  messageRows: number;
  nodeRows: number;
};

type Detail = {
  conversation: unknown;
  messages: unknown[];
  workflowRuns: unknown[];
  nodeExecutions: unknown[];
};

const emptyForm = {
  name: "",
  baseUrl: "",
  difyAppId: "",
  token: "",
};

const isTauriRuntime = "__TAURI_INTERNALS__" in window;

async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime) {
    return tauriInvoke<T>(command, args);
  }

  if (command === "list_app_configs") {
    return [] as T;
  }

  throw new Error("当前是浏览器预览模式，后端 SQLite/同步/导出功能需要在 Tauri 桌面窗口中使用。");
}

function App() {
  const [tab, setTab] = useState("settings");
  const [apps, setApps] = useState<AppConfig[]>([]);
  const [selectedAppId, setSelectedAppId] = useState<number | "">("");
  const [status, setStatus] = useState("");

  const selectedApp = useMemo(
    () => apps.find((app) => app.id === selectedAppId),
    [apps, selectedAppId],
  );

  async function loadApps() {
    const rows = await safeInvoke<AppConfig[]>("list_app_configs");
    setApps(rows);
    if (!isTauriRuntime) {
      setStatus("浏览器预览模式：可查看界面，真实同步和导出需要运行 Tauri 桌面窗口。");
    }
    if (!selectedAppId && rows.length > 0) {
      setSelectedAppId(rows[0].id);
    }
  }

  useEffect(() => {
    loadApps().catch((error) => setStatus(String(error)));
  }, []);

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <Database size={22} />
          <div>
            <strong>Dify Log Retriever</strong>
            <span>对话归档与评测导出</span>
          </div>
        </div>
        <nav>
          <TabButton icon={<Settings size={18} />} active={tab === "settings"} onClick={() => setTab("settings")}>
            应用配置
          </TabButton>
          <TabButton icon={<RefreshCw size={18} />} active={tab === "sync"} onClick={() => setTab("sync")}>
            同步
          </TabButton>
          <TabButton icon={<Archive size={18} />} active={tab === "logs"} onClick={() => setTab("logs")}>
            日志浏览
          </TabButton>
          <TabButton icon={<Download size={18} />} active={tab === "export"} onClick={() => setTab("export")}>
            导出
          </TabButton>
        </nav>
        <label className="field compact">
          <span>当前应用</span>
          <select
            value={selectedAppId}
            onChange={(event) => setSelectedAppId(Number(event.target.value))}
          >
            <option value="" disabled>
              选择应用
            </option>
            {apps.map((app) => (
              <option value={app.id} key={app.id}>
                {app.name}
              </option>
            ))}
          </select>
        </label>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div>
            <h1>{titleFor(tab)}</h1>
            <p>{selectedApp ? `${selectedApp.name} · ${selectedApp.difyAppId}` : "尚未选择应用"}</p>
          </div>
          <button className="icon-button" onClick={() => loadApps()} title="刷新应用">
            <RefreshCw size={18} />
          </button>
        </header>

        {status && <div className="notice">{status}</div>}
        {tab === "settings" && (
          <SettingsPanel apps={apps} reload={loadApps} setStatus={setStatus} />
        )}
        {tab === "sync" && (
          <SyncPanel appId={selectedAppId} setStatus={setStatus} />
        )}
        {tab === "logs" && (
          <LogsPanel appId={selectedAppId} setStatus={setStatus} />
        )}
        {tab === "export" && (
          <ExportPanel appId={selectedAppId} setStatus={setStatus} />
        )}
      </section>
    </main>
  );
}

function TabButton(props: {
  icon: React.ReactNode;
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button className={props.active ? "tab active" : "tab"} onClick={props.onClick}>
      {props.icon}
      <span>{props.children}</span>
    </button>
  );
}

function SettingsPanel(props: {
  apps: AppConfig[];
  reload: () => Promise<void>;
  setStatus: (value: string) => void;
}) {
  const [form, setForm] = useState(emptyForm);
  const [editingId, setEditingId] = useState<number | undefined>();

  function edit(app: AppConfig) {
    setEditingId(app.id);
    setForm({
      name: app.name,
      baseUrl: app.baseUrl,
      difyAppId: app.difyAppId,
      token: "",
    });
  }

  async function save() {
    try {
      props.setStatus("保存应用配置中...");
      const saved = await safeInvoke<AppConfig>("upsert_app_config", {
        input: {
          id: editingId,
          name: form.name,
          baseUrl: form.baseUrl,
          difyAppId: form.difyAppId,
          token: form.token || null,
        },
      });
      props.setStatus(`已保存：${saved.name}`);
      setEditingId(undefined);
      setForm(emptyForm);
      await props.reload();
    } catch (error) {
      props.setStatus(String(error));
    }
  }

  async function test(appId: number) {
    try {
      props.setStatus("测试连接中...");
      await safeInvoke("test_app_connection", { appConfigId: appId });
      props.setStatus("连接测试成功");
    } catch (error) {
      props.setStatus(String(error));
    }
  }

  return (
    <div className="grid two">
      <section className="panel">
        <h2>{editingId ? "编辑应用" : "新增应用"}</h2>
        <div className="form-grid">
          <Input label="名称" value={form.name} onChange={(name) => setForm({ ...form, name })} />
          <Input label="Dify Console URL" value={form.baseUrl} onChange={(baseUrl) => setForm({ ...form, baseUrl })} />
          <Input label="App ID" value={form.difyAppId} onChange={(difyAppId) => setForm({ ...form, difyAppId })} />
          <Input label="Bearer Token" type="password" value={form.token} onChange={(token) => setForm({ ...form, token })} />
        </div>
        <div className="actions">
          <button onClick={save} disabled={!isTauriRuntime}>
            <CheckCircle2 size={17} />
            保存
          </button>
          <button className="secondary" onClick={() => { setEditingId(undefined); setForm(emptyForm); }}>
            清空
          </button>
        </div>
      </section>
      <section className="panel">
        <h2>已配置应用</h2>
        <div className="list">
          {props.apps.map((app) => (
            <article className="row-card" key={app.id}>
              <div>
                <strong>{app.name}</strong>
                <span>{app.baseUrl}</span>
                <code>{app.difyAppId}</code>
              </div>
              <div className="row-actions">
                <button className="icon-button" onClick={() => test(app.id)} title="测试连接" disabled={!isTauriRuntime}>
                  <Activity size={17} />
                </button>
                <button className="secondary" onClick={() => edit(app)}>
                  编辑
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}

function SyncPanel(props: { appId: number | ""; setStatus: (value: string) => void }) {
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [summary, setSummary] = useState<SyncSummary | null>(null);

  async function sync() {
    if (!props.appId) return props.setStatus("请先选择应用");
    try {
      props.setStatus("同步进行中...");
      const result = await safeInvoke<SyncSummary>("sync_app_logs", {
        request: { appConfigId: props.appId, start, end },
      });
      setSummary(result);
      props.setStatus(result.error ? `同步失败：${result.error}` : "同步完成");
    } catch (error) {
      props.setStatus(String(error));
    }
  }

  return (
    <section className="panel">
      <h2>手动增量同步</h2>
      <div className="form-row">
        <Input label="开始时间" type="datetime-local" value={start} onChange={setStart} />
        <Input label="结束时间" type="datetime-local" value={end} onChange={setEnd} />
        <button onClick={sync} disabled={!isTauriRuntime}>
          <RefreshCw size={17} />
          同步
        </button>
      </div>
      {summary && (
        <div className="metrics">
          <Metric label="会话" value={summary.conversations} />
          <Metric label="消息" value={summary.messages} />
          <Metric label="工作流" value={summary.workflowRuns} />
          <Metric label="节点" value={summary.nodeExecutions} />
        </div>
      )}
    </section>
  );
}

function LogsPanel(props: { appId: number | ""; setStatus: (value: string) => void }) {
  const [keyword, setKeyword] = useState("");
  const [rows, setRows] = useState<Conversation[]>([]);
  const [detail, setDetail] = useState<Detail | null>(null);

  async function search() {
    if (!props.appId) return props.setStatus("请先选择应用");
    try {
      const result = await safeInvoke<Conversation[]>("list_conversations", {
        filters: { appConfigId: props.appId, keyword, limit: 200 },
      });
      setRows(result);
      props.setStatus(`找到 ${result.length} 条会话`);
    } catch (error) {
      props.setStatus(String(error));
    }
  }

  async function open(row: Conversation) {
    try {
      const result = await safeInvoke<Detail>("get_conversation_detail", {
        appConfigId: row.appConfigId,
        conversationId: row.difyConversationId,
      });
      setDetail(result);
    } catch (error) {
      props.setStatus(String(error));
    }
  }

  return (
    <div className="grid logs">
      <section className="panel">
        <h2>会话</h2>
        <div className="searchbar">
          <Search size={17} />
          <input value={keyword} onChange={(event) => setKeyword(event.target.value)} placeholder="关键词" />
          <button onClick={search}>查询</button>
        </div>
        <div className="list scroll">
          {rows.map((row) => (
            <button className="conversation-row" key={row.id} onClick={() => open(row)}>
              <strong>{row.name || row.summary || row.difyConversationId}</strong>
              <span>{row.status || "unknown"} · {row.messageCount ?? 0} messages</span>
            </button>
          ))}
        </div>
      </section>
      <section className="panel detail">
        <h2>详情</h2>
        <pre>{detail ? JSON.stringify(detail, null, 2) : "选择一条会话查看原始副本和节点执行"}</pre>
      </section>
    </div>
  );
}

function ExportPanel(props: { appId: number | ""; setStatus: (value: string) => void }) {
  const [path, setPath] = useState("");
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [granularity, setGranularity] = useState("both");
  const [summary, setSummary] = useState<ExportSummary | null>(null);

  async function exportRows() {
    if (!props.appId) return props.setStatus("请先选择应用");
    if (!path) return props.setStatus("请输入导出路径");
    try {
      const result = await safeInvoke<ExportSummary>("export_jsonl", {
        request: { appConfigId: props.appId, start, end, granularity, outputPath: path },
      });
      setSummary(result);
      props.setStatus("导出完成");
    } catch (error) {
      props.setStatus(String(error));
    }
  }

  return (
    <section className="panel">
      <h2>JSONL 导出</h2>
      <div className="form-row">
        <label className="field grow">
          <span>输出文件</span>
          <input value={path} onChange={(event) => setPath(event.target.value)} placeholder="/tmp/dify-eval.jsonl" />
        </label>
        <Input label="开始时间" type="datetime-local" value={start} onChange={setStart} />
        <Input label="结束时间" type="datetime-local" value={end} onChange={setEnd} />
        <label className="field">
          <span>粒度</span>
          <select value={granularity} onChange={(event) => setGranularity(event.target.value)}>
            <option value="both">消息 + 节点</option>
            <option value="messages">仅消息</option>
            <option value="nodes">仅 LLM/Agent 节点</option>
          </select>
        </label>
        <button onClick={exportRows} disabled={!isTauriRuntime}>
          <Download size={17} />
          导出
        </button>
      </div>
      {summary && (
        <div className="metrics">
          <Metric label="消息行" value={summary.messageRows} />
          <Metric label="节点行" value={summary.nodeRows} />
        </div>
      )}
    </section>
  );
}

function Input(props: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: string;
  placeholder?: string;
}) {
  return (
    <label className="field">
      <span>{props.label}</span>
      <input
        type={props.type || "text"}
        value={props.value}
        placeholder={props.placeholder}
        onChange={(event) => props.onChange(event.target.value)}
      />
    </label>
  );
}

function Metric(props: { label: string; value: number }) {
  return (
    <div className="metric">
      <span>{props.label}</span>
      <strong>{props.value}</strong>
    </div>
  );
}

function titleFor(tab: string) {
  return {
    settings: "应用配置",
    sync: "同步",
    logs: "日志浏览",
    export: "导出",
  }[tab];
}

createRoot(document.getElementById("root")!).render(<App />);
