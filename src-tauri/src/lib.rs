use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use rand::RngCore;
use reqwest::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{Manager, State};
use url::Url;
use uuid::Uuid;

const KEYRING_SERVICE: &str = "dify-log-retriever";
const KEYRING_USER: &str = "token-encryption-key";

type AppResult<T> = Result<T, String>;

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    key_fallback_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppConfigInput {
    id: Option<i64>,
    name: String,
    base_url: String,
    dify_app_id: String,
    token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppConfigView {
    id: i64,
    name: String,
    base_url: String,
    dify_app_id: String,
    token_configured: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationFilters {
    app_config_id: Option<i64>,
    keyword: Option<String>,
    status: Option<String>,
    start: Option<String>,
    end: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConversationRow {
    id: String,
    app_config_id: i64,
    dify_conversation_id: String,
    name: Option<String>,
    summary: Option<String>,
    status: Option<String>,
    message_count: Option<i64>,
    created_at: Option<i64>,
    updated_at: Option<i64>,
    synced_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConversationDetail {
    conversation: Value,
    messages: Vec<Value>,
    workflow_runs: Vec<Value>,
    node_executions: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncRequest {
    app_config_id: i64,
    start: Option<String>,
    end: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncSummary {
    sync_run_id: String,
    status: String,
    conversations: u64,
    messages: u64,
    workflow_runs: u64,
    node_executions: u64,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportRequest {
    app_config_id: i64,
    start: Option<String>,
    end: Option<String>,
    granularity: String,
    output_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportSummary {
    output_path: String,
    message_rows: u64,
    node_rows: u64,
}

#[derive(Debug, Clone)]
struct StoredAppConfig {
    id: i64,
    base_url: String,
    dify_app_id: String,
    encrypted_token: String,
}

#[derive(Default)]
struct SyncCounters {
    conversations: u64,
    messages: u64,
    workflow_runs: u64,
    node_executions: u64,
}

#[derive(Debug, Deserialize)]
struct ListPage {
    #[serde(default)]
    data: Vec<Value>,
    #[serde(default)]
    has_more: bool,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            fs::create_dir_all(&app_dir)?;
            let db_path = app_dir.join("dify-log-retriever.sqlite");
            let key_fallback_path = app_dir.join(".token-key");
            let state = AppState {
                db_path,
                key_fallback_path,
            };
            init_db(&state.db_path)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_app_configs,
            upsert_app_config,
            test_app_connection,
            sync_app_logs,
            list_conversations,
            get_conversation_detail,
            export_jsonl
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Dify Log Retriever");
}

fn open_db(state: &AppState) -> AppResult<Connection> {
    Connection::open(&state.db_path).map_err(to_string)
}

fn init_db(db_path: &Path) -> AppResult<()> {
    let conn = Connection::open(db_path).map_err(to_string)?;
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS apps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            base_url TEXT NOT NULL,
            dify_app_id TEXT NOT NULL,
            encrypted_token TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(base_url, dify_app_id)
        );
        CREATE TABLE IF NOT EXISTS sync_runs (
            id TEXT PRIMARY KEY,
            app_config_id INTEGER NOT NULL,
            start_filter TEXT,
            end_filter TEXT,
            status TEXT NOT NULL,
            error TEXT,
            conversations_count INTEGER NOT NULL DEFAULT 0,
            messages_count INTEGER NOT NULL DEFAULT 0,
            workflow_runs_count INTEGER NOT NULL DEFAULT 0,
            node_executions_count INTEGER NOT NULL DEFAULT 0,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            FOREIGN KEY(app_config_id) REFERENCES apps(id) ON DELETE CASCADE
        );
        "#,
    )
    .map_err(to_string)?;
    ensure_composite_tables(&conn)?;
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_conversations_app_created ON conversations(app_config_id, created_at);
        CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(app_config_id, dify_conversation_id, created_at);
        CREATE INDEX IF NOT EXISTS idx_nodes_workflow ON node_executions(app_config_id, workflow_run_id, node_type);
        "#,
    )
    .map_err(to_string)?;
    Ok(())
}

fn ensure_composite_tables(conn: &Connection) -> AppResult<()> {
    ensure_composite_table(
        conn,
        "conversations",
        "dify_conversation_id",
        r#"
        CREATE TABLE conversations (
            dify_conversation_id TEXT NOT NULL,
            app_config_id INTEGER NOT NULL,
            name TEXT,
            summary TEXT,
            status TEXT,
            message_count INTEGER,
            created_at INTEGER,
            updated_at INTEGER,
            raw_json TEXT NOT NULL,
            detail_json TEXT,
            synced_at TEXT NOT NULL,
            PRIMARY KEY(app_config_id, dify_conversation_id),
            FOREIGN KEY(app_config_id) REFERENCES apps(id) ON DELETE CASCADE
        );
        "#,
        "dify_conversation_id, app_config_id, name, summary, status, message_count, created_at, updated_at, raw_json, detail_json, synced_at",
    )?;
    ensure_composite_table(
        conn,
        "messages",
        "dify_message_id",
        r#"
        CREATE TABLE messages (
            dify_message_id TEXT NOT NULL,
            app_config_id INTEGER NOT NULL,
            dify_conversation_id TEXT NOT NULL,
            workflow_run_id TEXT,
            query TEXT,
            answer TEXT,
            status TEXT,
            created_at INTEGER,
            raw_json TEXT NOT NULL,
            synced_at TEXT NOT NULL,
            PRIMARY KEY(app_config_id, dify_message_id),
            FOREIGN KEY(app_config_id) REFERENCES apps(id) ON DELETE CASCADE
        );
        "#,
        "dify_message_id, app_config_id, dify_conversation_id, workflow_run_id, query, answer, status, created_at, raw_json, synced_at",
    )?;
    ensure_composite_table(
        conn,
        "workflow_runs",
        "dify_workflow_run_id",
        r#"
        CREATE TABLE workflow_runs (
            dify_workflow_run_id TEXT NOT NULL,
            app_config_id INTEGER NOT NULL,
            status TEXT,
            created_at INTEGER,
            finished_at INTEGER,
            raw_json TEXT NOT NULL,
            synced_at TEXT NOT NULL,
            PRIMARY KEY(app_config_id, dify_workflow_run_id),
            FOREIGN KEY(app_config_id) REFERENCES apps(id) ON DELETE CASCADE
        );
        "#,
        "dify_workflow_run_id, app_config_id, status, created_at, finished_at, raw_json, synced_at",
    )?;
    ensure_composite_table(
        conn,
        "node_executions",
        "dify_node_execution_id",
        r#"
        CREATE TABLE node_executions (
            dify_node_execution_id TEXT NOT NULL,
            app_config_id INTEGER NOT NULL,
            workflow_run_id TEXT NOT NULL,
            message_id TEXT,
            conversation_id TEXT,
            node_id TEXT,
            node_type TEXT,
            title TEXT,
            status TEXT,
            created_at INTEGER,
            raw_json TEXT NOT NULL,
            synced_at TEXT NOT NULL,
            PRIMARY KEY(app_config_id, dify_node_execution_id),
            FOREIGN KEY(app_config_id) REFERENCES apps(id) ON DELETE CASCADE
        );
        "#,
        "dify_node_execution_id, app_config_id, workflow_run_id, message_id, conversation_id, node_id, node_type, title, status, created_at, raw_json, synced_at",
    )?;
    Ok(())
}

fn ensure_composite_table(
    conn: &Connection,
    table: &str,
    legacy_id_col: &str,
    create_sql: &str,
    columns: &str,
) -> AppResult<()> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            params![table],
            |row| row.get(0),
        )
        .map_err(to_string)?;
    if !exists {
        conn.execute_batch(create_sql).map_err(to_string)?;
        return Ok(());
    }

    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(to_string)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?))
        })
        .map_err(to_string)?;
    let pk_cols = collect_rows(rows)?;
    let has_composite_pk = pk_cols
        .iter()
        .any(|(name, pk)| name == "app_config_id" && *pk > 0)
        && pk_cols
            .iter()
            .any(|(name, pk)| name == legacy_id_col && *pk > 0);
    if has_composite_pk {
        return Ok(());
    }

    let legacy = format!("{table}_legacy_{}", Utc::now().timestamp());
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .map_err(to_string)?;
    conn.execute(&format!("ALTER TABLE {table} RENAME TO {legacy}"), [])
        .map_err(to_string)?;
    conn.execute_batch(create_sql).map_err(to_string)?;
    conn.execute(
        &format!("INSERT OR IGNORE INTO {table} ({columns}) SELECT {columns} FROM {legacy}"),
        [],
    )
    .map_err(to_string)?;
    conn.execute(&format!("DROP TABLE {legacy}"), [])
        .map_err(to_string)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(to_string)?;
    Ok(())
}

