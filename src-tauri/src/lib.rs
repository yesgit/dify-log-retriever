mod db;
mod dify_api;
mod dsl;
mod export;
mod models;

use tauri::State;
use std::collections::{HashMap, HashSet};

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
    sync_workflow_details: Option<bool>,
) -> Result<SyncResult, String> {
    let config = state.db.get_config()?.ok_or("请先配置连接信息")?;
    let client = DifyApiClient::new(&config.api_base, &config.api_key, config.proxy.as_deref())?;

    // Determine app mode from local DB
    let apps = state.db.get_apps()?;
    let app_mode = apps
        .iter()
        .find(|a| a.id == app_id)
        .map(|a| a.mode.clone())
        .unwrap_or_default();

    // Auto-refresh on auth error
    let client = {
        let probe_result = if app_mode == "workflow" {
            client.fetch_workflow_app_logs(&app_id, 1, 1).await.map(|_| ())
        } else {
            client.fetch_conversations(&app_id, 1, 1).await.map(|_| ())
        };
        match probe_result {
            Ok(()) => Ok(client),
            Err(ref e) if DifyApiClient::is_auth_error(e) => {
                let refreshed_config = try_auto_refresh(&state.db).await?;
                DifyApiClient::new(
                    &refreshed_config.api_base,
                    &refreshed_config.api_key,
                    refreshed_config.proxy.as_deref(),
                )
            }
            Err(e) => Err(e),
        }
    }?;

    let fetch_wf = sync_workflow_details.unwrap_or(false);

    if app_mode == "workflow" {
        sync_workflow_app(&state, &client, &app_id, incremental.unwrap_or(false), fetch_wf).await
    } else {
        sync_chat_app(&state, &client, &app_id, incremental.unwrap_or(false), fetch_wf).await
    }
}

