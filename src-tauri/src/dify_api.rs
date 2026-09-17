use std::time::Duration;

use reqwest::{Client, Proxy, RequestBuilder};
use serde::de::DeserializeOwned;

use crate::models::*;

#[derive(Clone)]
pub struct DifyApiClient {
    client: Client,
    /// Client for fetching large payloads (knowledge-base document files).
    /// Has no overall request timeout so a slow download of a big original
    /// file isn't killed by the 30s API timeout.
    file_client: Client,
    api_base: String,
    api_key: String,
}

impl DifyApiClient {
    pub fn new(api_base: &str, api_key: &str, proxy: Option<&str>) -> Result<Self, String> {
        let base = api_base.trim_end_matches('/').to_string();
        let client = Self::build_http_client(Some(Duration::from_secs(30)), proxy)?;
        let file_client = Self::build_http_client(None, proxy)?;
        Ok(Self {
            client,
            file_client,
            api_base: base,
            api_key: api_key.to_string(),
        })
    }

    fn build_http_client(timeout: Option<Duration>, proxy: Option<&str>) -> Result<Client, String> {
        let mut builder = Client::builder();
        if let Some(t) = timeout {
            builder = builder.timeout(t);
        }

        if let Some(proxy_url) = proxy {
            let trimmed = proxy_url.trim();
            if !trimmed.is_empty() {
                let p = Proxy::all(trimmed)
                    .map_err(|e| format!("代理配置无效 '{}': {}", trimmed, e))?;
                builder = builder.proxy(p);
            }
        }

        builder
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))
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
                        // Sort by updated_at desc so incremental sync can stop early
                        // once it reaches conversations unchanged since last sync.
                        // Sorting by created_at (the old value) made that early-stop
                        // unsound: an old conversation with new activity sits on a late
                        // page and got missed when an earlier page was all-unchanged.
                        ("sort_by", "-updated_at".to_string()),
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
        // Safety valve: cap pagination so a misbehaving cursor (server returning
        // has_more=true without advancing) can't loop forever and stall the sync.
        const MAX_MESSAGE_PAGES: usize = 500;

        for _ in 0..MAX_MESSAGE_PAGES {
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
            let prev_cursor = cursor.clone();
            all_messages.extend(result.data);

            if !result.has_more {
                break;
            }
            // Stop if there is no next cursor, or it didn't advance since the last
            // request (otherwise we'd re-fetch the same page forever).
            let new_cursor = all_messages.last().map(|m| m.id.clone());
            if new_cursor.is_none() || new_cursor == prev_cursor {
                break;
            }
            cursor = new_cursor;
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

    // ===== Knowledge Base: list datasets (all pages) =====
    pub async fn fetch_datasets(&self, keyword: Option<&str>) -> Result<Vec<DifyDatasetItem>, String> {
        let mut all: Vec<DifyDatasetItem> = Vec::new();
        let mut page: i64 = 1;
        let limit: i64 = 100;
        // Safety valve against a server that never flips has_more.
        const MAX_PAGES: i64 = 200;

        loop {
            let mut req = self.authed_get("/datasets").query(&[
                ("page", page.to_string()),
                ("limit", limit.to_string()),
            ]);
            if let Some(kw) = keyword.map(str::trim).filter(|s| !s.is_empty()) {
                req = req.query(&[("keyword", kw.to_string())]);
            }

            let result: DifyDatasetsResponse = self
                .send_json(req, "获取知识库列表失败")
                .await?;
            let fetched_count = result.data.len();
            all.extend(result.data);

            // Don't rely on has_more: some Dify builds omit it, so a short page
            // is the only universally reliable end-of-list signal.
            if fetched_count < limit as usize || page >= MAX_PAGES {
                break;
            }
            page += 1;
        }

        Ok(all)
    }

    // ===== Knowledge Base: list documents in a dataset (all pages) =====
    pub async fn fetch_dataset_documents(
        &self,
        dataset_id: &str,
        keyword: Option<&str>,
    ) -> Result<Vec<DifyDatasetDocumentItem>, String> {
        let mut all: Vec<DifyDatasetDocumentItem> = Vec::new();
        let mut page: i64 = 1;
        let limit: i64 = 100;
        const MAX_PAGES: i64 = 500;
        // Newer Dify exposes GET /datasets/{id}/documents; older builds only
        // had /datasets/{id}/document_list. Switch on the first 404.
        let mut path = format!("/datasets/{}/documents", dataset_id);

        loop {
            let mut req = self.authed_get(&path).query(&[
                ("page", page.to_string()),
                ("limit", limit.to_string()),
            ]);
            if let Some(kw) = keyword.map(str::trim).filter(|s| !s.is_empty()) {
                req = req.query(&[("keyword", kw.to_string())]);
            }

            let result: DifyDatasetDocumentsResponse = match self
                .send_json(req, "获取文档列表失败")
                .await
            {
                // 405 on old builds where the route only accepts POST.
                Err(e)
                    if page == 1
                        && (e.contains("(404") || e.contains("(405"))
                        && path.ends_with("/documents") =>
                {
                    path = format!("/datasets/{}/document_list", dataset_id);
                    continue;
                }
                other => other?,
            };

            let fetched_count = result.data.len();
            all.extend(result.data);

            if fetched_count < limit as usize || page >= MAX_PAGES {
                break;
            }
            page += 1;
        }

        Ok(all)
    }

    // ===== Knowledge Base: signed download URL for the original file =====
    pub async fn fetch_document_download_url(
        &self,
        dataset_id: &str,
        document_id: &str,
    ) -> Result<String, String> {
        let value = self
            .send_value(
                self.authed_get(&format!(
                    "/datasets/{}/documents/{}/download",
                    dataset_id, document_id
                )),
                "获取文档下载地址失败",
            )
            .await?;

        // Response: { "url": "<signed url>" }; a few versions used download_url.
        let url = value
            .get("url")
            .or_else(|| value.get("download_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if url.is_empty() {
            return Err("文档下载地址为空".to_string());
        }
        Ok(url)
    }

    /// Fetch raw bytes from a (signed, unauthenticated) storage URL.
    pub async fn fetch_url_bytes(&self, url: &str) -> Result<Vec<u8>, String> {
        let resp = self
            .file_client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("下载文件失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("下载文件失败 ({}): {}", status, body));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("读取文件内容失败: {}", e))?;
        Ok(bytes.to_vec())
    }

    // ===== Knowledge Base: rebuild document text by paging through segments =====
    pub async fn fetch_document_content(
        &self,
        dataset_id: &str,
        document_id: &str,
        with_markers: bool,
    ) -> Result<String, String> {
        // Segment rows do NOT contain the separator Dify used when chunking, so
        // chunk boundaries are only visible if we inject markers. Collect
        // (position, content, answer) and sort by position so the rebuild order
        // doesn't depend on the list API's paging order. In QA mode the answer
        // lives in its own field and would be dropped if we only kept content.
        let mut items: Vec<(i64, String, Option<String>)> = Vec::new();
        let mut page: i64 = 1;
        let limit: i64 = 100;
        const MAX_PAGES: i64 = 1000;

        loop {
            let value = self
                .send_value(
                    self.authed_get(&format!(
                        "/datasets/{}/documents/{}/segments",
                        dataset_id, document_id
                    ))
                    .query(&[
                        ("page", page.to_string()),
                        ("limit", limit.to_string()),
                    ]),
                    "获取文档分段失败",
                )
                .await?;

            let arr = value
                .get("data")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let fetched_count = arr.len();
            for (i, seg) in arr.iter().enumerate() {
                let content = seg
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if content.is_empty() {
                    continue;
                }
                let position = seg
                    .get("position")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(i as i64);
                let answer = seg
                    .get("answer")
                    .and_then(|v| v.as_str())
                    .filter(|a| !a.trim().is_empty())
                    .map(|a| a.to_string());
                items.push((position, content, answer));
            }

            let has_more = value.get("has_more").and_then(|v| v.as_bool()).unwrap_or(false);
            if !has_more || fetched_count == 0 || page >= MAX_PAGES {
                break;
            }
            page += 1;
        }

        items.sort_by_key(|(position, _, _)| *position);

        let mut out = String::new();
        for (idx, (_, content, answer)) in items.iter().enumerate() {
            if idx > 0 {
                if with_markers {
                    out.push_str(&format!("\n\n======== 分段 {} ========\n\n", idx + 1));
                } else {
                    out.push_str("\n\n");
                }
            }
            match answer {
                // QA 模式：按 Dify 的问答文本格式写为「问题 \t 答案」
                Some(ans) => out.push_str(&format!("{}\t{}", content, ans)),
                None => out.push_str(content),
            }
        }
        Ok(out)
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