#[tauri::command]
fn list_app_configs(state: State<AppState>) -> AppResult<Vec<AppConfigView>> {
    let conn = open_db(&state)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, base_url, dify_app_id, encrypted_token, created_at, updated_at
             FROM apps ORDER BY updated_at DESC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(AppConfigView {
                id: row.get(0)?,
                name: row.get(1)?,
                base_url: row.get(2)?,
                dify_app_id: row.get(3)?,
                token_configured: !row.get::<_, String>(4)?.is_empty(),
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(to_string)?;
    collect_rows(rows)
}

#[tauri::command]
fn upsert_app_config(input: AppConfigInput, state: State<AppState>) -> AppResult<AppConfigView> {
    validate_app_config(&input)?;
    let key = load_or_create_master_key(&state)?;
    let conn = open_db(&state)?;
    let now = now_string();

    let encrypted_token = match (&input.id, input.token.as_deref()) {
        (_, Some(token)) if !token.trim().is_empty() => encrypt_token(token.trim(), &key)?,
        (Some(id), _) => conn
            .query_row(
                "SELECT encrypted_token FROM apps WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(to_string)?
            .ok_or_else(|| "应用配置不存在".to_string())?,
        (None, _) => return Err("新增应用必须提供 Bearer token".to_string()),
    };

    if let Some(id) = input.id {
        conn.execute(
            "UPDATE apps
             SET name = ?1, base_url = ?2, dify_app_id = ?3, encrypted_token = ?4, updated_at = ?5
             WHERE id = ?6",
            params![
                input.name.trim(),
                normalize_base_url(&input.base_url)?,
                input.dify_app_id.trim(),
                encrypted_token,
                now,
                id
            ],
        )
        .map_err(to_string)?;
        get_app_view(&conn, id)
    } else {
        conn.execute(
            "INSERT INTO apps (name, base_url, dify_app_id, encrypted_token, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(base_url, dify_app_id) DO UPDATE SET
               name = excluded.name,
               encrypted_token = excluded.encrypted_token,
               updated_at = excluded.updated_at",
            params![
                input.name.trim(),
                normalize_base_url(&input.base_url)?,
                input.dify_app_id.trim(),
                encrypted_token,
                now
            ],
        )
        .map_err(to_string)?;
        let stored = get_app_by_identity(&conn, &input.base_url, &input.dify_app_id)?;
        get_app_view(&conn, stored.id)
    }
}

#[tauri::command]
async fn test_app_connection(app_config_id: i64, state: State<'_, AppState>) -> AppResult<Value> {
    let config = {
        let conn = open_db(&state)?;
        get_app_config(&conn, app_config_id)?
    };
    let token = decrypt_stored_token(&state, &config.encrypted_token)?;
    let client = http_client()?;
    let url = build_dify_url(
        &config.base_url,
        &config.dify_app_id,
        "/chat-conversations",
        &[("page", "1"), ("limit", "1"), ("sort_by", "-created_at")],
    )?;
    let value: Value = request_json(&client, &url, &token).await?;
    Ok(json!({
        "ok": true,
        "appConfigId": app_config_id,
        "sampleCount": value.get("data").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)
    }))
}

#[tauri::command]
async fn sync_app_logs(request: SyncRequest, state: State<'_, AppState>) -> AppResult<SyncSummary> {
    let sync_run_id = Uuid::new_v4().to_string();
    let mut counters = SyncCounters::default();
    let config = {
        let conn = open_db(&state)?;
        insert_sync_start(&conn, &sync_run_id, &request)?;
        get_app_config(&conn, request.app_config_id)?
    };

    let result = run_sync(&state, &config, &request, &mut counters).await;
    let mut conn = open_db(&state)?;
    match result {
        Ok(()) => {
            finish_sync(&mut conn, &sync_run_id, "success", None, &counters)?;
            Ok(SyncSummary {
                sync_run_id,
                status: "success".to_string(),
                conversations: counters.conversations,
                messages: counters.messages,
                workflow_runs: counters.workflow_runs,
                node_executions: counters.node_executions,
                error: None,
            })
        }
        Err(err) => {
            let status = if counters.conversations
                + counters.messages
                + counters.workflow_runs
                + counters.node_executions
                > 0
            {
                "partial_failed"
            } else {
                "failed"
            };
            finish_sync(&mut conn, &sync_run_id, status, Some(&err), &counters)?;
            Ok(SyncSummary {
                sync_run_id,
                status: status.to_string(),
                conversations: counters.conversations,
                messages: counters.messages,
                workflow_runs: counters.workflow_runs,
                node_executions: counters.node_executions,
                error: Some(err),
            })
        }
    }
}

#[tauri::command]
fn list_conversations(
    filters: ConversationFilters,
    state: State<AppState>,
) -> AppResult<Vec<ConversationRow>> {
    let conn = open_db(&state)?;
    let limit = filters.limit.unwrap_or(100).clamp(1, 500);
    let mut sql = String::from(
        "SELECT dify_conversation_id, app_config_id, name, summary, status, message_count,
                created_at, updated_at, synced_at
         FROM conversations WHERE 1 = 1",
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(app_config_id) = filters.app_config_id {
        sql.push_str(" AND app_config_id = ?");
        args.push(Box::new(app_config_id));
    }
    if let Some(status) = filters.status.filter(|v| !v.trim().is_empty()) {
        sql.push_str(" AND status = ?");
        args.push(Box::new(status));
    }
    if let Some(start) = parse_optional_i64(filters.start.as_deref())? {
        sql.push_str(" AND created_at >= ?");
        args.push(Box::new(start));
    }
    if let Some(end) = parse_optional_i64(filters.end.as_deref())? {
        sql.push_str(" AND created_at <= ?");
        args.push(Box::new(end));
    }
    if let Some(keyword) = filters.keyword.filter(|v| !v.trim().is_empty()) {
        sql.push_str(" AND (name LIKE ? OR summary LIKE ? OR dify_conversation_id LIKE ?)");
        let pattern = format!("%{}%", keyword.trim());
        args.push(Box::new(pattern.clone()));
        args.push(Box::new(pattern.clone()));
        args.push(Box::new(pattern));
    }
    sql.push_str(" ORDER BY COALESCE(created_at, 0) DESC LIMIT ?");
    args.push(Box::new(limit));

    let params = rusqlite::params_from_iter(args.iter().map(|v| &**v));
    let mut stmt = conn.prepare(&sql).map_err(to_string)?;
    let rows = stmt
        .query_map(params, |row| {
            Ok(ConversationRow {
                id: row.get(0)?,
                dify_conversation_id: row.get(0)?,
                app_config_id: row.get(1)?,
                name: row.get(2)?,
                summary: row.get(3)?,
                status: row.get(4)?,
                message_count: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                synced_at: row.get(8)?,
            })
        })
        .map_err(to_string)?;
    collect_rows(rows)
}

#[tauri::command]
fn get_conversation_detail(
    app_config_id: i64,
    conversation_id: String,
    state: State<AppState>,
) -> AppResult<ConversationDetail> {
    let conn = open_db(&state)?;
    let conversation_json: String = conn
        .query_row(
            "SELECT COALESCE(detail_json, raw_json)
             FROM conversations
             WHERE app_config_id = ?1 AND dify_conversation_id = ?2",
            params![app_config_id, conversation_id],
            |row| row.get(0),
        )
        .map_err(to_string)?;
    let conversation = serde_json::from_str(&conversation_json).map_err(to_string)?;
    let messages = query_json_values(
        &conn,
        "SELECT raw_json FROM messages
         WHERE app_config_id = ?1 AND dify_conversation_id = ?2
         ORDER BY COALESCE(created_at, 0) ASC",
        &[&app_config_id, &conversation_id],
    )?;
    let workflow_runs = query_json_values(
        &conn,
        "SELECT DISTINCT wr.raw_json FROM workflow_runs wr
         JOIN messages m
           ON m.app_config_id = wr.app_config_id
          AND m.workflow_run_id = wr.dify_workflow_run_id
         WHERE m.app_config_id = ?1 AND m.dify_conversation_id = ?2
         ORDER BY COALESCE(wr.created_at, 0) ASC",
        &[&app_config_id, &conversation_id],
    )?;
    let node_executions = query_json_values(
        &conn,
        "SELECT raw_json FROM node_executions
         WHERE app_config_id = ?1 AND conversation_id = ?2
         ORDER BY COALESCE(created_at, 0) ASC",
        &[&app_config_id, &conversation_id],
    )?;
    Ok(ConversationDetail {
        conversation,
        messages,
        workflow_runs,
        node_executions,
    })
}

#[tauri::command]
fn export_jsonl(request: ExportRequest, state: State<AppState>) -> AppResult<ExportSummary> {
    let conn = open_db(&state)?;
    let mut message_rows = 0;
    let mut node_rows = 0;
    validate_export_path(&request.output_path)?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&request.output_path)
        .map_err(to_string)?;
    let mut writer = BufWriter::new(file);

    let include_messages = request.granularity == "messages" || request.granularity == "both";
    let include_nodes = request.granularity == "nodes" || request.granularity == "both";
    if !include_messages && !include_nodes {
        return Err("granularity 必须是 messages、nodes 或 both".to_string());
    }

    if include_messages {
        message_rows = export_message_rows(&conn, &request, &mut writer)?;
    }
    if include_nodes {
        node_rows = export_node_rows(&conn, &request, &mut writer)?;
    }
    writer.flush().map_err(to_string)?;
    Ok(ExportSummary {
        output_path: request.output_path,
        message_rows,
        node_rows,
    })
}

async fn run_sync(
    state: &AppState,
    config: &StoredAppConfig,
    request: &SyncRequest,
    counters: &mut SyncCounters,
) -> AppResult<()> {
    let token = decrypt_stored_token(state, &config.encrypted_token)?;
    let client = http_client()?;
    let mut seen_workflow_runs = HashSet::new();
    let mut page = 1;
    let start_filter = request
        .start
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .map(format_time_for_dify)
        .transpose()?;
    let end_filter = request
        .end
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .map(format_time_for_dify)
        .transpose()?;

    loop {
        let page_s = page.to_string();
        let mut params = vec![
            ("page", page_s.as_str()),
            ("limit", "100"),
            ("sort_by", "-created_at"),
            ("annotation_status", "all"),
        ];
        if let Some(start) = start_filter.as_deref() {
            params.push(("start", start));
        }
        if let Some(end) = end_filter.as_deref() {
            params.push(("end", end));
        }
        let url = build_dify_url(
            &config.base_url,
            &config.dify_app_id,
            "/chat-conversations",
            &params,
        )?;
        let payload: ListPage = request_json(&client, &url, &token).await?;

        if payload.data.is_empty() {
            break;
        }

        for conversation in payload.data {
            let conversation_id = value_string(&conversation, "id")
                .ok_or_else(|| "会话列表返回缺少 id".to_string())?;
            let detail =
                fetch_conversation_detail(&client, config, &token, &conversation_id).await?;
            let conn = open_db(state)?;
            upsert_conversation(&conn, config.id, &conversation, Some(&detail))?;
            counters.conversations += 1;
            drop(conn);

            let messages = fetch_all_messages(&client, config, &token, &conversation_id).await?;
            for message in messages {
                let message_id =
                    value_string(&message, "id").ok_or_else(|| "消息返回缺少 id".to_string())?;
                let workflow_run_id = value_string(&message, "workflow_run_id");
                let conn = open_db(state)?;
                upsert_message(&conn, config.id, &conversation_id, &message)?;
                counters.messages += 1;
                drop(conn);

                if let Some(run_id) = workflow_run_id {
                    if !seen_workflow_runs.insert(run_id.clone()) {
                        continue;
                    }
                    let run = fetch_workflow_run(&client, config, &token, &run_id).await?;
                    let nodes = fetch_node_executions(&client, config, &token, &run_id).await?;
                    let conn = open_db(state)?;
                    upsert_workflow_run(&conn, config.id, &run)?;
                    counters.workflow_runs += 1;
                    for node in nodes {
                        upsert_node_execution(
                            &conn,
                            config.id,
                            &run_id,
                            Some(&message_id),
                            Some(&conversation_id),
                            &node,
                        )?;
                        counters.node_executions += 1;
                    }
                }
            }
        }

        if !payload.has_more {
            break;
        }
        page += 1;
    }

    Ok(())
}

async fn fetch_conversation_detail(
    client: &Client,
    config: &StoredAppConfig,
    token: &str,
    conversation_id: &str,
) -> AppResult<Value> {
    let path = format!("/chat-conversations/{conversation_id}");
    let url = build_dify_url(&config.base_url, &config.dify_app_id, &path, &[])?;
    request_json(client, &url, token).await
}

async fn fetch_all_messages(
    client: &Client,
    config: &StoredAppConfig,
    token: &str,
    conversation_id: &str,
) -> AppResult<Vec<Value>> {
    let mut all = Vec::new();
    let mut first_id: Option<String> = None;
    loop {
        let mut owned_params = vec![
            ("conversation_id".to_string(), conversation_id.to_string()),
            ("limit".to_string(), "100".to_string()),
        ];
        if let Some(id) = first_id.as_ref() {
            owned_params.push(("first_id".to_string(), id.clone()));
        }
        let borrowed: Vec<(&str, &str)> = owned_params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let url = build_dify_url(
            &config.base_url,
            &config.dify_app_id,
            "/chat-messages",
            &borrowed,
        )?;
        let payload: ListPage = request_json(client, &url, token).await?;
        if payload.data.is_empty() {
            break;
        }
        let next_first_id = payload.data.first().and_then(|m| value_string(m, "id"));
        all.extend(payload.data);
        if !payload.has_more {
            break;
        }
        let Some(next_first_id) = next_first_id else {
            return Err("消息分页返回 has_more=true 但缺少 first_id".to_string());
        };
        if first_id.as_deref() == Some(next_first_id.as_str()) {
            return Err("消息分页未前进，已停止以避免重复拉取".to_string());
        }
        first_id = Some(next_first_id);
    }
    Ok(all)
}

async fn fetch_workflow_run(
    client: &Client,
    config: &StoredAppConfig,
    token: &str,
    run_id: &str,
) -> AppResult<Value> {
    let path = format!("/workflow-runs/{run_id}");
    let url = build_dify_url(&config.base_url, &config.dify_app_id, &path, &[])?;
    request_json(client, &url, token).await
}

async fn fetch_node_executions(
    client: &Client,
    config: &StoredAppConfig,
    token: &str,
    run_id: &str,
) -> AppResult<Vec<Value>> {
    let path = format!("/workflow-runs/{run_id}/node-executions");
    let url = build_dify_url(&config.base_url, &config.dify_app_id, &path, &[])?;
    let payload: ListPage = request_json(client, &url, token).await?;
    Ok(payload.data)
}

async fn request_json<T: for<'de> Deserialize<'de>>(
    client: &Client,
    url: &str,
    token: &str,
) -> AppResult<T> {
    let response = client
        .get(url)
        .bearer_auth(token)
        .header("accept", "application/json")
        .send()
        .await
        .map_err(to_string)?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Dify API 请求失败: {status} {text}"));
    }
    response.json::<T>().await.map_err(to_string)
}

fn build_dify_url(
    base_url: &str,
    app_id: &str,
    api_path: &str,
    params: &[(&str, &str)],
) -> AppResult<String> {
    let base = normalize_base_url(base_url)?;
    let clean_path = api_path.trim_start_matches('/');
    let mut url = Url::parse(&format!(
        "{base}/console/api/apps/{}/{}",
        app_id.trim(),
        clean_path
    ))
    .map_err(to_string)?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            query.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}

fn http_client() -> AppResult<Client> {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(to_string)
}

fn normalize_base_url(base_url: &str) -> AppResult<String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    let url = Url::parse(trimmed)
        .map_err(|_| "base_url 必须是完整 URL，例如 https://example.com".to_string())?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("base_url 只支持 http 或 https".to_string());
    }
    Ok(trimmed.to_string())
}

