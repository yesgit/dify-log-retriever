use std::time::Duration;

use reqwest::{Client, Proxy, RequestBuilder};
use serde::de::DeserializeOwned;

use crate::models::*;

#[derive(Clone)]
pub struct DifyApiClient {
    client: Client,
    api_base: String,
    api_key: String,
}

impl DifyApiClient {
    pub fn new(api_base: &str, api_key: &str, proxy: Option<&str>) -> Result<Self, String> {
        let base = api_base.trim_end_matches('/').to_string();
        let mut builder = Client::builder().timeout(Duration::from_secs(30));

        if let Some(proxy_url) = proxy {
            let trimmed = proxy_url.trim();
            if !trimmed.is_empty() {
                let p = Proxy::all(trimmed)
                    .map_err(|e| format!("代理配置无效 '{}': {}", trimmed, e))?;
                builder = builder.proxy(p);
            }
        }

        let client = builder
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
        Ok(Self {
            client,
            api_base: base,
            api_key: api_key.to_string(),
        })
    }

    fn console_url(&self, path: &str) -> String {
        format!("{}/console/api{}", self.api_base, path)
    }

    // ===== Login to get access token =====
    pub async fn login(api_base: &str, email: &str, password: &str, proxy: Option<&str>) -> Result<LoginResponse, String> {
        let base = api_base.trim_end_matches('/').to_string();
        let mut builder = Client::builder().timeout(Duration::from_secs(30));

        if let Some(proxy_url) = proxy {
            let trimmed = proxy_url.trim();
            if !trimmed.is_empty() {
                let p = Proxy::all(trimmed)
                    .map_err(|e| format!("代理配置无效 '{}': {}", trimmed, e))?;
                builder = builder.proxy(p);
            }
        }

        let client = builder
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let login_body = serde_json::json!({
            "email": email,
            "password": password,
            "language": "zh-Hans",
        });

        let resp = client
            .post(format!("{}/console/api/login", base))
            .header("Content-Type", "application/json")
            .json(&login_body)
            .send()
            .await
            .map_err(|e| format!("登录请求失败: {}", e))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            // Try to extract error message from response
            if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&body) {
                let msg = err_json.get("message")
                    .or(err_json.get("msg"))
                    .or(err_json.get("error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("未知错误");
                return Err(format!("登录失败 ({}): {}", status, msg));
            }
            return Err(format!("登录失败 ({}): {}", status, body));
        }

        // Parse the login response - Dify returns { data: { access_token, refresh_token } }
        let login_resp: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("解析登录响应失败: {}", e))?;

