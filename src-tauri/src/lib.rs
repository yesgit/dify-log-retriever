mod db;
mod dify_api;
mod export;
mod models;

use tauri::State;
use std::collections::HashSet;

use db::Database;
use dify_api::DifyApiClient;
use models::*;

struct AppState {
    db: Database,
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<Option<DifyConfigDisplay>, String> {
    state.db.get_config_display()
}

#[tauri::command]
fn save_config(
    state: State<AppState>,
    api_base: String,
    api_key: String,
    proxy: Option<String>,
    auth_mode: Option<String>,
    auth_email: Option<String>,
    auth_password: Option<String>,
) -> Result<(), String> {
    // If the frontend sends __KEEP_EXISTING__, keep the old key
    let actual_key = if api_key == "__KEEP_EXISTING__" {
        let existing = state.db.get_config()?;
        match existing {
            Some(c) if !c.api_key.is_empty() => c.api_key,
            _ => return Err("当前没有已保存的 API Token，请输入新的 Token".to_string()),
        }
    } else {
        api_key
    };

    // Determine auth_email and auth_password based on auth_mode
    let (final_email, final_password) = match auth_mode.as_deref().unwrap_or("token") {
        "login" => {
            let e = auth_email.filter(|e| !e.trim().is_empty());
            let p = auth_password.filter(|p| !p.trim().is_empty());

            match (e, p) {
                // Both provided: use new values (fresh login or credential change)
                (Some(email), Some(password)) => (Some(email), Some(password)),
                // Neither provided: keep existing credentials
                (None, None) => {
                    let existing = state.db.get_config()?;
                    match existing {
                        Some(c) => (c.auth_email, c.auth_password),
                        None => (None, None),
                    }
                }
                // Only one provided: reject to prevent credential mismatch
                (Some(_), None) => {
                    return Err("修改登录凭据时，邮箱和密码必须同时提供".to_string());
                }
                (None, Some(_)) => {
                    return Err("修改登录凭据时，邮箱和密码必须同时提供".to_string());
                }
            }
        }
        _ => (None, None), // token mode: clear credentials
    };

    let config = DifyConfig {
        api_base,
        api_key: actual_key,
        proxy,
        auth_email: final_email,
        auth_password: final_password,
    };
    state.db.save_config(&config)
}

#[tauri::command]
async fn login_to_dify(
    state: State<'_, AppState>,
    api_base: String,
    email: String,
    password: String,
    proxy: Option<String>,
) -> Result<String, String> {
    let login_resp = DifyApiClient::login(&api_base, &email, &password, proxy.as_deref()).await?;

    // Save the token and credentials
    let config = DifyConfig {
        api_base: api_base.trim_end_matches('/').to_string(),
        api_key: login_resp.access_token.clone(),
        proxy: proxy.filter(|p| !p.trim().is_empty()),
        auth_email: Some(email),
        auth_password: Some(password),
    };
    state.db.save_config(&config)?;

    Ok(login_resp.access_token)
}

/// Try to auto-refresh the token by re-logging in with stored credentials.
/// Returns the new config if successful, or the original error if not.
async fn try_auto_refresh(db: &Database) -> Result<DifyConfig, String> {
    let config = db.get_config()?.ok_or("请先配置连接信息")?;

    let email = config.auth_email.clone().ok_or("Token 已过期，请重新登录")?;
    let password = config.auth_password.clone().ok_or("Token 已过期，请重新登录")?;

    let login_resp = DifyApiClient::login(
        &config.api_base,
        &email,
        &password,
        config.proxy.as_deref(),
    )
    .await?;

    // Update the stored token
    db.update_api_key(&login_resp.access_token)?;

    Ok(DifyConfig {
        api_key: login_resp.access_token,
        ..config
    })
}

#[tauri::command]
async fn test_connection(api_base: String, api_key: String, proxy: Option<String>) -> Result<usize, String> {
    // If key is masked placeholder, we can't test without the real key
    if api_key == "__KEEP_EXISTING__" {
        return Err("请重新输入 API Token 后再测试连接".to_string());
    }
    let client = DifyApiClient::new(&api_base, &api_key, proxy.as_deref())?;
    let apps = client.fetch_apps().await?;
    Ok(apps.len())
}

#[tauri::command]
async fn fetch_apps_from_dify(state: State<'_, AppState>) -> Result<Vec<DifyApp>, String> {
    let config = state.db.get_config()?.ok_or("请先配置连接信息")?;
    let client = DifyApiClient::new(&config.api_base, &config.api_key, config.proxy.as_deref())?;
    let apps_result = client.fetch_all_apps().await;

    // Auto-refresh on auth error
    let apps = match apps_result {
        Ok(apps) => apps,
        Err(ref e) if DifyApiClient::is_auth_error(e) => {
            let refreshed_config = try_auto_refresh(&state.db).await?;
            let new_client = DifyApiClient::new(
                &refreshed_config.api_base,
                &refreshed_config.api_key,
                refreshed_config.proxy.as_deref(),
            )?;
            new_client.fetch_all_apps().await?
        }
        Err(e) => return Err(e),
    };

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
    incremental: Option<bool>,
) -> Result<SyncResult, String> {
    let config = state.db.get_config()?.ok_or("请先配置连接信息")?;
    let client = DifyApiClient::new(&config.api_base, &config.api_key, config.proxy.as_deref())?;

    // Try initial request; auto-refresh on auth error
    let client = match client.fetch_conversations(&app_id, 1, 1).await {
        Ok(_) => client,
        Err(ref e) if DifyApiClient::is_auth_error(e) => {
            let refreshed_config = try_auto_refresh(&state.db).await?;
            DifyApiClient::new(
                &refreshed_config.api_base,
                &refreshed_config.api_key,
                refreshed_config.proxy.as_deref(),
            )?
        }
        Err(e) => return Err(e),
    };

    let is_incremental = incremental.unwrap_or(false);

    let mut total_conversations: i64 = 0;
    let mut synced_conversations: i64 = 0;
    let mut total_messages: i64 = 0;
    let mut synced_messages: i64 = 0;
    let mut synced_workflow_runs: i64 = 0;
    let mut synced_node_executions: i64 = 0;
    let mut failed_details: i64 = 0;
    let mut new_conversations: i64 = 0;
    let mut updated_conversations: i64 = 0;
    let mut skipped_conversations: i64 = 0;
    let mut fetched_workflow_runs: HashSet<String> = HashSet::new();
    let mut page: i64 = 1;

    loop {
        let conv_resp = client.fetch_conversations(&app_id, 100, page).await?;

        // In incremental mode, check which conversations have changed
        let local_updated_map = if is_incremental && !conv_resp.data.is_empty() {
            let ids: Vec<String> = conv_resp.data.iter().map(|c| c.id.clone()).collect();
            state.db.get_conversations_updated_at(&app_id, &ids)?
        } else {
            std::collections::HashMap::new()
        };

        let mut page_skipped = 0;

        for conv in &conv_resp.data {
            // In incremental mode, check if conversation has changed
            if is_incremental {
                if let Some(local_updated) = local_updated_map.get(&conv.id) {
                    if *local_updated == conv.updated_at {
                        // Conversation hasn't changed, skip
                        skipped_conversations += 1;
                        page_skipped += 1;
                        continue;
                    } else {
                        // Conversation has been updated
                        updated_conversations += 1;
                    }
                } else {
                    // New conversation
                    new_conversations += 1;
                }
            }

            total_conversations += 1;
            state.db.upsert_conversation(&app_id, conv)?;
            synced_conversations += 1;

            match client.fetch_conversation_detail(&app_id, &conv.id).await {
                Ok(mut detail) => {
                    fill_missing_conversation_detail(&mut detail, conv);
                    state.db.upsert_conversation(&app_id, &detail)?;
                }
                Err(_) => {
                    failed_details += 1;
                }
            }

            let messages = client.fetch_messages(&app_id, &conv.id, 100).await?;
            total_messages += messages.len() as i64;

            for msg in &messages {
                state.db.upsert_message(&app_id, &conv.id, msg)?;
                synced_messages += 1;

                if let Some(run_id) = msg.workflow_run_id.as_deref().filter(|id| !id.is_empty()) {
                    let cache_key = format!("{}:{}", app_id, run_id);
                    if fetched_workflow_runs.insert(cache_key) {
                        match client.fetch_workflow_run(&app_id, run_id).await {
                            Ok(run) => {
                                state.db.upsert_workflow_run(&app_id, &run)?;
                                synced_workflow_runs += 1;
                            }
                            Err(_) => {
                                failed_details += 1;
                            }
                        }

                        match client.fetch_node_executions(&app_id, run_id).await {
                            Ok(nodes) => {
                                for node in &nodes {
                                    state.db.upsert_node_execution(&app_id, run_id, node)?;
                                    synced_node_executions += 1;
                                }
                            }
                            Err(_) => {
                                failed_details += 1;
                            }
                        }
                    }
                }
            }
        }

        // In incremental mode, if entire page was skipped, stop paginating
        if is_incremental && !conv_resp.data.is_empty() && page_skipped == conv_resp.data.len() {
            break;
        }

        if conv_resp.has_more {
            page += 1;
        } else {
            break;
        }
    }

    Ok(SyncResult {
        total_conversations,
        synced_conversations,
        total_messages,
        synced_messages,
        synced_workflow_runs,
        synced_node_executions,
        failed_details,
        new_conversations,
        updated_conversations,
        skipped_conversations,
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
    app_id: Option<String>,
) -> Result<Vec<MessageDetail>, String> {
    state.db.get_messages(app_id.as_deref(), &conversation_id)
}

#[tauri::command]
fn get_dashboard_stats(
    state: State<AppState>,
    app_id: Option<String>,
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> Result<DashboardStats, String> {
    state.db.get_dashboard_stats(app_id.as_deref(), start_time, end_time)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
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

    let ext = format;
    let default_filename = format!("dify_export_{}.{}", chrono::Local::now().format("%Y%m%d_%H%M%S"), ext);

    export::save_export_file_with_dialog(&content, &default_filename, &ext)
}

#[tauri::command]
fn get_feedback_messages(
    state: State<AppState>,
    app_id: Option<String>,
    feedback_type: Option<String>,
    keyword: Option<String>,
    page: i64,
    page_size: i64,
) -> Result<FeedbackResult, String> {
    state.db.get_feedback_messages(
        app_id.as_deref(),
        feedback_type.as_deref(),
        keyword.as_deref(),
        page,
        page_size,
    )
}

#[tauri::command]
fn export_feedback_data(
    state: State<AppState>,
    format: String,
    app_id: Option<String>,
    feedback_type: Option<String>,
    keyword: Option<String>,
    save_path: Option<String>,
) -> Result<String, String> {
    // Fetch all matching feedback messages (no pagination)
    let result = state.db.get_feedback_messages(
        app_id.as_deref(),
        feedback_type.as_deref(),
        keyword.as_deref(),
        1,
        1000000,
    )?;

    if result.data.is_empty() {
        return Err("没有找到匹配的反馈数据".to_string());
    }

    let path = save_path.map(std::path::PathBuf::from);

    match format.as_str() {
        "xlsx" => export::export_feedback_to_excel(&result.data, path.as_deref()),
        "csv" => export::export_feedback_to_csv(&result.data, path.as_deref()),
        "json" => export::export_feedback_to_json(&result.data, path.as_deref()),
        _ => Err(format!("不支持的格式: {}", format)),
    }
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Open database in the app data directory
    let db_path = dirs_data_dir().join("dify_log_retriever.db");
    let db = Database::open(&db_path).expect("Failed to open database");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { db })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            login_to_dify,
            test_connection,
            fetch_apps_from_dify,
            get_local_apps,
            delete_app_data,
            sync_app_data,
            get_conversations,
            get_messages,
            get_dashboard_stats,
            export_data,
            get_feedback_messages,
            export_feedback_data,
            get_app_version,
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

fn fill_missing_conversation_detail(detail: &mut DifyConversationItem, list_item: &DifyConversationItem) {
    if detail.name.is_empty() {
        detail.name = list_item.name.clone();
    }
    if detail.summary.is_empty() {
        detail.summary = list_item.summary.clone();
    }
    if detail.status.is_empty() {
        detail.status = list_item.status.clone();
    }
    if detail.introduction.is_empty() {
        detail.introduction = list_item.introduction.clone();
    }
    if detail.from_source.is_empty() {
        detail.from_source = list_item.from_source.clone();
    }
    if detail.from_end_user_id.is_empty() {
        detail.from_end_user_id = list_item.from_end_user_id.clone();
    }
    if detail.from_end_user_session_id.is_empty() {
        detail.from_end_user_session_id = list_item.from_end_user_session_id.clone();
    }
    if detail.read_at.is_none() {
        detail.read_at = list_item.read_at;
    }
    if !detail.annotated && list_item.annotated {
        detail.annotated = true;
    }
    if is_empty_json(&detail.inputs) {
        detail.inputs = list_item.inputs.clone();
    }
    if is_empty_json(&detail.model_config) {
        detail.model_config = list_item.model_config.clone();
    }
    if is_empty_json(&detail.user_feedback_stats) {
        detail.user_feedback_stats = list_item.user_feedback_stats.clone();
    }
    if is_empty_json(&detail.admin_feedback_stats) {
        detail.admin_feedback_stats = list_item.admin_feedback_stats.clone();
    }
    if is_empty_json(&detail.status_count) {
        detail.status_count = list_item.status_count.clone();
    }
    if detail.created_at == 0 {
        detail.created_at = list_item.created_at;
    }
    if detail.updated_at == 0 {
        detail.updated_at = list_item.updated_at;
    }
}

fn is_empty_json(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::Array(items) => items.is_empty(),
        serde_json::Value::Object(map) => map.is_empty(),
        _ => false,
    }
}