fn validate_app_config(input: &AppConfigInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err("应用名称不能为空".to_string());
    }
    if input.dify_app_id.trim().is_empty() {
        return Err("Dify app_id 不能为空".to_string());
    }
    normalize_base_url(&input.base_url)?;
    Ok(())
}

fn validate_export_path(output_path: &str) -> AppResult<()> {
    let path = Path::new(output_path);
    if output_path.trim().is_empty() {
        return Err("输出路径不能为空".to_string());
    }
    if !path.is_absolute() {
        return Err("输出路径必须是绝对路径".to_string());
    }
    if path.exists() {
        return Err("输出文件已存在，请换一个路径，避免覆盖已有文件".to_string());
    }
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err("输出目录不存在".to_string());
        }
    }
    Ok(())
}

fn load_or_create_master_key(state: &AppState) -> AppResult<Vec<u8>> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        if let Ok(password) = entry.get_password() {
            return B64.decode(password).map_err(to_string);
        }
        let key = random_key();
        let encoded = B64.encode(&key);
        if entry.set_password(&encoded).is_ok() {
            return Ok(key);
        }
    }

    if state.key_fallback_path.exists() {
        let encoded = fs::read_to_string(&state.key_fallback_path).map_err(to_string)?;
        return B64.decode(encoded.trim()).map_err(to_string);
    }
    let key = random_key();
    write_private_key_file(&state.key_fallback_path, &B64.encode(&key))?;
    Ok(key)
}

