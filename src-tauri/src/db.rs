use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

use crate::models::*;

fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    format!("{}****{}", &key[..4], &key[key.len()-4..])
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

            CREATE INDEX IF NOT EXISTS idx_conversations_app_id ON conversations(app_id);
            CREATE INDEX IF NOT EXISTS idx_conversations_conversation_id ON conversations(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_conversations_created_at ON conversations(created_at);

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

            CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_messages_app_id ON messages(app_id);
            CREATE INDEX IF NOT EXISTS idx_messages_message_id ON messages(message_id);
            CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);
            "
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Settings =====
    pub fn save_config(&self, config: &DifyConfig) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["api_base", &config.api_base],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["api_key", &config.api_key],
        ).map_err(|e| e.to_string())?;
        if let Some(ref proxy) = config.proxy {
            let trimmed = proxy.trim();
            if !trimmed.is_empty() {
                conn.execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                    params!["proxy", trimmed],
                ).map_err(|e| e.to_string())?;
            } else {
                conn.execute(
                    "DELETE FROM settings WHERE key = 'proxy'",
                    [],
                ).map_err(|e| e.to_string())?;
            }
        } else {
            conn.execute(
                "DELETE FROM settings WHERE key = 'proxy'",
                [],
            ).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn get_config(&self) -> Result<Option<DifyConfig>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let api_base: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'api_base'",
                [],
                |row| row.get(0),
            )
            .ok();
        let api_key: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'api_key'",
                [],
                |row| row.get(0),
            )
            .ok();
        let proxy: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'proxy'",
                [],
                |row| row.get(0),
            )
            .ok();

        match (api_base, api_key) {
            (Some(base), Some(key)) => Ok(Some(DifyConfig {
                api_base: base,
                api_key: key,
                proxy,
            })),
            _ => Ok(None),
        }
    }

    pub fn get_config_display(&self) -> Result<Option<DifyConfigDisplay>, String> {
        let config = self.get_config()?;
        Ok(config.map(|c| {
            let masked = mask_api_key(&c.api_key);
            DifyConfigDisplay {
                api_base: c.api_base,
                api_key_masked: masked,
                proxy: c.proxy,
                has_key: !c.api_key.is_empty(),
            }
        }))
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
        conn.execute("DELETE FROM messages WHERE app_id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM conversations WHERE app_id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM apps WHERE id = ?1", params![app_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Conversations =====
    pub fn upsert_conversation(&self, app_id: &str, conv: &DifyConversationItem) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let id = format!("{}:{}", app_id, conv.id);
        let inputs = serde_json::to_string(&conv.inputs).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT OR REPLACE INTO conversations (id, app_id, conversation_id, name, inputs, status, introduction, created_at, updated_at, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, strftime('%s','now'))",
            params![id, app_id, conv.id, conv.name, inputs, conv.status, conv.introduction, conv.created_at, conv.updated_at],
        ).map_err(|e| e.to_string())?;
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
                    conditions.push("(c.name LIKE ? OR c.conversation_id LIKE ?)".to_string());
                    let pattern = format!("%{}%", kw);
                    params.push(Box::new(pattern.clone()));
                    params.push(Box::new(pattern));
                }
            }
            (conditions.join(" AND "), params)
        };

        // Count total
        let count_sql = format!("SELECT COUNT(*) FROM conversations c WHERE {}", where_clause);
        let total: i64 = if params_vec.is_empty() {
            conn.query_row(&count_sql, [], |row| row.get(0)).map_err(|e| e.to_string())?
        } else {
            let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
            conn.query_row(&count_sql, param_refs.as_slice(), |row| row.get(0)).map_err(|e| e.to_string())?
        };

        // Query data
        let data_sql = format!(
            "SELECT c.id, c.app_id, c.conversation_id, c.name, c.created_at, c.updated_at, \
             (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.conversation_id AND m.app_id = c.app_id) as message_count, \
             a.name as app_name \
             FROM conversations c \
             LEFT JOIN apps a ON c.app_id = a.id \
             WHERE {} \
             ORDER BY c.created_at DESC \
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
        let feedback_rating = msg.feedback.as_ref().map(|f| f.rating.clone());
        let retriever_resources = serde_json::to_string(&msg.retriever_resources).unwrap_or_else(|_| "[]".to_string());
        let metadata = serde_json::to_string(&msg.message_metadata).unwrap_or_else(|_| "{}".to_string());
        let agent_thoughts = serde_json::to_string(&msg.agent_thoughts).unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "INSERT OR REPLACE INTO messages (id, app_id, conversation_id, message_id, query, answer, feedback, retriever_resources, message_metadata, agent_thoughts, answer_tokens, prompt_tokens, elapsed_time, created_at, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, strftime('%s','now'))",
            params![id, app_id, conversation_id, msg.id, msg.query, msg.answer, feedback_rating, retriever_resources, metadata, agent_thoughts, msg.answer_tokens, msg.prompt_tokens, msg.elapsed_time, msg.created_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_messages(&self, conversation_id: &str) -> Result<Vec<MessageDetail>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, app_id, conversation_id, message_id, query, answer, feedback, retriever_resources, message_metadata, agent_thoughts, answer_tokens, prompt_tokens, elapsed_time, created_at \
                 FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| e.to_string())?;

        let messages: Vec<Message> = stmt
            .query_map(params![conversation_id], |row| {
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
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(messages.into_iter().map(MessageDetail::from).collect())
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
                // Include the full end day (until 23:59:59)
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
            "SELECT id, app_id, conversation_id, message_id, query, answer, feedback, retriever_resources, message_metadata, agent_thoughts, answer_tokens, prompt_tokens, elapsed_time, created_at \
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
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(messages.into_iter().map(MessageDetail::from).collect())
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

        // Top apps
        let mut stmt = conn.prepare(
            "SELECT c.app_id, a.name, COUNT(DISTINCT c.conversation_id) as conv_count, COUNT(m.id) as msg_count \
             FROM conversations c \
             LEFT JOIN apps a ON c.app_id = a.id \
             LEFT JOIN messages m ON m.app_id = c.app_id AND m.conversation_id = c.conversation_id \
             GROUP BY c.app_id \
             ORDER BY conv_count DESC \
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

        // Recent 7 days
        let mut stmt = conn.prepare(
            "SELECT date(created_at, 'unixepoch', 'localtime') as day, \
             COUNT(DISTINCT conversation_id) as conv_count, \
             COUNT(*) as msg_count \
             FROM messages \
             WHERE created_at >= strftime('%s', 'now', '-7 days') \
             GROUP BY day \
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