/// Sync workflow-type app using workflow-app-logs API
async fn sync_workflow_app(
    state: &State<'_, AppState>,
    client: &DifyApiClient,
    app_id: &str,
    is_incremental: bool,
    fetch_workflow_details: bool,
) -> Result<SyncResult, String> {
    let mut total_conversations: i64 = 0;
    let mut synced_conversations: i64 = 0;
    let mut total_messages: i64 = 0;
    let mut synced_messages: i64 = 0;
    let mut synced_workflow_runs: i64 = 0;
    let mut synced_node_executions: i64 = 0;
    let mut failed_details: i64 = 0;
    let mut fetched_workflow_runs: HashSet<String> = HashSet::new();
    let mut workflow_tasks: Vec<tokio::task::JoinHandle<(String, Result<DifyWorkflowRun, String>, Result<Vec<DifyNodeExecution>, String>)>> = Vec::new();
    let mut page: i64 = 1;
    let limit: i64 = 100;

    // For incremental sync, get the latest created_at from local DB to know when to stop
    let max_local_created_at: Option<i64> = if is_incremental {
        state.db.get_workflow_app_log_max_created_at(app_id)?
    } else {
        None
    };

    loop {
        let logs_resp = client.fetch_workflow_app_logs(app_id, page, limit).await?;
        total_messages += logs_resp.data.len() as i64;

        let mut early_stop = false;
        for log_item in &logs_resp.data {
            // In incremental mode, stop when we reach data older than our latest local record.
            // Use < (strict less-than) to re-process any logs at the same timestamp boundary,
            // ensuring we don't miss logs that share the same created_at second.
            // Note: use break (not return) to ensure spawned workflow_tasks are flushed below.
            if is_incremental {
                if let Some(max_ts) = max_local_created_at {
                    if log_item.created_at < max_ts {
                        early_stop = true;
                        break;
                    }
                }
            }

            state.db.upsert_workflow_app_log(app_id, log_item)?;

            // Create conversation record so workflow data shows in conversation list UI
            let run_id_for_name = &log_item.workflow_run.id;
            let display_name = if !run_id_for_name.is_empty() {
                let truncated: String = run_id_for_name.chars().take(8).collect();
                if run_id_for_name.chars().count() > 8 {
                    format!("Workflow: {}...", truncated)
                } else {
                    format!("Workflow: {}", truncated)
                }
            } else {
                "Workflow Run".to_string()
            };

            let conv = DifyConversationItem {
                id: log_item.id.clone(),
                name: display_name,
                summary: format!("Status: {}", log_item.workflow_run.status),
                inputs: serde_json::Value::Null,
                status: log_item.workflow_run.status.clone(),
                introduction: String::new(),
                from_source: log_item.created_from.clone(),
                from_end_user_id: log_item.created_by_end_user.as_ref().map(|u| u.id.clone()).unwrap_or_default(),
                from_end_user_session_id: log_item.created_by_end_user.as_ref().map(|u| u.session_id.clone()).unwrap_or_default(),
                read_at: None,
                annotated: false,
                model_config: serde_json::Value::Null,
                user_feedback_stats: serde_json::Value::Null,
                admin_feedback_stats: serde_json::Value::Null,
                status_count: serde_json::Value::Null,
                created_at: log_item.created_at,
                updated_at: if log_item.workflow_run.finished_at > 0 { log_item.workflow_run.finished_at } else { log_item.created_at },
                raw_json: serde_json::Value::Null,
            };
            state.db.upsert_conversation(app_id, &conv)?;
            total_conversations += 1;
            synced_conversations += 1;

            // Create message record so workflow run details appear in message detail UI
            let workflow_run_id = if log_item.workflow_run.id.is_empty() { None } else { Some(log_item.workflow_run.id.clone()) };
            let msg = DifyMessageItem {
                id: log_item.id.clone(),
                conversation_id: log_item.id.clone(),
                inputs: serde_json::Value::Null,
                query: String::from("Workflow Execution"),
                message: String::new(),
                answer: format!("Status: {}, Tokens: {}, Time: {:.2}s, Steps: {}",
                    log_item.workflow_run.status,
                    log_item.workflow_run.total_tokens,
                    log_item.workflow_run.elapsed_time,
                    log_item.workflow_run.total_steps),
                feedback: None,
                feedbacks: serde_json::Value::Null,
                retriever_resources: serde_json::Value::Null,
                message_metadata: serde_json::Value::Null,
                agent_thoughts: serde_json::Value::Null,
                message_files: serde_json::Value::Null,
                annotation: serde_json::Value::Null,
                annotation_hit_history: serde_json::Value::Null,
                status: log_item.workflow_run.status.clone(),
                error: log_item.workflow_run.error.clone().unwrap_or(serde_json::Value::Null),
                parent_message_id: String::new(),
                workflow_run_id,
                answer_tokens: log_item.workflow_run.total_tokens,
                prompt_tokens: 0,
                message_tokens: 0,
                provider_response_latency: 0.0,
                elapsed_time: log_item.workflow_run.elapsed_time,
                created_at: log_item.created_at,
                raw_json: serde_json::Value::Null,
            };
            state.db.upsert_message(app_id, &log_item.id, &msg)?;
            synced_messages += 1;

            let run_id = &log_item.workflow_run.id;
            if fetch_workflow_details && !run_id.is_empty() {
                let cache_key = format!("{}:{}", app_id, run_id);
                if fetched_workflow_runs.insert(cache_key) {
                    let rid = run_id.clone();
                    let aid = app_id.to_string();
                    let c = client.clone();
                    let handle = tokio::spawn(async move {
                        let run_result = c.fetch_workflow_run(&aid, &rid).await;
                        let nodes_result = c.fetch_node_executions(&aid, &rid).await;
                        (rid, run_result, nodes_result)
                    });
                    workflow_tasks.push(handle);
                }
            }
        }

        // Flush pending workflow tasks at end of each page
        for task in workflow_tasks.drain(..) {
            if let Ok((run_id, run_result, nodes_result)) = task.await {
                if let Ok(run) = run_result {
                    match state.db.upsert_workflow_run(app_id, &run) {
                        Ok(_) => synced_workflow_runs += 1,
                        Err(_) => failed_details += 1,
                    }
                } else {
                    failed_details += 1;
                }
                if let Ok(nodes) = nodes_result {
                    if !nodes.is_empty() {
                        match state.db.batch_upsert_node_executions(app_id, &run_id, &nodes) {
                            Ok(count) => synced_node_executions += count as i64,
                            Err(_) => failed_details += 1,
                        }
                    }
                } else {
                    failed_details += 1;
                }
            }
        }

        if early_stop || !logs_resp.has_more {
            break;
        }
        page += 1;
    }

    Ok(SyncResult {
        total_conversations,
        synced_conversations,
        total_messages,
        synced_messages,
        synced_workflow_runs,
        synced_node_executions,
        failed_details,
        new_conversations: 0,
        updated_conversations: 0,
        skipped_conversations: 0,
    })
}