fn write_private_key_file(path: &Path, content: &str) -> AppResult<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(to_string)?;
    file.write_all(content.as_bytes()).map_err(to_string)?;
    Ok(())
}

fn random_key() -> Vec<u8> {
    let mut key = vec![0_u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

fn encrypt_token(token: &str, key: &[u8]) -> AppResult<String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(to_string)?;
    let mut nonce_bytes = [0_u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), token.as_bytes())
        .map_err(to_string)?;
    Ok(json!({
        "v": 1,
        "nonce": B64.encode(nonce_bytes),
        "ciphertext": B64.encode(ciphertext)
    })
    .to_string())
}

fn decrypt_token(payload: &str, key: &[u8]) -> AppResult<String> {
    let value: Value = serde_json::from_str(payload).map_err(to_string)?;
    let nonce = value
        .get("nonce")
        .and_then(Value::as_str)
        .ok_or_else(|| "加密 token 缺少 nonce".to_string())
        .and_then(|v| B64.decode(v).map_err(to_string))?;
    let ciphertext = value
        .get("ciphertext")
        .and_then(Value::as_str)
        .ok_or_else(|| "加密 token 缺少 ciphertext".to_string())
        .and_then(|v| B64.decode(v).map_err(to_string))?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(to_string)?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(to_string)?;
    String::from_utf8(plaintext).map_err(to_string)
}

