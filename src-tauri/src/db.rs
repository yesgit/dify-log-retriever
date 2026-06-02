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

            CREATE TABLE IF NOT EXISTS workflow_app_logs (
                id TEXT PRIMARY KEY,
                app_id TEXT NOT NULL,
                log_id TEXT NOT NULL,
                workflow_run_id TEXT DEFAULT '',
                status TEXT DEFAULT '',
                elapsed_time REAL DEFAULT 0.0,
                total_tokens INTEGER DEFAULT 0,
                total_steps INTEGER DEFAULT 0,
                created_from TEXT DEFAULT '',
                created_by_role TEXT DEFAULT '',
                created_by_end_user_id TEXT DEFAULT '',
                created_by_end_user_session_id TEXT DEFAULT '',
                error TEXT DEFAULT 'null',
                created_at INTEGER DEFAULT 0,
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
            CREATE INDEX IF NOT EXISTS idx_messages_app_created ON messages(app_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_conversations_app_created ON conversations(app_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_workflow_runs_app_run ON workflow_runs(app_id, workflow_run_id);
            CREATE INDEX IF NOT EXISTS idx_node_executions_app_run ON node_executions(app_id, workflow_run_id);
            CREATE INDEX IF NOT EXISTS idx_node_executions_app_type ON node_executions(app_id, node_type, node_id, status);
            CREATE INDEX IF NOT EXISTS idx_workflow_app_logs_app ON workflow_app_logs(app_id);
            CREATE INDEX IF NOT EXISTS idx_workflow_app_logs_run_id ON workflow_app_logs(app_id, workflow_run_id);
            CREATE INDEX IF NOT EXISTS idx_workflow_app_logs_created ON workflow_app_logs(app_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_messages_app_run ON messages(app_id, workflow_run_id);
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

    // ===== Auto Sync Settings =====
    pub fn get_auto_sync_settings(&self) -> Result<AutoSyncSettings, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let enabled: bool = conn
            .query_row("SELECT value FROM settings WHERE key = 'auto_sync_enabled'", [], |row| row.get::<_, String>(0))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);
        let interval_minutes: i64 = conn
            .query_row("SELECT value FROM settings WHERE key = 'auto_sync_interval'", [], |row| row.get::<_, String>(0))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        let mode: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'auto_sync_mode'", [], |row| row.get::<_, String>(0))
            .ok()
            .unwrap_or_else(|| "incremental".to_string());
        let last_synced_at: Option<i64> = conn
            .query_row("SELECT value FROM settings WHERE key = 'auto_sync_last_synced'", [], |row| row.get::<_, String>(0))
            .ok()
            .and_then(|v| v.parse().ok());
        Ok(AutoSyncSettings {
            enabled,
            interval_minutes,
            mode,
            last_synced_at,
        })
    }

    pub fn save_auto_sync_settings(&self, settings: &AutoSyncSettings) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["auto_sync_enabled", settings.enabled.to_string()],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["auto_sync_interval", settings.interval_minutes.to_string()],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["auto_sync_mode", &settings.mode],
        )
        .map_err(|e| e.to_string())?;
        if let Some(ts) = settings.last_synced_at {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params!["auto_sync_last_synced", ts.to_string()],
            )
            .map_err(|e| e.to_string())?;
        } else {
            conn.execute("DELETE FROM settings WHERE key = 'auto_sync_last_synced'", [])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn update_auto_sync_last_synced(&self, ts: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["auto_sync_last_synced", ts.to_string()],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
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
        conn.execute("DELETE FROM workflow_app_logs WHERE app_id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM messages WHERE app_id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM conversations WHERE app_id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM apps WHERE id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Workflow App Logs =====
    pub fn upsert_workflow_app_log(&self, app_id: &str, log: &DifyWorkflowAppLogItem) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let id = format!("{}:{}", app_id, log.id);
        let end_user_id = log.created_by_end_user.as_ref().map(|u| u.id.clone()).unwrap_or_default();
        let end_user_session_id = log.created_by_end_user.as_ref().map(|u| u.session_id.clone()).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO workflow_app_logs (
                id, app_id, log_id, workflow_run_id, status, elapsed_time, total_tokens, total_steps,
                created_from, created_by_role, created_by_end_user_id, created_by_end_user_session_id,
                error, created_at, synced_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, strftime('%s','now'))",
            params![
                id,
                app_id,
                log.id,
                log.workflow_run.id,
                log.workflow_run.status,
                log.workflow_run.elapsed_time,
                log.workflow_run.total_tokens,
                log.workflow_run.total_steps,
                log.created_from,
                log.created_by_role,
                end_user_id,
                end_user_session_id,
                json_string(log.workflow_run.error.as_ref().unwrap_or(&serde_json::Value::Null), "null"),
                log.created_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_workflow_app_log_count(&self, app_id: &str) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workflow_app_logs WHERE app_id = ?1",
                params![app_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(count)
    }

    /// Get the maximum created_at timestamp among existing workflow app logs.
    /// Used for incremental sync to know where to stop.
    pub fn get_workflow_app_log_max_created_at(&self, app_id: &str) -> Result<Option<i64>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result: Option<i64> = conn
            .query_row(
                "SELECT MAX(created_at) FROM workflow_app_logs WHERE app_id = ?1",
                params![app_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .flatten();
        // .optional returns Ok(Some(row)) or Ok(None); row.get(0) could be NULL
        Ok(result)
    }

    /// Check if a workflow app log already exists by its log_id
    #[allow(dead_code)]
    pub fn workflow_app_log_exists(&self, app_id: &str, log_id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let id = format!("{}:{}", app_id, log_id);
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM workflow_app_logs WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(exists)
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

    // ===== Node Eval Export =====
    pub fn get_app_node_types(&self, app_id: &str) -> Result<Vec<NodeTypeSummary>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT node_type, node_id, title, COUNT(*) as cnt
                 FROM node_executions
                 WHERE app_id = ?1 AND status = 'succeeded'
                 GROUP BY node_type, node_id, title
                 ORDER BY node_type, node_id",
            )
            .map_err(|e| e.to_string())?;
        let result = stmt
            .query_map(params![app_id], |row| {
                Ok(NodeTypeSummary {
                    node_type: row.get(0)?,
                    node_id: row.get(1)?,
                    node_title: row.get(2)?,
                    count: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(result)
    }

    #[allow(unused_assignments)]
    pub fn get_node_executions_for_export(
        &self,
        app_id: &str,
        node_type: Option<&str>,
        node_id: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<Vec<NodeEvalRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut conditions = vec![
            "ne.app_id = ?1".to_string(),
            "ne.status = 'succeeded'".to_string(),
        ];
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(app_id.to_string())];
        let mut param_idx = 2;

        if let Some(nt) = node_type {
            if !nt.is_empty() {
                conditions.push(format!("ne.node_type = ?{}", param_idx));
                params.push(Box::new(nt.to_string()));
                param_idx += 1;
            }
        }
        if let Some(nid) = node_id {
            if !nid.is_empty() {
                conditions.push(format!("ne.node_id = ?{}", param_idx));
                params.push(Box::new(nid.to_string()));
                param_idx += 1;
            }
        }
        if let Some(sd) = start_date {
            if !sd.is_empty() {
                conditions.push(format!("m.created_at >= strftime('%s', ?{})", param_idx));
                params.push(Box::new(sd.to_string()));
                param_idx += 1;
            }
        }
        if let Some(ed) = end_date {
            if !ed.is_empty() {
                conditions.push(format!("m.created_at < strftime('%s', ?{}, '+1 day')", param_idx));
                params.push(Box::new(ed.to_string()));
                param_idx += 1;
            }
        }

        let where_clause = conditions.join(" AND ");

        // Use a subquery to get one message per workflow_run_id deterministically
        let sql = format!(
            "SELECT ne.execution_id, ne.workflow_run_id, ne.node_id, ne.node_type, ne.title,
                    ne.app_id, m.conversation_id, m.message_id, m.query,
                    ne.inputs, ne.outputs, ne.process_data, ne.status, ne.elapsed_time, ne.created_at
             FROM node_executions ne
             INNER JOIN (
                SELECT app_id, workflow_run_id,
                       MIN(conversation_id) as conversation_id,
                       MIN(message_id) as message_id,
                       MIN(query) as query
                FROM messages
                WHERE workflow_run_id IS NOT NULL AND workflow_run_id != ''
                GROUP BY app_id, workflow_run_id
             ) m ON ne.app_id = m.app_id AND ne.workflow_run_id = m.workflow_run_id
             WHERE {}
             ORDER BY ne.created_at ASC",
            where_clause
        );

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let records = stmt
            .query_map(param_refs.as_slice(), |row| {
                let inputs_str: String = row.get(9)?;
                let outputs_str: String = row.get(10)?;
                let process_data_str: String = row.get(11)?;
                Ok(NodeEvalRecord {
                    execution_id: row.get(0)?,
                    workflow_run_id: row.get(1)?,
                    node_id: row.get(2)?,
                    node_type: row.get(3)?,
                    node_title: row.get(4)?,
                    app_id: row.get(5)?,
                    conversation_id: row.get(6)?,
                    message_id: row.get(7)?,
                    query: row.get(8)?,
                    inputs: parse_json(&inputs_str),
                    outputs: parse_json(&outputs_str),
                    process_data: parse_json(&process_data_str),
                    status: row.get(12)?,
                    elapsed_time: row.get(13)?,
                    created_at: row.get(14)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(records)
    }

    // ===== Dashboard Stats =====
    pub fn get_dashboard_stats(
        &self,
        app_id: Option<&str>,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> Result<DashboardStats, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // Build WHERE clauses
        // Note: app_id comes from DB-sourced dropdown values, timestamps are i64 - safe from injection
        let msg_where = build_where(app_id, start_time, end_time);
        // Query-only WHERE: only messages with non-empty query (user questions)
        let msg_where_q = format!("query != '' AND {}", msg_where);
        let conv_where = build_conv_where(app_id, start_time, end_time);
        // Prefixed message WHERE for queries that alias messages as `m`.
        let msg_where_m = build_where_prefixed("m.", app_id, start_time, end_time);
        let msg_where_m_q = format!("m.query != '' AND {}", msg_where_m);

        // ── Basic counts ──
        let total_apps: i64 = if let Some(aid) = app_id {
            conn.query_row("SELECT COUNT(*) FROM apps WHERE id = ?1", params![aid], |row| row.get(0)).unwrap_or(1)
        } else {
            conn.query_row("SELECT COUNT(*) FROM apps", [], |row| row.get(0)).map_err(|e| e.to_string())?
        };

        let total_conversations: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM conversations c WHERE {}", conv_where),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        // Messages = user messages with non-empty query (user questions only)
        let total_messages: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM messages WHERE {}", msg_where_q),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        // Users: distinct from_end_user_id from conversations
        let total_users: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(DISTINCT from_end_user_id) FROM conversations c WHERE from_end_user_id != '' AND {}",
                conv_where
            ),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        // ── Token totals ──
        let total_answer_tokens: i64 = conn.query_row(
            &format!("SELECT COALESCE(SUM(answer_tokens), 0) FROM messages WHERE {}", msg_where_q),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        let total_prompt_tokens: i64 = conn.query_row(
            &format!("SELECT COALESCE(SUM(prompt_tokens), 0) FROM messages WHERE {}", msg_where_q),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        // Message-level effective tokens:
        // - Prefer message_tokens when available
        // - Fallback to prompt + answer for historical records without message_tokens
        let message_effective_tokens: i64 = conn.query_row(
            &format!(
                "SELECT COALESCE(SUM(CASE WHEN message_tokens > 0 THEN message_tokens ELSE (answer_tokens + prompt_tokens) END), 0) FROM messages WHERE {}",
                msg_where_q
            ),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        // Workflow supplement (non-duplicating):
        // build run scope from the same filtered message set, then add only run-level remainder.
        let workflow_supplement_tokens: i64 = conn.query_row(
            &format!(
                "WITH workflow_run_scope AS (
                    SELECT m.app_id,
                           m.workflow_run_id,
                           SUM(CASE WHEN m.message_tokens > 0 THEN m.message_tokens ELSE (m.answer_tokens + m.prompt_tokens) END) AS msg_token_sum,
                           COALESCE(MAX(wr.total_tokens), 0) AS run_total_tokens
                    FROM messages m
                    LEFT JOIN workflow_runs wr
                      ON wr.app_id = m.app_id AND wr.workflow_run_id = m.workflow_run_id
                    WHERE m.workflow_run_id IS NOT NULL AND m.workflow_run_id != '' AND {}
                    GROUP BY m.app_id, m.workflow_run_id
                )
                SELECT COALESCE(SUM(
                    CASE
                        WHEN run_total_tokens > msg_token_sum THEN run_total_tokens - msg_token_sum
                        ELSE 0
                    END
                ), 0)
                FROM workflow_run_scope",
                msg_where_m_q
            ),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        let total_tokens = message_effective_tokens + workflow_supplement_tokens;

        // ── Average distributions ──
        let messages_per_conversation_distribution = compute_distribution(
            &conn,
            &format!(
                "SELECT CAST(msg_count AS REAL) FROM (
                    SELECT app_id, conversation_id, COUNT(*) as msg_count
                    FROM messages WHERE {}
                    GROUP BY app_id, conversation_id
                ) sub",
                msg_where_q
            ),
        )?;

        let conversations_per_user_distribution = compute_distribution(
            &conn,
            &format!(
                "SELECT CAST(conv_count AS REAL) FROM (
                    SELECT from_end_user_id, COUNT(*) as conv_count
                    FROM conversations c WHERE from_end_user_id != '' AND {}
                    GROUP BY from_end_user_id
                ) sub",
                conv_where
            ),
        )?;

        let messages_per_user_distribution = compute_distribution(
            &conn,
            &format!(
                "SELECT CAST(msg_count AS REAL) FROM (
                    SELECT c.from_end_user_id, COUNT(*) as msg_count
                    FROM conversations c
                    INNER JOIN messages m ON m.app_id = c.app_id AND m.conversation_id = c.conversation_id
                    WHERE c.from_end_user_id != '' AND {}
                    GROUP BY c.from_end_user_id
                ) sub",
                msg_where_m_q
            ),
        )?;

        // ── Feedback counts ──
        let feedback_like: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM messages WHERE feedback = 'like' AND {}", msg_where_q),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        let feedback_dislike: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM messages WHERE feedback = 'dislike' AND {}", msg_where_q),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        let feedback_none: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM messages WHERE feedback IS NULL AND {}", msg_where_q),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        let feedback_total = feedback_like + feedback_dislike;
        let feedback_like_rate = if feedback_total > 0 { feedback_like as f64 / feedback_total as f64 * 100.0 } else { 0.0 };

        // Feedback with content: feedbacks JSON array has at least one item with non-empty label/rating or content/message
        let feedback_with_content: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM messages WHERE feedback IS NOT NULL AND EXISTS (
                    SELECT 1 FROM json_each(feedbacks) WHERE
                        (COALESCE(json_extract(value, '$.rating'), json_extract(value, '$.label'), json_extract(value, '$.value'), '') != '')
                        OR (COALESCE(json_extract(value, '$.content'), json_extract(value, '$.message'), '') != '')
                ) AND {}",
                msg_where_q
            ),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string()).unwrap_or(0);

        let avg_feedback_per_user = if total_users > 0 { feedback_total as f64 / total_users as f64 } else { 0.0 };
        let avg_feedback_per_conversation = if total_conversations > 0 { feedback_total as f64 / total_conversations as f64 } else { 0.0 };
        let avg_feedback_per_message = if total_messages > 0 { feedback_total as f64 / total_messages as f64 } else { 0.0 };

        // ── Error stats ──
        let error_count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM messages WHERE ((error IS NOT NULL AND error != 'null' AND error != '') OR status = 'error') AND {}", msg_where_q),
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        let error_rate = if total_messages > 0 { error_count as f64 / total_messages as f64 * 100.0 } else { 0.0 };

        // ── Dify-aligned metrics ──
        let satisfaction_rate = if total_messages > 0 { feedback_like as f64 / total_messages as f64 * 1000.0 } else { 0.0 };
        let avg_conversation_interactions = if total_conversations > 0 { total_messages as f64 / total_conversations as f64 } else { 0.0 };

        // ── Distributions ──
        // TTFT (provider_response_latency)
        let ttft_distribution = compute_distribution(
            &conn,
            &format!("SELECT provider_response_latency FROM messages WHERE provider_response_latency > 0 AND {}", msg_where_q),
        )?;

        // Elapsed time
        let elapsed_time_distribution = compute_distribution(
            &conn,
            &format!("SELECT elapsed_time FROM messages WHERE elapsed_time > 0 AND {}", msg_where_q),
        )?;

        // Tokens per message
        let token_per_message_distribution = compute_distribution(
            &conn,
            &format!(
                "SELECT (CASE WHEN message_tokens > 0 THEN message_tokens ELSE (answer_tokens + prompt_tokens) END) FROM messages WHERE (CASE WHEN message_tokens > 0 THEN message_tokens ELSE (answer_tokens + prompt_tokens) END) > 0 AND {}",
                msg_where_q
            ),
        )?;

        // Token speed (tokens/s) = answer_tokens / elapsed_time
        let token_speed_distribution = compute_distribution(
            &conn,
            &format!("SELECT CAST(answer_tokens AS REAL) / elapsed_time FROM messages WHERE elapsed_time > 0 AND answer_tokens > 0 AND {}", msg_where_q),
        )?;

        // User feedback count distribution
        let user_feedback_count_distribution = compute_distribution(
            &conn,
            &format!(
                "SELECT CAST(feedback_count AS REAL) FROM (
                    SELECT c.from_end_user_id, COUNT(m.id) as feedback_count
                    FROM conversations c
                    LEFT JOIN messages m ON m.app_id = c.app_id AND m.conversation_id = c.conversation_id AND m.feedback IS NOT NULL
                    WHERE c.from_end_user_id != '' AND {}
                    GROUP BY c.from_end_user_id
                ) sub",
                conv_where
            ),
        )?;

        // Conversation feedback count distribution
        let conversation_feedback_count_distribution = compute_distribution(
            &conn,
            &format!(
                "SELECT CAST(feedback_count AS REAL) FROM (
                    SELECT c.app_id, c.conversation_id, COUNT(m.id) as feedback_count
                    FROM conversations c
                    LEFT JOIN messages m ON m.app_id = c.app_id AND m.conversation_id = c.conversation_id AND m.feedback IS NOT NULL
                    WHERE {}
                    GROUP BY c.app_id, c.conversation_id
                ) sub",
                conv_where
            ),
        )?;

        // Message feedback count distribution (feedbacks array length per message with feedback)
        let message_feedback_count_distribution = compute_distribution(
            &conn,
            &format!(
                "SELECT CAST(json_array_length(feedbacks) AS REAL) FROM messages WHERE feedback IS NOT NULL AND json_array_length(feedbacks) >= 0 AND {}",
                msg_where_q
            ),
        )?;

        // ── Feedback label stats ──
        let feedback_label_sql = format!(
            "SELECT COALESCE(feedback, 'none') as fb_label, COUNT(*) as cnt FROM messages WHERE {} GROUP BY fb_label ORDER BY cnt DESC",
            msg_where_q
        );
        let mut stmt = conn.prepare(&feedback_label_sql).map_err(|e| e.to_string())?;
        let feedback_label_stats: Vec<FeedbackLabelStat> = stmt.query_map([], |row| {
            Ok(FeedbackLabelStat {
                feedback: row.get(0)?,
                count: row.get(1)?,
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        // ── Top apps ──
        let top_apps_sql = format!(
            "SELECT c.app_id, a.name, COUNT(DISTINCT c.conversation_id) as conv_count, COUNT(m.id) as msg_count
             FROM conversations c
             LEFT JOIN apps a ON c.app_id = a.id
             LEFT JOIN messages m ON m.app_id = c.app_id AND m.conversation_id = c.conversation_id AND m.query != ''
             WHERE {}
             GROUP BY c.app_id
             ORDER BY conv_count DESC
             LIMIT 10",
            conv_where
        );
        let mut stmt = conn.prepare(&top_apps_sql).map_err(|e| e.to_string())?;
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

        // ── Daily trend ──
        // (msg_where_m and msg_where_m_q already defined above for user messages distribution)

        // For daily users, join messages with conversations (only query messages)
        let daily_users_sql = format!(
            "SELECT date(m.created_at, 'unixepoch', 'localtime') as day,
                    COUNT(DISTINCT c.from_end_user_id) as user_count
             FROM messages m
             INNER JOIN conversations c ON m.app_id = c.app_id AND m.conversation_id = c.conversation_id
             WHERE {} AND c.from_end_user_id IS NOT NULL AND c.from_end_user_id != ''
             GROUP BY day",
            msg_where_m_q
        );
        // Build a map of date -> user count
        let mut daily_users_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        if let Ok(mut stmt) = conn.prepare(&daily_users_sql) {
            let _ = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            }).map(|rows| {
                for r in rows.flatten() {
                    daily_users_map.insert(r.0, r.1);
                }
            });
        }

        let daily_sql = format!(
            "WITH message_daily AS (
                SELECT date(created_at, 'unixepoch', 'localtime') as day,
                       COUNT(DISTINCT conversation_id) as conv_count,
                       COUNT(*) as msg_count,
                       COALESCE(SUM(CASE WHEN message_tokens > 0 THEN message_tokens ELSE (answer_tokens + prompt_tokens) END), 0) as msg_token_sum,
                       SUM(CASE WHEN ((error IS NOT NULL AND error != 'null' AND error != '') OR status = 'error') THEN 1 ELSE 0 END) as error_count,
                       SUM(CASE WHEN feedback = 'like' THEN 1 ELSE 0 END) as like_count,
                       SUM(CASE WHEN feedback = 'dislike' THEN 1 ELSE 0 END) as dislike_count,
                       COALESCE(AVG(CASE WHEN elapsed_time > 0 THEN elapsed_time END), 0) as avg_elapsed,
                       COALESCE(AVG(CASE WHEN provider_response_latency > 0 THEN provider_response_latency END), 0) as avg_ttft_val,
                       COALESCE(AVG(CASE WHEN elapsed_time > 0 AND answer_tokens > 0 THEN CAST(answer_tokens AS REAL) / elapsed_time END), 0) as avg_speed,
                       COALESCE(SUM(answer_tokens), 0) as answer_token_sum,
                       COALESCE(SUM(prompt_tokens), 0) as prompt_token_sum
                FROM messages
                WHERE {}
                GROUP BY day
            ),
            workflow_run_scope AS (
                SELECT m.app_id,
                       m.workflow_run_id,
                       date(MAX(m.created_at), 'unixepoch', 'localtime') as workflow_day,
                       SUM(CASE WHEN m.message_tokens > 0 THEN m.message_tokens ELSE (m.answer_tokens + m.prompt_tokens) END) as msg_token_sum,
                       COALESCE(MAX(wr.total_tokens), 0) as run_total_tokens
                FROM messages m
                LEFT JOIN workflow_runs wr
                  ON wr.app_id = m.app_id AND wr.workflow_run_id = m.workflow_run_id
                WHERE m.workflow_run_id IS NOT NULL AND m.workflow_run_id != '' AND {}
                GROUP BY m.app_id, m.workflow_run_id
            ),
            workflow_daily AS (
                SELECT workflow_day as day,
                       COALESCE(SUM(
                           CASE
                               WHEN run_total_tokens > msg_token_sum THEN run_total_tokens - msg_token_sum
                               ELSE 0
                           END
                       ), 0) as wf_token_sum
                FROM workflow_run_scope
                WHERE workflow_day IS NOT NULL
                GROUP BY workflow_day
            ),
            all_days AS (
                SELECT day FROM message_daily
                UNION
                SELECT day FROM workflow_daily
            )
            SELECT d.day,
                   COALESCE(md.conv_count, 0) as conv_count,
                   COALESCE(md.msg_count, 0) as msg_count,
                   COALESCE(md.msg_token_sum, 0) + COALESCE(wd.wf_token_sum, 0) as token_sum,
                   COALESCE(md.error_count, 0) as error_count,
                   COALESCE(md.like_count, 0) as like_count,
                   COALESCE(md.dislike_count, 0) as dislike_count,
                   COALESCE(md.avg_elapsed, 0) as avg_elapsed,
                   COALESCE(md.avg_ttft_val, 0) as avg_ttft_val,
                   COALESCE(md.avg_speed, 0) as avg_speed,
                   COALESCE(md.answer_token_sum, 0) as answer_token_sum,
                   COALESCE(md.prompt_token_sum, 0) as prompt_token_sum
            FROM all_days d
            LEFT JOIN message_daily md ON md.day = d.day
            LEFT JOIN workflow_daily wd ON wd.day = d.day
            ORDER BY d.day ASC",
            msg_where_q,
            msg_where_m_q
        );
        let mut stmt = conn.prepare(&daily_sql).map_err(|e| e.to_string())?;
        let recent_daily: Vec<DailyStats> = stmt.query_map([], |row| {
            let date: String = row.get(0)?;
            let users = daily_users_map.get(&date).copied().unwrap_or(0);
            Ok(DailyStats {
                date: date.clone(),
                conversations: row.get(1)?,
                messages: row.get(2)?,
                tokens: row.get(3)?,
                users,
                errors: row.get(4)?,
                likes: row.get(5)?,
                dislikes: row.get(6)?,
                avg_elapsed_time: row.get(7)?,
                avg_ttft: row.get(8)?,
                avg_token_speed: row.get(9)?,
                total_answer_tokens: row.get(10)?,
                total_prompt_tokens: row.get(11)?,
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        // Keep denominator aligned with the merged daily timeline when no full explicit range is provided.
        let days_in_range = if let (Some(s), Some(e)) = (start_time, end_time) {
            (((e - s) as f64 / 86400.0).floor() + 1.0).max(1.0)
        } else if let Some(s) = start_time {
            let now_ts: i64 = conn.query_row("SELECT CAST(strftime('%s','now') AS INTEGER)", [], |row| row.get::<_, i64>(0)).unwrap_or(0);
            (((now_ts - s) as f64 / 86400.0).floor() + 1.0).max(1.0)
        } else if recent_daily.len() >= 2 {
            // Use SQL julianday to compute calendar span from the daily results
            let first = &recent_daily.first().unwrap().date;
            let last = &recent_daily.last().unwrap().date;
            let span: i64 = conn.query_row(
                "SELECT CAST(julianday(?1) - julianday(?2) + 1 AS INTEGER)",
                params![last, first],
                |row| row.get(0),
            ).unwrap_or(1);
            (span as f64).max(1.0)
        } else {
            1.0
        };
        let daily_avg_tokens = total_tokens as f64 / days_in_range;

        // ── Per-model token speed daily trend ──
        // Extract model from message_metadata JSON field and group by model + date
        let model_speed_sql = format!(
            "SELECT
                COALESCE(json_extract(message_metadata, '$.model'), 'unknown') as model,
                date(created_at, 'unixepoch', 'localtime') as day,
                AVG(CASE WHEN elapsed_time > 0 THEN CAST(answer_tokens AS REAL) / elapsed_time ELSE 0 END) as avg_speed,
                COUNT(*) as cnt
             FROM messages
             WHERE answer_tokens > 0 AND elapsed_time > 0 AND {}
             GROUP BY model, day
             ORDER BY model, day",
            msg_where_q
        );
        let mut model_stmt = conn.prepare(&model_speed_sql).map_err(|e| e.to_string())?;
        let model_token_speed_daily: Vec<ModelDailyTokenSpeed> = model_stmt.query_map([], |row| {
            Ok(ModelDailyTokenSpeed {
                model: row.get(0)?,
                date: row.get(1)?,
                avg_token_speed: row.get(2)?,
                message_count: row.get(3)?,
            })
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

        // ── Per-model aggregated performance stats ──
        let model_perf_sql = format!(
            "SELECT
                COALESCE(json_extract(message_metadata, '$.model'), 'unknown') as model,
                COUNT(*) as msg_count,
                COALESCE(SUM(CASE WHEN message_tokens > 0 THEN message_tokens ELSE (answer_tokens + prompt_tokens) END), 0) as total_tokens,
                COALESCE(AVG(CASE WHEN elapsed_time > 0 THEN elapsed_time END), 0) as avg_elapsed,
                COALESCE(AVG(CASE WHEN provider_response_latency > 0 THEN provider_response_latency END), 0) as avg_ttft,
                COALESCE(AVG(CASE WHEN elapsed_time > 0 AND answer_tokens > 0 THEN CAST(answer_tokens AS REAL) / elapsed_time END), 0) as avg_speed,
                SUM(CASE WHEN ((error IS NOT NULL AND error != 'null' AND error != '') OR status = 'error') THEN 1 ELSE 0 END) as err_count
             FROM messages
             WHERE {}
             GROUP BY model
             ORDER BY msg_count DESC",
            msg_where_q
        );
        let mut model_perf_stmt = conn.prepare(&model_perf_sql).map_err(|e| e.to_string())?;
        let model_performance: Vec<ModelPerformanceStats> = model_perf_stmt.query_map([], |row| {
            let msg_count: i64 = row.get(1)?;
            let err_count: i64 = row.get(6)?;
            Ok(ModelPerformanceStats {
                model: row.get(0)?,
                message_count: msg_count,
                total_tokens: row.get(2)?,
                avg_elapsed_time: row.get(3)?,
                avg_ttft: row.get(4)?,
                avg_token_speed: row.get(5)?,
                error_count: err_count,
                error_rate: if msg_count > 0 { err_count as f64 / msg_count as f64 * 100.0 } else { 0.0 },
            })
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

        // ── Per-node-type aggregated performance stats ──
        let node_where = build_where_prefixed("ne.", app_id, start_time, end_time);
        let node_perf_sql = format!(
            "SELECT
                ne.node_type,
                COALESCE(NULLIF(ne.title, ''), '(未命名)') as title,
                COUNT(*) as exec_count,
                COALESCE(AVG(ne.elapsed_time), 0) as avg_elapsed,
                SUM(CASE WHEN ne.status = 'succeeded' THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN ne.status != 'succeeded' THEN 1 ELSE 0 END) as err_count
             FROM node_executions ne
             WHERE ne.node_type != '' AND {}
             GROUP BY ne.node_type, COALESCE(NULLIF(ne.title, ''), '(未命名)')
             ORDER BY exec_count DESC",
            node_where
        );
        let mut node_perf_stmt = conn.prepare(&node_perf_sql).map_err(|e| e.to_string())?;
        let node_performance: Vec<NodePerformanceStats> = node_perf_stmt.query_map([], |row| {
            let exec_count: i64 = row.get(2)?;
            let success_count: i64 = row.get(4)?;
            Ok(NodePerformanceStats {
                node_type: row.get(0)?,
                title: row.get(1)?,
                execution_count: exec_count,
                avg_elapsed_time: row.get(3)?,
                success_count,
                success_rate: if exec_count > 0 { success_count as f64 / exec_count as f64 * 100.0 } else { 0.0 },
                error_count: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

        Ok(DashboardStats {
            total_apps,
            total_users,
            total_conversations,
            total_messages,
            total_answer_tokens,
            total_prompt_tokens,
            total_tokens,
            daily_avg_tokens,
            messages_per_conversation_distribution,
            conversations_per_user_distribution,
            messages_per_user_distribution,
            feedback_total,
            feedback_like,
            feedback_dislike,
            feedback_none,
            feedback_with_content,
            feedback_like_rate,
            avg_feedback_per_user,
            avg_feedback_per_conversation,
            avg_feedback_per_message,
            error_count,
            error_rate,
            satisfaction_rate,
            avg_conversation_interactions,
            ttft_distribution,
            elapsed_time_distribution,
            token_per_message_distribution,
            token_speed_distribution,
            user_feedback_count_distribution,
            conversation_feedback_count_distribution,
            message_feedback_count_distribution,
            feedback_label_stats,
            top_apps,
            recent_daily,
            model_token_speed_daily,
            model_performance,
            node_performance,
        })
    }
}

fn build_where(app_id: Option<&str>, start_time: Option<i64>, end_time: Option<i64>) -> String {
    build_where_prefixed("", app_id, start_time, end_time)
}

fn build_conv_where(app_id: Option<&str>, start_time: Option<i64>, end_time: Option<i64>) -> String {
    build_where_prefixed("c.", app_id, start_time, end_time)
}

fn build_where_prefixed(prefix: &str, app_id: Option<&str>, start_time: Option<i64>, end_time: Option<i64>) -> String {
    let mut conditions = vec!["1=1".to_string()];
    if let Some(aid) = app_id {
        conditions.push(format!("{}app_id = '{}'", prefix, aid));
    }
    if let Some(st) = start_time {
        conditions.push(format!("{}created_at >= {}", prefix, st));
    }
    if let Some(et) = end_time {
        conditions.push(format!("{}created_at <= {}", prefix, et));
    }
    conditions.join(" AND ")
}

fn compute_distribution(conn: &Connection, sql: &str) -> Result<Option<StatDistribution>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let values: Vec<f64> = stmt
        .query_map([], |row| row.get::<_, f64>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    if values.is_empty() {
        return Ok(None);
    }

    let count = values.len() as i64;
    let mut sorted = values;
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let sum: f64 = sorted.iter().sum();
    let avg = sum / count as f64;

    let p50 = percentile(&sorted, 50.0);
    let p80 = percentile(&sorted, 80.0);
    let p95 = percentile(&sorted, 95.0);

    Ok(Some(StatDistribution {
        min,
        max,
        avg,
        p50,
        p80,
        p95,
        count,
    }))
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p / 100.0 * (sorted.len() - 1) as f64) as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn feedback_rating_from_array(feedbacks: &serde_json::Value) -> Option<String> {
    feedbacks
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("rating").or_else(|| item.get("value")).or_else(|| item.get("type")))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}