        // Try { data: { access_token } } first, then fall back to { access_token }
        let access_token = login_resp
            .get("data")
            .and_then(|d| d.get("access_token"))
            .or_else(|| login_resp.get("access_token"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let refresh_token = login_resp
            .get("data")
            .and_then(|d| d.get("refresh_token"))
            .or_else(|| login_resp.get("refresh_token"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if access_token.is_empty() {
            return Err("登录成功但未获取到 access_token".to_string());
        }

        Ok(LoginResponse {
            access_token,
            refresh_token,
        })
    }

    // ===== Check if error is an auth error (401) =====
    pub fn is_auth_error(err: &str) -> bool {
        err.contains("(401)") || err.contains("Unauthorized") || err.contains("401 Unauthorized")
    }

    fn authed_get(&self, path: &str) -> RequestBuilder {
        self.client
            .get(self.console_url(path))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
    }

    async fn send_json<T: DeserializeOwned>(&self, req: RequestBuilder, error_prefix: &str) -> Result<T, String> {
        let resp = req
            .send()
            .await
            .map_err(|e| format!("{}: {}", error_prefix, e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("{} ({}): {}", error_prefix, status, body));
        }

        resp.json().await.map_err(|e| format!("解析响应失败: {}", e))
    }

    async fn send_value(&self, req: RequestBuilder, error_prefix: &str) -> Result<serde_json::Value, String> {
        self.send_json(req, error_prefix).await
    }

    // ===== Test Connection (fetch apps, first page only) =====
    pub async fn fetch_apps(&self) -> Result<Vec<DifyAppItem>, String> {
        let result: DifyAppsResponse = self
            .send_json(self.authed_get("/apps"), "请求失败")
            .await?;
        Ok(result.data)
    }

    // ===== Fetch All Apps (with pagination) =====
    pub async fn fetch_all_apps(&self) -> Result<Vec<DifyAppItem>, String> {
        let mut all_apps: Vec<DifyAppItem> = Vec::new();
        let mut page: i64 = 1;
        let limit: i64 = 100;

        loop {
            let result: DifyAppsResponse = self
                .send_json(
                    self.authed_get("/apps").query(&[
                        ("limit", limit.to_string()),
                        ("page", page.to_string()),
                    ]),
                    "获取应用列表失败",
                )
                .await?;

            let fetched_count = result.data.len();
            all_apps.extend(result.data);

            if fetched_count < limit as usize {
                break;
            }
            page += 1;
        }

        Ok(all_apps)
    }

    // ===== Fetch Conversations for an App =====
    pub async fn fetch_conversations(
        &self,
        app_id: &str,
        limit: i64,
        page: i64,
    ) -> Result<DifyConversationsResponse, String> {
        let value = self
            .send_value(
                self.authed_get(&format!("/apps/{}/chat-conversations", app_id))
                    .query(&[
                        ("limit", limit.to_string()),
                        ("page", page.to_string()),
                        ("sort_by", "-created_at".to_string()),
                        ("annotation_status", "all".to_string()),
                    ]),
                "获取对话列表失败",
            )
            .await?;
        conversation_response_from_value(value)
    }

    pub async fn fetch_conversation_detail(
        &self,
        app_id: &str,
        conversation_id: &str,
    ) -> Result<DifyConversationItem, String> {
        let value = self
            .send_value(
                self.authed_get(&format!(
                    "/apps/{}/chat-conversations/{}",
                    app_id, conversation_id
                )),
                "获取对话详情失败",
            )
            .await?;
        let mut item: DifyConversationItem = serde_json::from_value(value.clone())
            .map_err(|e| format!("解析对话详情失败: {}", e))?;
        item.raw_json = value;
        Ok(item)
    }

    // ===== Fetch Messages for an App (with pagination) =====
    pub async fn fetch_messages(
        &self,
        app_id: &str,
        conversation_id: &str,
        limit: i64,
    ) -> Result<Vec<DifyMessageItem>, String> {
        let mut all_messages: Vec<DifyMessageItem> = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let mut req = self
                .authed_get(&format!("/apps/{}/chat-messages", app_id))
                .query(&[
                    ("conversation_id", conversation_id),
                    ("limit", &limit.to_string()),
                ]);

            if let Some(ref c) = cursor {
                req = req.query(&[("cursor", c)]);
            }

            let value = self.send_value(req, "获取消息列表失败").await?;
            let result = messages_response_from_value(value)?;
            all_messages.extend(result.data);

            if result.has_more {
                if let Some(last) = all_messages.last() {
                    cursor = Some(last.id.clone());
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(all_messages)
    }

    pub async fn fetch_workflow_run(&self, app_id: &str, run_id: &str) -> Result<DifyWorkflowRun, String> {
        let value = self
            .send_value(
                self.authed_get(&format!("/apps/{}/workflow-runs/{}", app_id, run_id)),
                "获取 workflow run 失败",
            )
            .await?;
        let mut run: DifyWorkflowRun = serde_json::from_value(value.clone())
            .map_err(|e| format!("解析 workflow run 失败: {}", e))?;
        run.raw_json = value;
        Ok(run)
    }

    // ===== Fetch Workflow App Logs (for workflow-type apps) =====
    pub async fn fetch_workflow_app_logs(
        &self,
        app_id: &str,
        page: i64,
        limit: i64,
    ) -> Result<DifyWorkflowAppLogsResponse, String> {
        let result: DifyWorkflowAppLogsResponse = self
            .send_json(
                self.authed_get(&format!("/apps/{}/workflow-app-logs", app_id))
                    .query(&[
                        ("page", page.to_string()),
                        ("limit", limit.to_string()),
                    ]),
                "获取 workflow 应用日志失败",
            )
            .await?;
        Ok(result)
    }

    // ===== Export App DSL =====
    pub async fn fetch_app_dsl(&self, app_id: &str, include_secret: bool) -> Result<String, String> {
        let secret_param = if include_secret { "true" } else { "false" };
        let value = self
            .send_value(
                self.authed_get(&format!("/apps/{}/export", app_id))
                    .query(&[("include_secret", secret_param)]),
                "导出应用 DSL 失败",
            )
            .await?;

        // Response format: { "data": "yaml_string" }
        let dsl_content = value
            .get("data")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if dsl_content.is_empty() {
            return Err("导出 DSL 返回内容为空".to_string());
        }

        Ok(dsl_content)
    }

    pub async fn fetch_node_executions(
        &self,
        app_id: &str,
        run_id: &str,
    ) -> Result<Vec<DifyNodeExecution>, String> {
        let value = self
            .send_value(
                self.authed_get(&format!(
                    "/apps/{}/workflow-runs/{}/node-executions",
                    app_id, run_id
                )),
                "获取 node executions 失败",
            )
            .await?;
        let response = node_executions_response_from_value(value)?;
        Ok(response.data)
    }
}

fn conversation_response_from_value(value: serde_json::Value) -> Result<DifyConversationsResponse, String> {
    let has_more = value.get("has_more").and_then(|v| v.as_bool()).unwrap_or(false);
    let data = value
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|item_value| {
            let mut item: DifyConversationItem = serde_json::from_value(item_value.clone())
                .map_err(|e| format!("解析对话列表失败: {}", e))?;
            item.raw_json = item_value;
            Ok(item)
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DifyConversationsResponse { data, has_more })
}

fn messages_response_from_value(value: serde_json::Value) -> Result<DifyMessagesResponse, String> {
    let has_more = value.get("has_more").and_then(|v| v.as_bool()).unwrap_or(false);
    let data = value
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|item_value| {
            let mut item: DifyMessageItem = serde_json::from_value(item_value.clone())
                .map_err(|e| format!("解析消息列表失败: {}", e))?;
            item.raw_json = item_value;
            Ok(item)
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DifyMessagesResponse { data, has_more })
}

fn node_executions_response_from_value(value: serde_json::Value) -> Result<DifyNodeExecutionsResponse, String> {
    let data = value
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|item_value| {
            let mut item: DifyNodeExecution = serde_json::from_value(item_value.clone())
                .map_err(|e| format!("解析 node execution 失败: {}", e))?;
            item.raw_json = item_value;
            Ok(item)
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DifyNodeExecutionsResponse { data })
}