fn decrypt_stored_token(state: &AppState, encrypted_token: &str) -> AppResult<String> {
    let key = load_or_create_master_key(state)?;
    decrypt_token(encrypted_token, &key)
}

fn get_app_view(conn: &Connection, id: i64) -> AppResult<AppConfigView> {
    conn.query_row(
        "SELECT id, name, base_url, dify_app_id, encrypted_token, created_at, updated_at
         FROM apps WHERE id = ?1",
        params![id],
        |row| {
            Ok(AppConfigView {
                id: row.get(0)?,
                name: row.get(1)?,
                base_url: row.get(2)?,
                dify_app_id: row.get(3)?,
                token_configured: !row.get::<_, String>(4)?.is_empty(),
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    )
    .map_err(to_string)
}

fn get_app_config(conn: &Connection, id: i64) -> AppResult<StoredAppConfig> {
    conn.query_row(
        "SELECT id, base_url, dify_app_id, encrypted_token FROM apps WHERE id = ?1",
        params![id],
        |row| {
            Ok(StoredAppConfig {
                id: row.get(0)?,
                base_url: row.get(1)?,
                dify_app_id: row.get(2)?,
                encrypted_token: row.get(3)?,
            })
        },
    )
    .map_err(to_string)
}

fn get_app_by_identity(
    conn: &Connection,
    base_url: &str,
    app_id: &str,
) -> AppResult<StoredAppConfig> {
    conn.query_row(
        "SELECT id, base_url, dify_app_id, encrypted_token
         FROM apps WHERE base_url = ?1 AND dify_app_id = ?2",
        params![normalize_base_url(base_url)?, app_id.trim()],
        |row| {
            Ok(StoredAppConfig {
                id: row.get(0)?,
                base_url: row.get(1)?,
                dify_app_id: row.get(2)?,
                encrypted_token: row.get(3)?,
            })
        },
    )
    .map_err(to_string)
}

fn insert_sync_start(conn: &Connection, id: &str, request: &SyncRequest) -> AppResult<()> {
    conn.execute(
        "INSERT INTO sync_runs (id, app_config_id, start_filter, end_filter, status, started_at)
         VALUES (?1, ?2, ?3, ?4, 'running', ?5)",
        params![
            id,
            request.app_config_id,
            request.start,
            request.end,
            now_string()
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn finish_sync(
    conn: &mut Connection,
    id: &str,
    status: &str,
    error: Option<&str>,
    counters: &SyncCounters,
) -> AppResult<()> {
    conn.execute(
        "UPDATE sync_runs
         SET status = ?1, error = ?2, conversations_count = ?3, messages_count = ?4,
             workflow_runs_count = ?5, node_executions_count = ?6, finished_at = ?7
         WHERE id = ?8",
        params![
            status,
            error,
            counters.conversations as i64,
            counters.messages as i64,
            counters.workflow_runs as i64,
            counters.node_executions as i64,
            now_string(),
            id
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn upsert_conversation(
    conn: &Connection,
    app_config_id: i64,
    raw: &Value,
    detail: Option<&Value>,
) -> AppResult<()> {
    let id = value_string(raw, "id").ok_or_else(|| "会话缺少 id".to_string())?;
    conn.execute(
        "INSERT INTO conversations
         (dify_conversation_id, app_config_id, name, summary, status, message_count, created_at,
          updated_at, raw_json, detail_json, synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(app_config_id, dify_conversation_id) DO UPDATE SET
           name = excluded.name,
           summary = excluded.summary,
           status = excluded.status,
           message_count = excluded.message_count,
           created_at = excluded.created_at,
           updated_at = excluded.updated_at,
           raw_json = excluded.raw_json,
           detail_json = excluded.detail_json,
           synced_at = excluded.synced_at",
        params![
            id,
            app_config_id,
            value_string(raw, "name"),
            value_string(raw, "summary"),
            value_string(raw, "status"),
            value_i64(raw, "message_count"),
            value_i64(raw, "created_at"),
            value_i64(raw, "updated_at"),
            raw.to_string(),
            detail.map(Value::to_string),
            now_string()
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn upsert_message(
    conn: &Connection,
    app_config_id: i64,
    conversation_id: &str,
    raw: &Value,
) -> AppResult<()> {
    let id = value_string(raw, "id").ok_or_else(|| "消息缺少 id".to_string())?;
    conn.execute(
        "INSERT INTO messages
         (dify_message_id, app_config_id, dify_conversation_id, workflow_run_id, query,
          answer, status, created_at, raw_json, synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(app_config_id, dify_message_id) DO UPDATE SET
           dify_conversation_id = excluded.dify_conversation_id,
           workflow_run_id = excluded.workflow_run_id,
           query = excluded.query,
           answer = excluded.answer,
           status = excluded.status,
           created_at = excluded.created_at,
           raw_json = excluded.raw_json,
           synced_at = excluded.synced_at",
        params![
            id,
            app_config_id,
            conversation_id,
            value_string(raw, "workflow_run_id"),
            value_string(raw, "query"),
            value_string(raw, "answer"),
            value_string(raw, "status"),
            value_i64(raw, "created_at"),
            raw.to_string(),
            now_string()
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn upsert_workflow_run(conn: &Connection, app_config_id: i64, raw: &Value) -> AppResult<()> {
    let id = value_string(raw, "id").ok_or_else(|| "workflow run 缺少 id".to_string())?;
    conn.execute(
        "INSERT INTO workflow_runs
         (dify_workflow_run_id, app_config_id, status, created_at, finished_at, raw_json, synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(app_config_id, dify_workflow_run_id) DO UPDATE SET
           status = excluded.status,
           created_at = excluded.created_at,
           finished_at = excluded.finished_at,
           raw_json = excluded.raw_json,
           synced_at = excluded.synced_at",
        params![
            id,
            app_config_id,
            value_string(raw, "status"),
            value_i64(raw, "created_at"),
            value_i64(raw, "finished_at"),
            raw.to_string(),
            now_string()
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn upsert_node_execution(
    conn: &Connection,
    app_config_id: i64,
    workflow_run_id: &str,
    message_id: Option<&str>,
    conversation_id: Option<&str>,
    raw: &Value,
) -> AppResult<()> {
    let id = value_string(raw, "id").ok_or_else(|| "节点执行缺少 id".to_string())?;
    conn.execute(
        "INSERT INTO node_executions
         (dify_node_execution_id, app_config_id, workflow_run_id, message_id, conversation_id,
          node_id, node_type, title, status, created_at, raw_json, synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(app_config_id, dify_node_execution_id) DO UPDATE SET
           workflow_run_id = excluded.workflow_run_id,
           message_id = excluded.message_id,
           conversation_id = excluded.conversation_id,
           node_id = excluded.node_id,
           node_type = excluded.node_type,
           title = excluded.title,
           status = excluded.status,
           created_at = excluded.created_at,
           raw_json = excluded.raw_json,
           synced_at = excluded.synced_at",
        params![
            id,
            app_config_id,
            workflow_run_id,
            message_id,
            conversation_id,
            value_string(raw, "node_id"),
            value_string(raw, "node_type"),
            value_string(raw, "title"),
            value_string(raw, "status"),
            value_i64(raw, "created_at"),
            raw.to_string(),
            now_string()
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn export_message_rows(
    conn: &Connection,
    request: &ExportRequest,
    writer: &mut BufWriter<File>,
) -> AppResult<u64> {
    let mut sql = String::from(
        "SELECT a.name, c.raw_json, m.raw_json
         FROM messages m
         JOIN conversations c
           ON c.app_config_id = m.app_config_id
          AND c.dify_conversation_id = m.dify_conversation_id
         JOIN apps a ON a.id = m.app_config_id
         WHERE m.app_config_id = ?",
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(request.app_config_id)];
    append_time_filters(
        &mut sql,
        &mut args,
        "m.created_at",
        request.start.as_deref(),
        request.end.as_deref(),
    )?;
    sql.push_str(" ORDER BY COALESCE(m.created_at, 0) ASC");

    let params = rusqlite::params_from_iter(args.iter().map(|v| &**v));
    let mut stmt = conn.prepare(&sql).map_err(to_string)?;
    let mut rows = stmt.query(params).map_err(to_string)?;
    let mut count = 0;
    while let Some(row) = rows.next().map_err(to_string)? {
        let app_name: String = row.get(0).map_err(to_string)?;
        let conversation: Value =
            serde_json::from_str(&row.get::<_, String>(1).map_err(to_string)?)
                .map_err(to_string)?;
        let message: Value = serde_json::from_str(&row.get::<_, String>(2).map_err(to_string)?)
            .map_err(to_string)?;
        let line = json!({
            "granularity": "message",
            "app": {"id": request.app_config_id, "name": app_name},
            "conversation": {
                "id": value_string(&conversation, "id"),
                "name": value_string(&conversation, "name"),
                "summary": value_string(&conversation, "summary")
            },
            "message": {
                "id": value_string(&message, "id"),
                "query": message.get("query").cloned().unwrap_or(Value::Null),
                "answer": message.get("answer").cloned().unwrap_or(Value::Null),
                "inputs": message.get("inputs").cloned().unwrap_or(Value::Null),
                "metadata": message.get("metadata").cloned().unwrap_or(Value::Null),
                "workflow_run_id": value_string(&message, "workflow_run_id"),
                "created_at": value_i64(&message, "created_at"),
                "status": value_string(&message, "status")
            },
            "raw": {"conversation": conversation, "message": message}
        });
        writeln!(writer, "{line}").map_err(to_string)?;
        count += 1;
    }
    Ok(count)
}

fn export_node_rows(
    conn: &Connection,
    request: &ExportRequest,
    writer: &mut BufWriter<File>,
) -> AppResult<u64> {
    let mut sql = String::from(
        "SELECT a.name, c.raw_json, m.raw_json, n.raw_json
         FROM node_executions n
         LEFT JOIN messages m
           ON m.app_config_id = n.app_config_id
          AND m.dify_message_id = n.message_id
         LEFT JOIN conversations c
           ON c.app_config_id = n.app_config_id
          AND c.dify_conversation_id = n.conversation_id
         JOIN apps a ON a.id = n.app_config_id
         WHERE n.app_config_id = ? AND n.node_type IN ('llm', 'agent')",
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(request.app_config_id)];
    append_time_filters(
        &mut sql,
        &mut args,
        "n.created_at",
        request.start.as_deref(),
        request.end.as_deref(),
    )?;
    sql.push_str(" ORDER BY COALESCE(n.created_at, 0) ASC");

    let params = rusqlite::params_from_iter(args.iter().map(|v| &**v));
    let mut stmt = conn.prepare(&sql).map_err(to_string)?;
    let mut rows = stmt.query(params).map_err(to_string)?;
    let mut count = 0;
    while let Some(row) = rows.next().map_err(to_string)? {
        let app_name: String = row.get(0).map_err(to_string)?;
        let conversation = optional_json(row.get::<_, Option<String>>(1).map_err(to_string)?)?;
        let message = optional_json(row.get::<_, Option<String>>(2).map_err(to_string)?)?;
        let node: Value = serde_json::from_str(&row.get::<_, String>(3).map_err(to_string)?)
            .map_err(to_string)?;
        let line = json!({
            "granularity": "node",
            "app": {"id": request.app_config_id, "name": app_name},
            "conversation": conversation.as_ref().map(|v| json!({
                "id": value_string(v, "id"),
                "name": value_string(v, "name"),
                "summary": value_string(v, "summary")
            })).unwrap_or(Value::Null),
            "message": message.as_ref().map(|v| json!({
                "id": value_string(v, "id"),
                "query": v.get("query").cloned().unwrap_or(Value::Null),
                "answer": v.get("answer").cloned().unwrap_or(Value::Null)
            })).unwrap_or(Value::Null),
            "node": {
                "execution_id": value_string(&node, "id"),
                "node_id": value_string(&node, "node_id"),
                "node_type": value_string(&node, "node_type"),
                "title": value_string(&node, "title"),
                "inputs": node.get("inputs").cloned().unwrap_or(Value::Null),
                "process_data": node.get("process_data").cloned().unwrap_or(Value::Null),
                "outputs": node.get("outputs").cloned().unwrap_or(Value::Null),
                "execution_metadata": node.get("execution_metadata").cloned().unwrap_or(Value::Null),
                "created_at": value_i64(&node, "created_at"),
                "status": value_string(&node, "status")
            },
            "raw": {"conversation": conversation, "message": message, "node": node}
        });
        writeln!(writer, "{line}").map_err(to_string)?;
        count += 1;
    }
    Ok(count)
}

fn append_time_filters(
    sql: &mut String,
    args: &mut Vec<Box<dyn rusqlite::ToSql>>,
    column: &str,
    start: Option<&str>,
    end: Option<&str>,
) -> AppResult<()> {
    if let Some(value) = parse_optional_i64(start)? {
        sql.push_str(&format!(" AND {column} >= ?"));
        args.push(Box::new(value));
    }
    if let Some(value) = parse_optional_i64(end)? {
        sql.push_str(&format!(" AND {column} <= ?"));
        args.push(Box::new(value));
    }
    Ok(())
}

fn query_json_values(
    conn: &Connection,
    sql: &str,
    args: &[&dyn rusqlite::ToSql],
) -> AppResult<Vec<Value>> {
    let mut stmt = conn.prepare(sql).map_err(to_string)?;
    let rows = stmt
        .query_map(args, |row| row.get::<_, String>(0))
        .map_err(to_string)?;
    let values = collect_rows(rows)?;
    values
        .into_iter()
        .map(|text| serde_json::from_str(&text).map_err(to_string))
        .collect()
}

fn optional_json(value: Option<String>) -> AppResult<Option<Value>> {
    value
        .map(|text| serde_json::from_str(&text).map_err(to_string))
        .transpose()
}

fn collect_rows<T, F>(rows: rusqlite::MappedRows<'_, F>) -> AppResult<Vec<T>>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(to_string)
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .filter(|v| !v.is_empty())
}

fn value_i64(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn format_time_for_dify(value: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if let Ok(timestamp) = trimmed.parse::<i64>() {
        return Local
            .timestamp_opt(timestamp, 0)
            .single()
            .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
            .ok_or_else(|| format!("Unix 时间戳无效: {timestamp}"));
    }
    let normalized = trimmed.replace('T', " ");
    let parsed = parse_local_datetime(&normalized)?;
    Ok(parsed.format("%Y-%m-%d %H:%M").to_string())
}

fn parse_optional_i64(value: Option<&str>) -> AppResult<Option<i64>> {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => parse_time_to_unix(v).map(Some),
        None => Ok(None),
    }
}

fn parse_time_to_unix(value: &str) -> AppResult<i64> {
    let trimmed = value.trim();
    if let Ok(timestamp) = trimmed.parse::<i64>() {
        return Ok(timestamp);
    }
    let normalized = trimmed.replace('T', " ");
    let parsed = parse_local_datetime(&normalized)?;
    Local
        .from_local_datetime(&parsed)
        .single()
        .map(|value| value.timestamp())
        .ok_or_else(|| format!("本地时间无效或不唯一: {value}"))
}

fn parse_local_datetime(value: &str) -> AppResult<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S"))
        .map_err(|_| {
            format!("时间格式必须是 YYYY-MM-DD HH:MM、YYYY-MM-DDTHH:MM 或 Unix 秒: {value}")
        })
}

fn now_string() -> String {
    Utc::now().to_rfc3339()
}

fn to_string<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn build_dify_url_encodes_query_params() {
        let url = build_dify_url(
            "https://example.com/",
            "app-1",
            "/chat-conversations",
            &[("page", "1"), ("start", "2026-05-22 00:00")],
        )
        .unwrap();
        assert_eq!(
            url,
            "https://example.com/console/api/apps/app-1/chat-conversations?page=1&start=2026-05-22+00%3A00"
        );
    }

    #[test]
    fn token_encrypt_roundtrip_and_wrong_key_fails() {
        let key = vec![7_u8; 32];
        let encrypted = encrypt_token("secret-token", &key).unwrap();
        assert_eq!(decrypt_token(&encrypted, &key).unwrap(), "secret-token");
        assert!(decrypt_token(&encrypted, &[8_u8; 32]).is_err());
    }

    #[test]
    fn node_export_filters_llm_and_agent_only() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("test.sqlite");
        init_db(&db).unwrap();
        let conn = Connection::open(db).unwrap();
        conn.execute(
            "INSERT INTO apps (id, name, base_url, dify_app_id, encrypted_token, created_at, updated_at)
             VALUES (1, 'A', 'https://example.com', 'app', '{}', 'now', 'now')",
            [],
        )
        .unwrap();
        let conv = json!({"id":"c1","name":"C"});
        let msg = json!({"id":"m1","query":"q","answer":"a","workflow_run_id":"r1"});
        upsert_conversation(&conn, 1, &conv, None).unwrap();
        upsert_message(&conn, 1, "c1", &msg).unwrap();
        upsert_node_execution(
            &conn,
            1,
            "r1",
            Some("m1"),
            Some("c1"),
            &json!({"id":"n1","node_type":"llm","node_id":"llm1"}),
        )
        .unwrap();
        upsert_node_execution(
            &conn,
            1,
            "r1",
            Some("m1"),
            Some("c1"),
            &json!({"id":"n2","node_type":"code","node_id":"code1"}),
        )
        .unwrap();
        let request = ExportRequest {
            app_config_id: 1,
            start: None,
            end: None,
            granularity: "nodes".to_string(),
            output_path: dir.path().join("nodes.jsonl").to_string_lossy().to_string(),
        };
        let file = File::create(&request.output_path).unwrap();
        let mut writer = BufWriter::new(file);
        assert_eq!(export_node_rows(&conn, &request, &mut writer).unwrap(), 1);
    }
}
