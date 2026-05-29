mod db;
mod dify_api;
mod export;
mod models;

use tauri::State;

use db::Database;
use dify_api::DifyApiClient;
use models::*;

struct AppState {
    db: Database,
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<Option<DifyConfig>, String> {
    state.db.get_config()
}

#[tauri::command]
fn save_config(state: State<AppState>, api_base: String, api_key: String) -> Result<(), String> {
    let config = DifyConfig { api_base, api_key };
    state.db.save_config(&config)
}

#[tauri::command]
async fn test_connection(api_base: String, api_key: String) -> Result<usize, String> {
    let client = DifyApiClient::new(&api_base, &api_key);
    let apps = client.fetch_apps().await?;
    Ok(apps.len())
}

#[tauri::command]
async fn fetch_apps_from_dify(state: State<'_, AppState>) -> Result<Vec<DifyApp>, String> {
    let config = state.db.get_config()?.ok_or("请先配置连接信息")?;
    let client = DifyApiClient::new(&config.api_base, &config.api_key);
    let apps = client.fetch_apps().await?;

    for app in &apps {
        let local_app = DifyApp {
            id: app.id.clone(),
            name: app.name.clone(),
            description: app.description.clone(),
            mode: app.mode.clone(),
            icon: app.icon.clone(),
            icon_background: app.icon_background.clone(),
            created_at: app.created_at,
        };
        state.db.upsert_app(&local_app)?;
    }

    state.db.get_apps()
}

#[tauri::command]
fn get_local_apps(state: State<AppState>) -> Result<Vec<DifyApp>, String> {
    state.db.get_apps()
}

#[tauri::command]
fn delete_app_data(state: State<AppState>, app_id: String) -> Result<(), String> {
    state.db.delete_app_data(&app_id)
}

#[tauri::command]
async fn sync_app_data(
    state: State<'_, AppState>,
    app_id: String,
) -> Result<SyncResult, String> {
    let config = state.db.get_config()?.ok_or("请先配置连接信息")?;
    let client = DifyApiClient::new(&config.api_base, &config.api_key);

    let mut total_conversations: i64 = 0;
    let mut synced_conversations: i64 = 0;
    let mut total_messages: i64 = 0;
    let mut synced_messages: i64 = 0;
    let mut cursor: Option<String> = None;

    loop {
        let cursor_ref = cursor.as_deref();
        let conv_resp = client.fetch_conversations(&app_id, 100, cursor_ref).await?;
        total_conversations += conv_resp.data.len() as i64;

        for conv in &conv_resp.data {
            // Save conversation
            state.db.upsert_conversation(&app_id, conv)?;
            synced_conversations += 1;

            // Fetch messages for this conversation
            let messages = client.fetch_messages(&app_id, &conv.id, 100).await?;
            total_messages += messages.len() as i64;

            for msg in &messages {
                state.db.upsert_message(&app_id, &conv.id, msg)?;
                synced_messages += 1;
            }
        }

        if conv_resp.has_more {
            // Use last conversation id as cursor
            if let Some(last) = conv_resp.data.last() {
                cursor = Some(last.id.clone());
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(SyncResult {
        total_conversations,
        synced_conversations,
        total_messages,
        synced_messages,
    })
}

#[tauri::command]
fn get_conversations(
    state: State<AppState>,
    app_id: Option<String>,
    keyword: Option<String>,
    page: i64,
    page_size: i64,
) -> Result<ConversationsResult, String> {
    state.db.get_conversations(
        app_id.as_deref(),
        keyword.as_deref(),
        page,
        page_size,
    )
}

#[tauri::command]
fn get_messages(
    state: State<AppState>,
    conversation_id: String,
) -> Result<Vec<MessageDetail>, String> {
    state.db.get_messages(&conversation_id)
}

#[tauri::command]
fn get_dashboard_stats(state: State<AppState>) -> Result<DashboardStats, String> {
    state.db.get_dashboard_stats()
}

#[tauri::command]
fn export_data(
    state: State<AppState>,
    format: String,
    app_id: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    keyword: Option<String>,
    include_metadata: bool,
    include_agent_thoughts: bool,
) -> Result<String, String> {
    let messages = state.db.get_messages_for_export(
        app_id.as_deref(),
        start_date.as_deref(),
        end_date.as_deref(),
        keyword.as_deref(),
    )?;

    if messages.is_empty() {
        return Err("没有找到匹配的数据".to_string());
    }

    let content = match format.as_str() {
        "json" => export::export_to_json(&messages, include_metadata, include_agent_thoughts)?,
        "csv" => export::export_to_csv(&messages)?,
        "jsonl" => export::export_to_jsonl(&messages, include_metadata, include_agent_thoughts)?,
        _ => return Err(format!("不支持的格式: {}", format)),
    };

    export::save_export_file(&content, &format)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Open database in the app data directory
    let db_path = dirs_data_dir().join("dify_log_retriever.db");
    let db = Database::open(&db_path).expect("Failed to open database");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState { db })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            test_connection,
            fetch_apps_from_dify,
            get_local_apps,
            delete_app_data,
            sync_app_data,
            get_conversations,
            get_messages,
            get_dashboard_stats,
            export_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn dirs_data_dir() -> std::path::PathBuf {
    // Cross-platform data directory
    // Linux: ~/.local/share/dify-log-retriever
    // macOS: ~/Library/Application Support/dify-log-retriever
    // Windows: C:\Users\<User>\AppData\Roaming\dify-log-retriever
    if let Some(data_dir) = dirs::data_dir() {
        let app_dir = data_dir.join("dify-log-retriever");
        std::fs::create_dir_all(&app_dir).ok();
        app_dir
    } else {
        let fallback = std::path::PathBuf::from(".dify-log-retriever");
        std::fs::create_dir_all(&fallback).ok();
        fallback
    }
}
