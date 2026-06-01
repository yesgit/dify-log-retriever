use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;

use crate::models::*;

fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    format!("{}****{}", &key[..4], &key[key.len() - 4..])
}

fn parse_json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap_or(serde_json::Value::Null)
}

fn bool_int(v: bool) -> i64 {
    if v { 1 } else { 0 }
}

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS apps (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT DEFAULT '',
                mode TEXT DEFAULT '',
                icon TEXT DEFAULT '',
                icon_background TEXT DEFAULT '',
                created_at INTEGER DEFAULT 0,
                synced_at INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                app_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                name TEXT DEFAULT '',
                inputs TEXT DEFAULT '{}',
                status TEXT DEFAULT '',
                introduction TEXT DEFAULT '',
                created_at INTEGER DEFAULT 0,
                updated_at INTEGER DEFAULT 0,
                synced_at INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                app_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                message_id TEXT NOT NULL,
                query TEXT DEFAULT '',
                answer TEXT DEFAULT '',
                feedback TEXT,
                retriever_resources TEXT DEFAULT '[]',
                message_metadata TEXT DEFAULT '{}',
                agent_thoughts TEXT DEFAULT '[]',
                answer_tokens INTEGER DEFAULT 0,
                prompt_tokens INTEGER DEFAULT 0,
                elapsed_time REAL DEFAULT 0.0,
                created_at INTEGER DEFAULT 0,
                synced_at INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS workflow_runs (
                id TEXT PRIMARY KEY,
                app_id TEXT NOT NULL,
                workflow_run_id TEXT NOT NULL,
                workflow_id TEXT DEFAULT '',
                status TEXT DEFAULT '',
                version TEXT DEFAULT '',
                graph TEXT DEFAULT '{}',
                elapsed_time REAL DEFAULT 0.0,
                total_tokens INTEGER DEFAULT 0,
                total_steps INTEGER DEFAULT 0,
                created_at INTEGER DEFAULT 0,
                finished_at INTEGER DEFAULT 0,
                raw_json TEXT DEFAULT '{}',
                synced_at INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS node_executions (
                id TEXT PRIMARY KEY,
                app_id TEXT NOT NULL,
                workflow_run_id TEXT NOT NULL,
                execution_id TEXT NOT NULL,
                execution_index INTEGER DEFAULT 0,
                node_id TEXT DEFAULT '',
                node_type TEXT DEFAULT '',
                title TEXT DEFAULT '',
                inputs TEXT DEFAULT '{}',
                process_data TEXT DEFAULT '{}',
                outputs TEXT DEFAULT '{}',
                execution_metadata TEXT DEFAULT '{}',
                extras TEXT DEFAULT '{}',
                status TEXT DEFAULT '',
                error TEXT DEFAULT 'null',
                elapsed_time REAL DEFAULT 0.0,
                created_at INTEGER DEFAULT 0,
                finished_at INTEGER DEFAULT 0,
                raw_json TEXT DEFAULT '{}',
                synced_at INTEGER DEFAULT 0
            );
            ",
        )
        .map_err(|e| e.to_string())?;

        self.add_columns(&conn)?;

        conn.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_conversations_app_id ON conversations(app_id);
            CREATE INDEX IF NOT EXISTS idx_conversations_conversation_id ON conversations(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_conversations_created_at ON conversations(created_at);
            CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_messages_app_id ON messages(app_id);
            CREATE INDEX IF NOT EXISTS idx_messages_message_id ON messages(message_id);
            CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);
            CREATE INDEX IF NOT EXISTS idx_messages_workflow_run_id ON messages(workflow_run_id);
            CREATE INDEX IF NOT EXISTS idx_messages_feedback ON messages(feedback);
            CREATE INDEX IF NOT EXISTS idx_workflow_runs_app_run ON workflow_runs(app_id, workflow_run_id);
            CREATE INDEX IF NOT EXISTS idx_node_executions_app_run ON node_executions(app_id, workflow_run_id);
            ",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn add_columns(&self, conn: &Connection) -> Result<(), String> {
        let conversation_columns = [
            ("summary", "TEXT DEFAULT ''"),
            ("from_source", "TEXT DEFAULT ''"),
            ("from_end_user_id", "TEXT DEFAULT ''"),
            ("from_end_user_session_id", "TEXT DEFAULT ''"),
            ("read_at", "INTEGER"),
            ("annotated", "INTEGER DEFAULT 0"),
            ("model_config", "TEXT DEFAULT '{}'"),
            ("user_feedback_stats", "TEXT DEFAULT '{}'"),
            ("admin_feedback_stats", "TEXT DEFAULT '{}'"),
            ("status_count", "TEXT DEFAULT '{}'"),
            ("raw_json", "TEXT DEFAULT '{}'"),
        ];
        for (name, def) in conversation_columns {
            self.add_column(conn, "conversations", name, def)?;
        }

        let message_columns = [
            ("workflow_run_id", "TEXT"),
            ("inputs", "TEXT DEFAULT '{}'"),
            ("message_tokens", "INTEGER DEFAULT 0"),
            ("provider_response_latency", "REAL DEFAULT 0.0"),
            ("feedbacks", "TEXT DEFAULT '[]'"),
            ("annotation", "TEXT DEFAULT 'null'"),
            ("annotation_hit_history", "TEXT DEFAULT 'null'"),
            ("message_files", "TEXT DEFAULT '[]'"),
            ("status", "TEXT DEFAULT ''"),
            ("error", "TEXT DEFAULT 'null'"),
            ("parent_message_id", "TEXT DEFAULT ''"),
            ("raw_json", "TEXT DEFAULT '{}'"),
        ];
        for (name, def) in message_columns {
            self.add_column(conn, "messages", name, def)?;
        }
        Ok(())
    }

    fn add_column(&self, conn: &Connection, table: &str, column: &str, definition: &str) -> Result<(), String> {
        let exists: bool = conn
            .prepare(&format!("PRAGMA table_info({})", table))
            .map_err(|e| e.to_string())?
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .any(|name| name == column);

        if !exists {
            conn.execute(
                &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition),
                [],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // ===== Settings =====
    pub fn save_config(&self, config: &DifyConfig) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["api_base", &config.api_base],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["api_key", &config.api_key],
        )
        .map_err(|e| e.to_string())?;
        if let Some(ref proxy) = config.proxy {
            let trimmed = proxy.trim();
            if !trimmed.is_empty() {
                conn.execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                    params!["proxy", trimmed],
                )
                .map_err(|e| e.to_string())?;
            } else {
                conn.execute("DELETE FROM settings WHERE key = 'proxy'", [])
                    .map_err(|e| e.to_string())?;
            }
        } else {
            conn.execute("DELETE FROM settings WHERE key = 'proxy'", [])
                .map_err(|e| e.to_string())?;
        }
        // Save auth credentials
        if let Some(ref email) = config.auth_email {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params!["auth_email", email],
            )
            .map_err(|e| e.to_string())?;
        } else {
            conn.execute("DELETE FROM settings WHERE key = 'auth_email'", [])
                .map_err(|e| e.to_string())?;
        }
        if let Some(ref password) = config.auth_password {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params!["auth_password", password],
            )
            .map_err(|e| e.to_string())?;
        } else {
            conn.execute("DELETE FROM settings WHERE key = 'auth_password'", [])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn get_config(&self) -> Result<Option<DifyConfig>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let api_base: Option<String> = conn
            .query_row("SELECT value FROM settings WHERE key = 'api_base'", [], |row| row.get(0))
            .ok();
        let api_key: Option<String> = conn
            .query_row("SELECT value FROM settings WHERE key = 'api_key'", [], |row| row.get(0))
            .ok();
        let proxy: Option<String> = conn
            .query_row("SELECT value FROM settings WHERE key = 'proxy'", [], |row| row.get(0))
            .ok();
        let auth_email: Option<String> = conn
            .query_row("SELECT value FROM settings WHERE key = 'auth_email'", [], |row| row.get(0))
            .ok();
        let auth_password: Option<String> = conn
            .query_row("SELECT value FROM settings WHERE key = 'auth_password'", [], |row| row.get(0))
            .ok();

        match (api_base, api_key) {
            (Some(base), Some(key)) => Ok(Some(DifyConfig {
                api_base: base,
                api_key: key,
                proxy,
                auth_email,
                auth_password,
            })),
            _ => Ok(None),
        }
    }

    pub fn get_config_display(&self) -> Result<Option<DifyConfigDisplay>, String> {
        let config = self.get_config()?;
        Ok(config.map(|c| {
            let auth_mode = if c.auth_email.is_some() && c.auth_password.is_some() {
                "login".to_string()
            } else {
                "token".to_string()
            };
            DifyConfigDisplay {
                api_base: c.api_base,
                api_key_masked: mask_api_key(&c.api_key),
                proxy: c.proxy,
                has_key: !c.api_key.is_empty(),
                auth_mode,
                auth_email: c.auth_email,
            }
        }))
    }

    /// Update only the API key in the config (used for auto-refresh after login)
    pub fn update_api_key(&self, new_key: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["api_key", new_key],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Apps =====
    pub fn upsert_app(&self, app: &DifyApp) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO apps (id, name, description, mode, icon, icon_background, created_at, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%s','now'))",
            params![app.id, app.name, app.description, app.mode, app.icon, app.icon_background, app.created_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_apps(&self) -> Result<Vec<DifyApp>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, mode, icon, icon_background, created_at FROM apps ORDER BY name")
            .map_err(|e| e.to_string())?;
        let apps = stmt
            .query_map([], |row| {
                Ok(DifyApp {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    mode: row.get(3)?,
                    icon: row.get(4)?,
                    icon_background: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(apps)
    }

    pub fn delete_app_data(&self, app_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM node_executions WHERE app_id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM workflow_runs WHERE app_id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM messages WHERE app_id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM conversations WHERE app_id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM apps WHERE id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Incremental Sync Helpers =====
    pub fn get_conversations_updated_at(&self, app_id: &str, conversation_ids: &[String]) -> Result<std::collections::HashMap<String, i64>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut result = std::collections::HashMap::new();
        for cid in conversation_ids {
            let key = format!("{}:{}", app_id, cid);
            let updated: Option<i64> = conn
                .query_row(
                    "SELECT updated_at FROM conversations WHERE id = ?1",
                    params![key],
                    |row| row.get(0),
                )
                .ok();
            if let Some(ts) = updated {
                result.insert(cid.clone(), ts);
            }
        }
        Ok(result)
    }

    // ===== Conversations =====
    pub fn upsert_conversation(&self, app_id: &str, conv: &DifyConversationItem) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let id = format!("{}:{}", app_id, conv.id);
        conn.execute(
            "INSERT OR REPLACE INTO conversations (
                id, app_id, conversation_id, name, inputs, status, introduction, summary,
                from_source, from_end_user_id, from_end_user_session_id, read_at, annotated,
                model_config, user_feedback_stats, admin_feedback_stats, status_count,
                raw_json, created_at, updated_at, synced_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, strftime('%s','now'))",
            params![
                id,
                app_id,
                conv.id,
                conv.name,
                json_string(&conv.inputs, "{}"),
                conv.status,
                conv.introduction,
                conv.summary,
                conv.from_source,
                conv.from_end_user_id,
                conv.from_end_user_session_id,
                conv.read_at,
                bool_int(conv.annotated),
                json_string(&conv.model_config, "{}"),
                json_string(&conv.user_feedback_stats, "{}"),
                json_string(&conv.admin_feedback_stats, "{}"),
                json_string(&conv.status_count, "{}"),
                json_string(&conv.raw_json, "{}"),
                conv.created_at,
                conv.updated_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_conversations(
        &self,
        app_id: Option<&str>,
        keyword: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<ConversationsResult, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let offset = (page - 1) * page_size;

        let (where_clause, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = {
            let mut conditions = vec!["1=1".to_string()];
            let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            if let Some(aid) = app_id {
                conditions.push("c.app_id = ?".to_string());
                params.push(Box::new(aid.to_string()));
            }
            if let Some(kw) = keyword {
                if !kw.is_empty() {
                    conditions.push("(c.name LIKE ? OR c.summary LIKE ? OR c.conversation_id LIKE ?)".to_string());
                    let pattern = format!("%{}%", kw);
                    params.push(Box::new(pattern.clone()));
                    params.push(Box::new(pattern.clone()));
                    params.push(Box::new(pattern));
                }
            }
            (conditions.join(" AND "), params)
        };

        let count_sql = format!("SELECT COUNT(*) FROM conversations c WHERE {}", where_clause);
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let total: i64 = conn
            .query_row(&count_sql, param_refs.as_slice(), |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let data_sql = format!(
            "SELECT c.id, c.app_id, c.conversation_id, c.name, c.created_at, c.updated_at,
             (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.conversation_id AND m.app_id = c.app_id) as message_count,
             a.name as app_name
             FROM conversations c
             LEFT JOIN apps a ON c.app_id = a.id
             WHERE {}
             ORDER BY c.created_at DESC
             LIMIT ? OFFSET ?",
            where_clause
        );

        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = params_vec;
        all_params.push(Box::new(page_size));
        all_params.push(Box::new(offset));
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&data_sql).map_err(|e| e.to_string())?;
        let data: Vec<ConversationSummary> = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(ConversationSummary {
                    id: row.get(0)?,
                    app_id: row.get(1)?,
                    conversation_id: row.get(2)?,
                    name: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    message_count: row.get(6)?,
                    app_name: row.get(7).unwrap_or_else(|_| "Unknown".to_string()),
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(ConversationsResult { data, total })
    }

    // ===== Messages =====
    pub fn upsert_message(&self, app_id: &str, conversation_id: &str, msg: &DifyMessageItem) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let id = format!("{}:{}", app_id, msg.id);
        let feedback_rating = msg
            .feedback
            .as_ref()
            .map(|f| f.rating.clone())
            .or_else(|| feedback_rating_from_array(&msg.feedbacks));
        let mut retriever_resources = msg.retriever_resources.clone();
        if retriever_resources.as_array().map(|items| items.is_empty()).unwrap_or(true) {
            if let Some(metadata_resources) = msg.message_metadata.get("retriever_resources") {
                retriever_resources = metadata_resources.clone();
            }
        }

        conn.execute(
            "INSERT OR REPLACE INTO messages (
                id, app_id, conversation_id, message_id, query, answer, feedback,
                retriever_resources, message_metadata, agent_thoughts, answer_tokens, prompt_tokens,
                elapsed_time, workflow_run_id, inputs, message_tokens, provider_response_latency,
                feedbacks, annotation, annotation_hit_history, message_files, status, error,
                parent_message_id, raw_json, created_at, synced_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, strftime('%s','now'))",
            params![
                id,
                app_id,
                conversation_id,
                msg.id,
                msg.query,
                msg.answer,
                feedback_rating,
                json_string(&retriever_resources, "[]"),
                json_string(&msg.message_metadata, "{}"),
                json_string(&msg.agent_thoughts, "[]"),
                msg.answer_tokens,
                msg.prompt_tokens,
                msg.elapsed_time,
                msg.workflow_run_id,
                json_string(&msg.inputs, "{}"),
                msg.message_tokens,
                msg.provider_response_latency,
                json_string(&msg.feedbacks, "[]"),
                json_string(&msg.annotation, "null"),
                json_string(&msg.annotation_hit_history, "null"),
                json_string(&msg.message_files, "[]"),
                msg.status,
                json_string(&msg.error, "null"),
                msg.parent_message_id,
                json_string(&msg.raw_json, "{}"),
                msg.created_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn upsert_workflow_run(&self, app_id: &str, run: &DifyWorkflowRun) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let id = format!("{}:{}", app_id, run.id);
        conn.execute(
            "INSERT OR REPLACE INTO workflow_runs (
                id, app_id, workflow_run_id, workflow_id, status, version, graph,
                elapsed_time, total_tokens, total_steps, created_at, finished_at, raw_json, synced_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, strftime('%s','now'))",
            params![
                id,
                app_id,
                run.id,
                run.workflow_id,
                run.status,
                run.version,
                json_string(&run.graph, "{}"),
                run.elapsed_time,
                run.total_tokens,
                run.total_steps,
                run.created_at,
                run.finished_at,
                json_string(&run.raw_json, "{}"),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn upsert_node_execution(&self, app_id: &str, workflow_run_id: &str, node: &DifyNodeExecution) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let id = format!("{}:{}", app_id, node.id);
        conn.execute(
            "INSERT OR REPLACE INTO node_executions (
                id, app_id, workflow_run_id, execution_id, execution_index, node_id, node_type,
                title, inputs, process_data, outputs, execution_metadata, extras, status, error,
                elapsed_time, created_at, finished_at, raw_json, synced_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, strftime('%s','now'))",
            params![
                id,
                app_id,
                workflow_run_id,
                node.id,
                node.index,
                node.node_id,
                node.node_type,
                node.title,
                json_string(&node.inputs, "{}"),
                json_string(&node.process_data, "{}"),
                json_string(&node.outputs, "{}"),
                json_string(&node.execution_metadata, "{}"),
                json_string(&node.extras, "{}"),
                node.status,
                json_string(&node.error, "null"),
                node.elapsed_time,
                node.created_at,
                node.finished_at,
                json_string(&node.raw_json, "{}"),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_messages(&self, app_id: Option<&str>, conversation_id: &str) -> Result<Vec<MessageDetail>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let (where_clause, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(aid) = app_id {
            (
                "conversation_id = ?1 AND app_id = ?2".to_string(),
                vec![Box::new(conversation_id.to_string()), Box::new(aid.to_string())],
            )
        } else {
            (
                "conversation_id = ?1".to_string(),
                vec![Box::new(conversation_id.to_string())],
            )
        };
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(
                &format!("SELECT id, app_id, conversation_id, message_id, query, answer, feedback,
                 retriever_resources, message_metadata, agent_thoughts, answer_tokens, prompt_tokens,
                 elapsed_time, created_at, workflow_run_id, inputs, message_tokens,
                 provider_response_latency, feedbacks, annotation, annotation_hit_history,
                 message_files, status, error, parent_message_id, raw_json
                 FROM messages WHERE {} ORDER BY created_at ASC", where_clause),
            )
            .map_err(|e| e.to_string())?;

        let messages: Vec<Message> = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(Message {
                    id: row.get(0)?,
                    app_id: row.get(1)?,
                    conversation_id: row.get(2)?,
                    message_id: row.get(3)?,
                    query: row.get(4)?,
                    answer: row.get(5)?,
                    feedback: row.get(6)?,
                    retriever_resources: row.get(7)?,
                    message_metadata: row.get(8)?,
                    agent_thoughts: row.get(9)?,
                    answer_tokens: row.get(10)?,
                    prompt_tokens: row.get(11)?,
                    elapsed_time: row.get(12)?,
                    created_at: row.get(13)?,
                    workflow_run_id: row.get(14)?,
                    inputs: row.get(15)?,
                    message_tokens: row.get(16)?,
                    provider_response_latency: row.get(17)?,
                    feedbacks: row.get(18)?,
                    annotation: row.get(19)?,
                    annotation_hit_history: row.get(20)?,
                    message_files: row.get(21)?,
                    status: row.get(22)?,
                    error: row.get(23)?,
                    parent_message_id: row.get(24)?,
                    raw_json: row.get(25)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        messages
            .into_iter()
            .map(|m| self.message_detail_from_row(&conn, m))
            .collect()
    }

    fn message_detail_without_workflow(&self, m: Message) -> MessageDetail {
        MessageDetail {
            id: m.id,
            app_id: m.app_id,
            message_id: m.message_id,
            conversation_id: m.conversation_id,
            query: m.query,
            answer: m.answer,
            feedback: m.feedback,
            feedbacks: parse_json(&m.feedbacks),
            retriever_resources: parse_json(&m.retriever_resources),
            message_metadata: parse_json(&m.message_metadata),
            agent_thoughts: parse_json(&m.agent_thoughts),
            inputs: parse_json(&m.inputs),
            message_files: parse_json(&m.message_files),
            annotation: parse_json(&m.annotation),
            annotation_hit_history: parse_json(&m.annotation_hit_history),
            status: m.status,
            error: parse_json(&m.error),
            parent_message_id: m.parent_message_id,
            workflow_run_id: m.workflow_run_id,
            workflow_run: None,
            node_executions: Vec::new(),
            raw_json: parse_json(&m.raw_json),
            answer_tokens: m.answer_tokens,
            prompt_tokens: m.prompt_tokens,
            message_tokens: m.message_tokens,
            provider_response_latency: m.provider_response_latency,
            elapsed_time: m.elapsed_time,
            created_at: m.created_at,
        }
    }

    fn message_detail_from_row(&self, conn: &Connection, m: Message) -> Result<MessageDetail, String> {
        let workflow_run = if let Some(ref run_id) = m.workflow_run_id {
            self.get_workflow_run(conn, &m.app_id, run_id)?
        } else {
            None
        };
        let node_executions = if let Some(ref run_id) = m.workflow_run_id {
            self.get_node_executions(conn, &m.app_id, run_id)?
        } else {
            Vec::new()
        };

        Ok(MessageDetail {
            id: m.id,
            app_id: m.app_id,
            message_id: m.message_id,
            conversation_id: m.conversation_id,
            query: m.query,
            answer: m.answer,
            feedback: m.feedback,
            feedbacks: parse_json(&m.feedbacks),
            retriever_resources: parse_json(&m.retriever_resources),
            message_metadata: parse_json(&m.message_metadata),
            agent_thoughts: parse_json(&m.agent_thoughts),
            inputs: parse_json(&m.inputs),
            message_files: parse_json(&m.message_files),
            annotation: parse_json(&m.annotation),
            annotation_hit_history: parse_json(&m.annotation_hit_history),
            status: m.status,
            error: parse_json(&m.error),
            parent_message_id: m.parent_message_id,
            workflow_run_id: m.workflow_run_id,
            workflow_run,
            node_executions,
            raw_json: parse_json(&m.raw_json),
            answer_tokens: m.answer_tokens,
            prompt_tokens: m.prompt_tokens,
            message_tokens: m.message_tokens,
            provider_response_latency: m.provider_response_latency,
            elapsed_time: m.elapsed_time,
            created_at: m.created_at,
        })
    }

    fn get_workflow_run(&self, conn: &Connection, app_id: &str, run_id: &str) -> Result<Option<WorkflowRunDetail>, String> {
        conn.query_row(
            "SELECT id, workflow_run_id, workflow_id, status, version, graph, elapsed_time, total_tokens, total_steps, created_at, finished_at, raw_json
             FROM workflow_runs WHERE app_id = ?1 AND workflow_run_id = ?2",
            params![app_id, run_id],
            |row| {
                let graph: String = row.get(5)?;
                let raw_json: String = row.get(11)?;
                Ok(WorkflowRunDetail {
                    id: row.get(0)?,
                    workflow_run_id: row.get(1)?,
                    workflow_id: row.get(2)?,
                    status: row.get(3)?,
                    version: row.get(4)?,
                    graph: parse_json(&graph),
                    elapsed_time: row.get(6)?,
                    total_tokens: row.get(7)?,
                    total_steps: row.get(8)?,
                    created_at: row.get(9)?,
                    finished_at: row.get(10)?,
                    raw_json: parse_json(&raw_json),
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    fn get_node_executions(&self, conn: &Connection, app_id: &str, run_id: &str) -> Result<Vec<NodeExecutionDetail>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, execution_id, workflow_run_id, node_id, node_type, title, inputs,
                 process_data, outputs, execution_metadata, extras, status, error, elapsed_time,
                 created_at, finished_at, raw_json
                 FROM node_executions
                 WHERE app_id = ?1 AND workflow_run_id = ?2
                 ORDER BY execution_index ASC, created_at ASC",
            )
            .map_err(|e| e.to_string())?;
        let nodes = stmt
            .query_map(params![app_id, run_id], |row| {
                let inputs: String = row.get(6)?;
                let process_data: String = row.get(7)?;
                let outputs: String = row.get(8)?;
                let execution_metadata: String = row.get(9)?;
                let extras: String = row.get(10)?;
                let error: String = row.get(12)?;
                let raw_json: String = row.get(16)?;
                Ok(NodeExecutionDetail {
                    id: row.get(0)?,
                    execution_id: row.get(1)?,
                    workflow_run_id: row.get(2)?,
                    node_id: row.get(3)?,
                    node_type: row.get(4)?,
                    title: row.get(5)?,
                    inputs: parse_json(&inputs),
                    process_data: parse_json(&process_data),
                    outputs: parse_json(&outputs),
                    execution_metadata: parse_json(&execution_metadata),
                    extras: parse_json(&extras),
                    status: row.get(11)?,
                    error: parse_json(&error),
                    elapsed_time: row.get(13)?,
                    created_at: row.get(14)?,
                    finished_at: row.get(15)?,
                    raw_json: parse_json(&raw_json),
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(nodes)
    }

    pub fn get_messages_for_export(
        &self,
        app_id: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        keyword: Option<&str>,
    ) -> Result<Vec<MessageDetail>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut conditions = vec!["1=1".to_string()];
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(aid) = app_id {
            conditions.push("app_id = ?".to_string());
            params.push(Box::new(aid.to_string()));
        }
        if let Some(sd) = start_date {
            if !sd.is_empty() {
                conditions.push("created_at >= strftime('%s', ?)".to_string());
                params.push(Box::new(sd.to_string()));
            }
        }
        if let Some(ed) = end_date {
            if !ed.is_empty() {
                conditions.push("created_at < strftime('%s', ?, '+1 day')".to_string());
                params.push(Box::new(ed.to_string()));
            }
        }
        if let Some(kw) = keyword {
            if !kw.is_empty() {
                conditions.push("(query LIKE ? OR answer LIKE ?)".to_string());
                let pattern = format!("%{}%", kw);
                params.push(Box::new(pattern.clone()));
                params.push(Box::new(pattern));
            }
        }

        let sql = format!(
            "SELECT id, app_id, conversation_id, message_id, query, answer, feedback,
             retriever_resources, message_metadata, agent_thoughts, answer_tokens, prompt_tokens,
             elapsed_time, created_at, workflow_run_id, inputs, message_tokens,
             provider_response_latency, feedbacks, annotation, annotation_hit_history,
             message_files, status, error, parent_message_id, raw_json
             FROM messages WHERE {} ORDER BY created_at ASC",
            conditions.join(" AND ")
        );
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let messages: Vec<Message> = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(Message {
                    id: row.get(0)?,
                    app_id: row.get(1)?,
                    conversation_id: row.get(2)?,
                    message_id: row.get(3)?,
                    query: row.get(4)?,
                    answer: row.get(5)?,
                    feedback: row.get(6)?,
                    retriever_resources: row.get(7)?,
                    message_metadata: row.get(8)?,
                    agent_thoughts: row.get(9)?,
                    answer_tokens: row.get(10)?,
                    prompt_tokens: row.get(11)?,
                    elapsed_time: row.get(12)?,
                    created_at: row.get(13)?,
                    workflow_run_id: row.get(14)?,
                    inputs: row.get(15)?,
                    message_tokens: row.get(16)?,
                    provider_response_latency: row.get(17)?,
                    feedbacks: row.get(18)?,
                    annotation: row.get(19)?,
                    annotation_hit_history: row.get(20)?,
                    message_files: row.get(21)?,
                    status: row.get(22)?,
                    error: row.get(23)?,
                    parent_message_id: row.get(24)?,
                    raw_json: row.get(25)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(messages
            .into_iter()
            .map(|m| self.message_detail_without_workflow(m))
            .collect())
    }

    // ===== Feedback Messages =====
    pub fn get_feedback_messages(
        &self,
        app_id: Option<&str>,
        feedback_type: Option<&str>,
        keyword: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<FeedbackResult, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let offset = (page - 1) * page_size;

        let mut conditions = vec!["m.feedback IS NOT NULL".to_string()];
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(aid) = app_id {
            if !aid.is_empty() {
                conditions.push("m.app_id = ?".to_string());
                params.push(Box::new(aid.to_string()));
            }
        }
        if let Some(ft) = feedback_type {
            if !ft.is_empty() {
                conditions.push("m.feedback = ?".to_string());
                params.push(Box::new(ft.to_string()));
            }
        }
        if let Some(kw) = keyword {
            if !kw.is_empty() {
                conditions.push("(m.query LIKE ? OR m.answer LIKE ?)".to_string());
                let pattern = format!("%{}%", kw);
                params.push(Box::new(pattern.clone()));
                params.push(Box::new(pattern));
            }
        }

        let where_clause = conditions.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM messages m WHERE {}", where_clause);
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let total: i64 = conn
            .query_row(&count_sql, param_refs.as_slice(), |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let data_sql = format!(
            "SELECT m.id, m.app_id, m.conversation_id, m.message_id, m.query, m.answer,
             m.feedback, m.feedbacks, m.answer_tokens, m.prompt_tokens, m.elapsed_time, m.created_at,
             COALESCE(a.name, 'Unknown') as app_name
             FROM messages m
             LEFT JOIN apps a ON m.app_id = a.id
             WHERE {}
             ORDER BY m.created_at DESC
             LIMIT ? OFFSET ?",
            where_clause
        );

        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = params;
        all_params.push(Box::new(page_size));
        all_params.push(Box::new(offset));
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&data_sql).map_err(|e| e.to_string())?;
        let data: Vec<FeedbackMessage> = stmt
            .query_map(param_refs.as_slice(), |row| {
                let feedbacks_str: String = row.get(7)?;
                Ok(FeedbackMessage {
                    id: row.get(0)?,
                    app_id: row.get(1)?,
                    conversation_id: row.get(2)?,
                    message_id: row.get(3)?,
                    query: row.get(4)?,
                    answer: row.get(5)?,
                    feedback: row.get(6)?,
                    feedbacks: parse_json(&feedbacks_str),
                    answer_tokens: row.get(8)?,
                    prompt_tokens: row.get(9)?,
                    elapsed_time: row.get(10)?,
                    created_at: row.get(11)?,
                    app_name: row.get(12)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(FeedbackResult { data, total })
    }

    // ===== Dashboard Stats =====
    pub fn get_dashboard_stats(&self) -> Result<DashboardStats, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let total_apps: i64 = conn.query_row("SELECT COUNT(*) FROM apps", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        let total_conversations: i64 = conn.query_row("SELECT COUNT(*) FROM conversations", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        let total_messages: i64 = conn.query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        let total_answer_tokens: i64 = conn.query_row("SELECT COALESCE(SUM(answer_tokens), 0) FROM messages", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        let total_prompt_tokens: i64 = conn.query_row("SELECT COALESCE(SUM(prompt_tokens), 0) FROM messages", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        let feedback_like: i64 = conn.query_row("SELECT COUNT(*) FROM messages WHERE feedback = 'like'", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        let feedback_dislike: i64 = conn.query_row("SELECT COUNT(*) FROM messages WHERE feedback = 'dislike'", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        let feedback_none: i64 = conn.query_row("SELECT COUNT(*) FROM messages WHERE feedback IS NULL", [], |row| row.get(0)).map_err(|e| e.to_string())?;

        let mut stmt = conn.prepare(
            "SELECT c.app_id, a.name, COUNT(DISTINCT c.conversation_id) as conv_count, COUNT(m.id) as msg_count
             FROM conversations c
             LEFT JOIN apps a ON c.app_id = a.id
             LEFT JOIN messages m ON m.app_id = c.app_id AND m.conversation_id = c.conversation_id
             GROUP BY c.app_id
             ORDER BY conv_count DESC
             LIMIT 10"
        ).map_err(|e| e.to_string())?;
        let top_apps: Vec<AppRanking> = stmt.query_map([], |row| {
            Ok(AppRanking {
                app_id: row.get(0)?,
                app_name: row.get(1).unwrap_or_else(|_| "Unknown".to_string()),
                conversation_count: row.get(2)?,
                message_count: row.get(3)?,
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        let mut stmt = conn.prepare(
            "SELECT date(created_at, 'unixepoch', 'localtime') as day,
             COUNT(DISTINCT conversation_id) as conv_count,
             COUNT(*) as msg_count
             FROM messages
             WHERE created_at >= strftime('%s', 'now', '-7 days')
             GROUP BY day
             ORDER BY day DESC"
        ).map_err(|e| e.to_string())?;
        let recent_daily: Vec<DailyStats> = stmt.query_map([], |row| {
            Ok(DailyStats {
                date: row.get(0)?,
                conversations: row.get(1)?,
                messages: row.get(2)?,
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        Ok(DashboardStats {
            total_apps,
            total_conversations,
            total_messages,
            total_answer_tokens,
            total_prompt_tokens,
            feedback_like,
            feedback_dislike,
            feedback_none,
            top_apps,
            recent_daily,
        })
    }
}

fn feedback_rating_from_array(feedbacks: &serde_json::Value) -> Option<String> {
    feedbacks
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("rating").or_else(|| item.get("value")).or_else(|| item.get("type")))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}