/// Sync chat/agent-type app using conversations + messages API
async fn sync_chat_app(
    state: &State<'_, AppState>,
    client: &DifyApiClient,
    app_id: &str,
    is_incremental: bool,
    fetch_workflow_details: bool,
) -> Result<SyncResult, String> {
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
        let conv_resp = client.fetch_conversations(app_id, 100, page).await?;

        // In incremental mode, check which conversations have changed
        let local_updated_map = if is_incremental && !conv_resp.data.is_empty() {
            let ids: Vec<String> = conv_resp.data.iter().map(|c| c.id.clone()).collect();
            state.db.get_conversations_updated_at(app_id, &ids)?
        } else {
            std::collections::HashMap::new()
        };

        let mut page_skipped = 0;

        for conv in &conv_resp.data {
            // In incremental mode, check if conversation has changed
            if is_incremental {
                if let Some(local_updated) = local_updated_map.get(&conv.id) {
                    if *local_updated == conv.updated_at {
                        skipped_conversations += 1;
                        page_skipped += 1;
                        continue;
                    } else {
                        updated_conversations += 1;
                    }
                } else {
                    new_conversations += 1;
                }
            }

            total_conversations += 1;
            state.db.upsert_conversation(app_id, conv)?;
            synced_conversations += 1;

            match client.fetch_conversation_detail(app_id, &conv.id).await {
                Ok(mut detail) => {
                    fill_missing_conversation_detail(&mut detail, conv);
                    state.db.upsert_conversation(app_id, &detail)?;
                }
                Err(_) => {
                    failed_details += 1;
                }
            }

            let messages = client.fetch_messages(app_id, &conv.id, 100).await?;
            total_messages += messages.len() as i64;

            // Batch insert messages in a single transaction
            if !messages.is_empty() {
                match state.db.batch_upsert_messages(app_id, &conv.id, &messages) {
                    Ok(count) => synced_messages += count as i64,
                    Err(_) => {
                        for msg in &messages {
                            match state.db.upsert_message(app_id, &conv.id, msg) {
                                Ok(_) => synced_messages += 1,
                                Err(_) => failed_details += 1,
                            }
                        }
                    }
                }
            }

            // Collect unique workflow run IDs to fetch in parallel (only if workflow details enabled)
            let run_ids: Vec<String> = if fetch_workflow_details {
                messages.iter()
                    .filter_map(|msg| msg.workflow_run_id.as_deref().filter(|id| !id.is_empty()).map(|id| id.to_string()))
                    .filter(|run_id| fetched_workflow_runs.insert(format!("{}:{}", app_id, run_id)))
                    .collect()
            } else {
                Vec::new()
            };

            // Fetch workflow runs and node executions in parallel (batches of 4)
            for run_id_chunk in run_ids.chunks(4) {
                let mut tasks = Vec::new();
                for run_id in run_id_chunk {
                    let client = client.clone();
                    let app_id = app_id.to_string();
                    let run_id = run_id.clone();
                    tasks.push(tokio::spawn(async move {
                        let run_result = client.fetch_workflow_run(&app_id, &run_id).await;
                        let nodes_result = client.fetch_node_executions(&app_id, &run_id).await;
                        (run_id, run_result, nodes_result)
                    }));
                }
                for task in tasks {
                    if let Ok((run_id, run_result, nodes_result)) = task.await {
                        if let Ok(run) = run_result {
                            match state.db.upsert_workflow_run(app_id, &run) {
                                Ok(_) => synced_workflow_runs += 1,
                                Err(_) => failed_details += 1,
                            }
                        } else {
                            failed_details += 1;
                        }
                        if let Ok(nodes) = nodes_result {
                            if !nodes.is_empty() {
                                match state.db.batch_upsert_node_executions(app_id, &run_id, &nodes) {
                                    Ok(count) => synced_node_executions += count as i64,
                                    Err(_) => failed_details += 1,
                                }
                            }
                        } else {
                            failed_details += 1;
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
    let agg_status = state.db.get_aggregation_status()?;
    if agg_status.last_aggregated_at.is_some() {
        state.db.get_dashboard_stats_from_agg(app_id.as_deref(), start_time, end_time)
    } else {
        state.db.get_dashboard_stats(app_id.as_deref(), start_time, end_time)
    }
}

#[tauri::command]
fn rebuild_dashboard_stats(state: State<AppState>) -> Result<String, String> {
    state.db.rebuild_dashboard_stats()
}

#[tauri::command]
fn get_aggregation_status(state: State<AppState>) -> Result<AggregationStatus, String> {
    state.db.get_aggregation_status()
}

#[tauri::command]
fn get_performance_stats(
    state: State<AppState>,
    app_id: Option<String>,
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> Result<PerformanceStats, String> {
    state.db.get_performance_stats(app_id.as_deref(), start_time, end_time)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn export_data(
    state: State<AppState>,
    export_type: Option<String>,
    format: String,
    app_id: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    keyword: Option<String>,
    include_metadata: bool,
    include_agent_thoughts: bool,
    save_path: Option<String>,
) -> Result<String, String> {
    let export_type = export_type.as_deref().unwrap_or("message");

    match export_type {
        "message" => {
            let messages = state.db.get_messages_for_export(
                app_id.as_deref(),
                start_date.as_deref(),
                end_date.as_deref(),
                keyword.as_deref(),
            )?;

            if messages.is_empty() {
                return Err("没有找到匹配的消息数据".to_string());
            }

            match format.as_str() {
                "xlsx" => {
                    let path = save_path.as_deref().map(std::path::Path::new);
                    return export::export_messages_to_excel(&messages, include_metadata, include_agent_thoughts, path);
                }
                "json" => export::export_messages_to_json(&messages, include_metadata, include_agent_thoughts),
                "csv" => export::export_messages_to_csv(&messages),
                "jsonl" => export::export_messages_to_jsonl(&messages, include_metadata, include_agent_thoughts),
                _ => return Err(format!("不支持的格式: {}", format)),
            }
        }
        _ => {
            let conversations = state.db.get_conversations_for_export(
                app_id.as_deref(),
                start_date.as_deref(),
                end_date.as_deref(),
                keyword.as_deref(),
            )?;

            if conversations.is_empty() {
                return Err("没有找到匹配的会话数据".to_string());
            }

            match format.as_str() {
                "xlsx" => {
                    let path = save_path.as_deref().map(std::path::Path::new);
                    return export::export_conversations_to_excel(&conversations, path);
                }
                "json" => export::export_conversations_to_json(&conversations),
                "csv" => export::export_conversations_to_csv(&conversations),
                "jsonl" => export::export_conversations_to_jsonl(&conversations),
                _ => return Err(format!("不支持的格式: {}", format)),
            }
        }
    }
    .and_then(|content| {
        if let Some(sp) = save_path {
            let path = std::path::PathBuf::from(&sp);
            std::fs::write(&path, &content).map_err(|e| format!("写入文件失败: {}", e))?;
            Ok(format!("已导出到: {}", path.display()))
        } else {
            let ext = format;
            let default_filename = format!("dify_export_{}.{}", chrono::Local::now().format("%Y%m%d_%H%M%S"), ext);
            export::save_export_file_with_dialog(&content, &default_filename, &ext)
        }
    })
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
fn get_app_node_types(
    state: State<AppState>,
    app_id: String,
) -> Result<Vec<NodeTypeSummary>, String> {
    state.db.get_app_node_types(&app_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn export_node_eval_data(
    state: State<'_, AppState>,
    format: String,
    app_id: String,
    node_type: Option<String>,
    node_id: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    save_path: Option<String>,
) -> Result<String, String> {
    let records = state.db.get_node_executions_for_export(
        &app_id,
        node_type.as_deref(),
        node_id.as_deref(),
        start_date.as_deref(),
        end_date.as_deref(),
    )?;

    if records.is_empty() {
        return Err("没有找到匹配的节点执行数据。请确认该应用已同步，且存在成功的 LLM/Agent 节点执行记录。".to_string());
    }

    // Fetch the app's latest DSL from the Dify console at export time (not from
    // local backups) and parse out per-node model params (temperature,
    // structured-output schema, ...). A fetch failure aborts the export so that
    // every exported record carries complete, reproducible parameters.
    let dsl_configs = fetch_dsl_configs(&state, &app_id).await?;

    if let Some(sp) = save_path {
        let result = export::export_node_eval(&records, &format, &dsl_configs)?;
        let path = std::path::PathBuf::from(&sp);
        std::fs::write(&path, &result.content).map_err(|e| format!("写入文件失败: {}", e))?;
        let base = format!("已导出到: {}", path.display());
        Ok(export::append_unmatched_note(&base, result.total, result.unmatched))
    } else {
        export::export_node_eval_to_file(&records, &format, &dsl_configs)
    }
}

/// Fetch an app's workflow DSL from the Dify console and parse it into a
/// `node_id -> NodeDslConfig` map. Auto-refreshes the access token once on auth
/// errors, mirroring `backup_all_dsl`.
async fn fetch_dsl_configs(
    state: &State<'_, AppState>,
    app_id: &str,
) -> Result<HashMap<String, NodeDslConfig>, String> {
    let config = state.db.get_config()?.ok_or("请先配置连接信息")?;
    let mut client = DifyApiClient::new(&config.api_base, &config.api_key, config.proxy.as_deref())?;

    let dsl_yaml = match client.fetch_app_dsl(app_id, false).await {
        Ok(c) => c,
        Err(ref e) if DifyApiClient::is_auth_error(e) => {
            let refreshed = try_auto_refresh(&state.db).await?;
            client = DifyApiClient::new(
                &refreshed.api_base,
                &refreshed.api_key,
                refreshed.proxy.as_deref(),
            )?;
            client
                .fetch_app_dsl(app_id, false)
                .await
                .map_err(|e| format!("拉取应用 DSL 失败: {}", e))?
        }
        Err(e) => return Err(format!("拉取应用 DSL 失败: {}", e)),
    };

    dsl::parse_node_configs(&dsl_yaml)
}

// ===== Per-App Sync Config =====
#[tauri::command]
fn get_sync_config(state: State<AppState>) -> Result<SyncConfig, String> {
    state.db.get_sync_config()
}

#[tauri::command]
fn save_sync_config(state: State<AppState>, config: SyncConfig) -> Result<(), String> {
    state.db.save_sync_config(&config)
}

#[tauri::command]
fn delete_app_sync_data(state: State<AppState>, app_id: String) -> Result<(), String> {
    state.db.delete_app_sync_data(&app_id)
}

#[tauri::command]
fn delete_app_workflow_details(state: State<AppState>, app_id: String) -> Result<(), String> {
    state.db.delete_app_workflow_details(&app_id)
}

#[tauri::command]
fn get_app_sync_data_info(state: State<AppState>, app_id: String) -> Result<AppSyncDataInfo, String> {
    state.db.get_app_sync_data_info(&app_id)
}

#[tauri::command]
fn get_auto_sync_settings(state: State<AppState>) -> Result<AutoSyncSettings, String> {
    state.db.get_auto_sync_settings()
}

#[tauri::command]
fn save_auto_sync_settings(state: State<AppState>, settings: AutoSyncSettings) -> Result<(), String> {
    state.db.save_auto_sync_settings(&settings)
}

/// Sync all local apps (used by auto-sync). Returns a summary.
#[tauri::command]
async fn sync_all_apps(state: State<'_, AppState>, incremental: Option<bool>) -> Result<String, String> {
    let apps = state.db.get_apps()?;
    if apps.is_empty() {
        return Ok("没有需要同步的应用".to_string());
    }

    let is_incremental = incremental.unwrap_or(true);
    let mut success_count = 0;
    let mut error_count = 0;
    let mut total_synced_conv = 0;
    let mut total_synced_msg = 0;
    let mut error_details: Vec<String> = Vec::new();

    // Get sync config to know which apps to sync and whether to fetch workflow details
    let sync_config = state.db.get_sync_config().unwrap_or(SyncConfig { apps: Vec::new() });
    let sync_config_map: std::collections::HashMap<String, (bool, bool)> = sync_config.apps.iter()
        .map(|s| (s.app_id.clone(), (s.enabled, s.sync_workflow_details)))
        .collect();

    for app in &apps {
        // Check if this app is enabled in sync config
        if let Some((enabled, sync_wf)) = sync_config_map.get(&app.id) {
            if !enabled {
                continue;
            }
            match sync_app_data(
                state.clone(),
                app.id.clone(),
                Some(is_incremental),
                Some(*sync_wf),
            )
            .await
            {
                Ok(result) => {
                    success_count += 1;
                    total_synced_conv += result.synced_conversations;
                    total_synced_msg += result.synced_messages;
                }
                Err(e) => {
                    error_count += 1;
                    error_details.push(format!("{}: {}", app.name, e));
                }
            }
        } else {
            // No config for this app - sync with defaults (enabled, no workflow details)
            match sync_app_data(
                state.clone(),
                app.id.clone(),
                Some(is_incremental),
                None,
            )
            .await
            {
                Ok(result) => {
                    success_count += 1;
                    total_synced_conv += result.synced_conversations;
                    total_synced_msg += result.synced_messages;
                }
                Err(e) => {
                    error_count += 1;
                    error_details.push(format!("{}: {}", app.name, e));
                }
            }
        }
    }

    // Update last synced timestamp
    let now = chrono::Utc::now().timestamp();
    let _ = state.db.update_auto_sync_last_synced(now);

    let mut summary = format!(
        "同步完成: {} 个应用成功, {} 个失败, 同步 {} 个对话, {} 条消息",
        success_count, error_count, total_synced_conv, total_synced_msg
    );
    if !error_details.is_empty() {
        summary.push_str(&format!("。失败详情: {}", error_details.join("; ")));
    }
    Ok(summary)
}

#[tauri::command]
fn export_dashboard_excel(
    state: State<AppState>,
    app_id: Option<String>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    save_path: Option<String>,
) -> Result<String, String> {
    let agg_status = state.db.get_aggregation_status()?;
    let stats = if agg_status.last_aggregated_at.is_some() {
        state.db.get_dashboard_stats_from_agg(app_id.as_deref(), start_time, end_time)?
    } else {
        state.db.get_dashboard_stats(app_id.as_deref(), start_time, end_time)?
    };

    // Resolve app name for the report header
    let app_name = if let Some(ref aid) = app_id {
        state.db.get_apps()
            .unwrap_or_default()
            .into_iter()
            .find(|a| a.id == *aid)
            .map(|a| a.name)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let path = save_path.map(std::path::PathBuf::from);
    export::export_dashboard_to_excel(&stats, &app_name, path.as_deref())
}

#[tauri::command]
fn get_db_size_info(state: State<AppState>) -> Result<DbSizeInfo, String> {
    state.db.get_db_size_info()
}

#[tauri::command]
fn export_performance_excel(
    state: State<AppState>,
    app_id: Option<String>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    save_path: Option<String>,
) -> Result<String, String> {
    let stats = state.db.get_performance_stats(app_id.as_deref(), start_time, end_time)?;

    let app_name = if let Some(ref aid) = app_id {
        state.db.get_apps()
            .unwrap_or_default()
            .into_iter()
            .find(|a| a.id == *aid)
            .map(|a| a.name)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let path = save_path.map(std::path::PathBuf::from);
    export::export_performance_to_excel(&stats, &app_name, path.as_deref())
}

#[tauri::command]
fn cleanup_raw_json(state: State<AppState>) -> Result<String, String> {
    let bytes = state.db.cleanup_raw_json()?;
    Ok(format!("已清理 raw_json，释放约 {} 字节 ({:.2} MB)", bytes, bytes as f64 / 1_048_576.0))
}

#[tauri::command]
fn vacuum_database(state: State<AppState>) -> Result<String, String> {
    let info_before = state.db.get_db_size_info()?;
    state.db.vacuum()?;
    let info_after = state.db.get_db_size_info()?;
    let freed = info_before.total_bytes - info_after.total_bytes;
    Ok(format!(
        "VACUUM 完成！数据库从 {:.2} MB 压缩到 {:.2} MB，释放 {:.2} MB",
        info_before.total_bytes as f64 / 1_048_576.0,
        info_after.total_bytes as f64 / 1_048_576.0,
        freed as f64 / 1_048_576.0,
    ))
}

// ===== DSL Backup =====
#[tauri::command]
fn get_dsl_backup_settings(state: State<AppState>) -> Result<DslBackupSettings, String> {
    state.db.get_dsl_backup_settings()
}

#[tauri::command]
fn save_dsl_backup_settings(state: State<AppState>, settings: DslBackupSettings) -> Result<(), String> {
    state.db.save_dsl_backup_settings(&settings)
}

#[tauri::command]
async fn backup_all_dsl(state: State<'_, AppState>, include_secret: Option<bool>) -> Result<Vec<DslBackupResult>, String> {
    let config = state.db.get_config()?.ok_or("请先配置连接信息")?;
    let apps = state.db.get_apps()?;
    if apps.is_empty() {
        return Err("没有已同步的应用，请先在应用管理中同步应用列表".to_string());
    }

    let backup_settings = state.db.get_dsl_backup_settings()?;
    let backup_dir = if backup_settings.backup_dir.is_empty() {
        return Err("请先配置备份目录".to_string());
    } else {
        backup_settings.backup_dir.clone()
    };

    let secret = include_secret.unwrap_or(backup_settings.include_secret);

    // Create timestamped subdirectory
    let now = chrono::Local::now();
    let subdir = now.format("%Y-%m-%d_%H%M%S").to_string();
    let backup_path = std::path::PathBuf::from(&backup_dir).join(&subdir);
    std::fs::create_dir_all(&backup_path)
        .map_err(|e| format!("创建备份目录失败: {}", e))?;

    let mut client = DifyApiClient::new(&config.api_base, &config.api_key, config.proxy.as_deref())?;

    let mut results: Vec<DslBackupResult> = Vec::new();

    for app in &apps {
        // Try to fetch DSL, auto-refresh on auth error
        let dsl_content = match client.fetch_app_dsl(&app.id, secret).await {
            Ok(content) => content,
            Err(ref e) if DifyApiClient::is_auth_error(e) => {
                let refreshed_config = try_auto_refresh(&state.db).await?;
                client = DifyApiClient::new(
                    &refreshed_config.api_base,
                    &refreshed_config.api_key,
                    refreshed_config.proxy.as_deref(),
                )?;
                match client.fetch_app_dsl(&app.id, secret).await {
                    Ok(content) => content,
                    Err(e) => {
                        results.push(DslBackupResult {
                            app_id: app.id.clone(),
                            app_name: app.name.clone(),
                            success: false,
                            file_path: None,
                            error: Some(e),
                        });
                        continue;
                    }
                }
            }
            Err(e) => {
                results.push(DslBackupResult {
                    app_id: app.id.clone(),
                    app_name: app.name.clone(),
                    success: false,
                    file_path: None,
                    error: Some(e),
                });
                continue;
            }
        };

        // Sanitize app name for filename, fallback to app ID if name is empty/all-special-chars
        // Include app ID suffix to avoid filename collisions when multiple apps share the same name
        let safe_name: String = {
            let sanitized: String = app.name
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                .collect();
            let trimmed = sanitized.trim_matches('_');
            let short_id: String = app.id.chars().take(8).collect();
            let name_with_id = if trimmed.is_empty() {
                app.id.clone()
            } else {
                format!("{}_{}", trimmed, short_id)
            };
            // Truncate to avoid exceeding filesystem limits (255 bytes).
            // Reserve space for ".yml" (4 bytes), use max 200 bytes for the name.
            let max_bytes = 200;
            let mut byte_count = 0;
            let truncated: String = name_with_id
                .chars()
                .take_while(|c| {
                    byte_count += c.len_utf8();
                    byte_count <= max_bytes
                })
                .collect();
            truncated
        };
        let filename = format!("{}.yml", safe_name);
        let file_path = backup_path.join(&filename);

        match std::fs::write(&file_path, &dsl_content) {
            Ok(_) => {
                results.push(DslBackupResult {
                    app_id: app.id.clone(),
                    app_name: app.name.clone(),
                    success: true,
                    file_path: Some(file_path.to_string_lossy().to_string()),
                    error: None,
                });
            }
            Err(e) => {
                results.push(DslBackupResult {
                    app_id: app.id.clone(),
                    app_name: app.name.clone(),
                    success: false,
                    file_path: None,
                    error: Some(format!("写入文件失败: {}", e)),
                });
            }
        }
    }

    // Only update last backup timestamp if at least one backup succeeded
    if results.iter().any(|r| r.success) {
        let now_ts = chrono::Utc::now().timestamp();
        let _ = state.db.update_dsl_backup_last_backup(now_ts);
    }

    Ok(results)
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
            rebuild_dashboard_stats,
            get_aggregation_status,
            get_performance_stats,
            export_data,
            get_feedback_messages,
            export_feedback_data,
            get_app_node_types,
            export_node_eval_data,
            get_sync_config,
            save_sync_config,
            delete_app_sync_data,
            delete_app_workflow_details,
            get_app_sync_data_info,
            get_auto_sync_settings,
            save_auto_sync_settings,
            sync_all_apps,
            export_dashboard_excel,
            export_performance_excel,
            get_db_size_info,
            cleanup_raw_json,
            vacuum_database,
            get_dsl_backup_settings,
            save_dsl_backup_settings,
            backup_all_dsl,
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
