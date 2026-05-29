use std::time::Duration;

use reqwest::Client;

use crate::models::*;

pub struct DifyApiClient {
    client: Client,
    api_base: String,
    api_key: String,
}

impl DifyApiClient {
    pub fn new(api_base: &str, api_key: &str) -> Self {
        let base = api_base.trim_end_matches('/').to_string();
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            api_base: base,
            api_key: api_key.to_string(),
        }
    }

    fn console_url(&self, path: &str) -> String {
        format!("{}/console/api{}", self.api_base, path)
    }

    // ===== Test Connection (fetch apps) =====
    pub async fn fetch_apps(&self) -> Result<Vec<DifyAppItem>, String> {
        let url = self.console_url("/apps");
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
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

    // ===== Fetch Conversations for an App =====
    pub async fn fetch_conversations(
        &self,
        app_id: &str,
        limit: i64,
        cursor: Option<&str>,
    ) -> Result<DifyConversationsResponse, String> {
        let url = self.console_url(&format!("/apps/{}/conversations", app_id));
        let mut req = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&[("limit", limit.to_string())]);

        if let Some(c) = cursor {
            req = req.query(&[("cursor", c)]);
        }

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
            let url = self.console_url(&format!("/apps/{}/messages", app_id));
            let mut req = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
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