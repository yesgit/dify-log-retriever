use std::time::Duration;

use reqwest::{Client, Proxy};

use crate::models::*;

pub struct DifyApiClient {
    client: Client,
    api_base: String,
    api_key: String,
}

impl DifyApiClient {
    pub fn new(api_base: &str, api_key: &str, proxy: Option<&str>) -> Result<Self, String> {
        let base = api_base.trim_end_matches('/').to_string();
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(30));

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

    // ===== Test Connection (fetch apps, first page only) =====
    pub async fn fetch_apps(&self) -> Result<Vec<DifyAppItem>, String> {
        let url = self.console_url("/apps");
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API 返回错误 ({}): {}", status, body));
        }

        let result: DifyAppsResponse = resp
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        Ok(result.data)
    }

    // ===== Fetch All Apps (with pagination) =====
    pub async fn fetch_all_apps(&self) -> Result<Vec<DifyAppItem>, String> {
        let mut all_apps: Vec<DifyAppItem> = Vec::new();
        let mut page: i64 = 1;
        let limit: i64 = 100;

        loop {
            let url = self.console_url("/apps");
            let resp = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .query(&[
                    ("limit", limit.to_string()),
                    ("page", page.to_string()),
                ])
                .send()
                .await
                .map_err(|e| format!("请求应用列表失败: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("获取应用列表失败 ({}): {}", status, body));
            }

            let result: DifyAppsResponse = resp
                .json()
                .await
                .map_err(|e| format!("解析应用列表失败: {}", e))?;

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
        let url = self.console_url(&format!("/apps/{}/chat-conversations", app_id));
        let req = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .query(&[
                ("limit", limit.to_string()),
                ("page", page.to_string()),
                ("sort_by", "-created_at".to_string()),
            ]);

        let resp = req
            .send()
            .await
            .map_err(|e| format!("请求对话列表失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("获取对话列表失败 ({}): {}", status, body));
        }

        let result: DifyConversationsResponse = resp
            .json()
            .await
            .map_err(|e| format!("解析对话列表失败: {}", e))?;

        Ok(result)
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
            let url = self.console_url(&format!("/apps/{}/chat-messages", app_id));
            let mut req = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .query(&[
                    ("conversation_id", conversation_id),
                    ("limit", &limit.to_string()),
                ]);

            if let Some(ref c) = cursor {
                req = req.query(&[("cursor", c)]);
            }

            let resp = req
                .send()
                .await
                .map_err(|e| format!("请求消息列表失败: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("获取消息列表失败 ({}): {}", status, body));
            }

            let result: DifyMessagesResponse = resp
                .json()
                .await
                .map_err(|e| format!("解析消息列表失败: {}", e))?;

            all_messages.extend(result.data);

            if result.has_more {
                // Use last message id as cursor
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

}