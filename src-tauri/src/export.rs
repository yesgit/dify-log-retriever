use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::models::MessageDetail;

pub fn export_to_json(messages: &[MessageDetail], include_metadata: bool, include_agent_thoughts: bool) -> Result<String, String> {
    let data: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| message_to_json(m, include_metadata, include_agent_thoughts))
        .collect();
    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
}

pub fn export_to_csv(messages: &[MessageDetail]) -> Result<String, String> {
    let mut lines: Vec<String> = vec![
        "\"id\",\"message_id\",\"conversation_id\",\"query\",\"answer\",\"feedback\",\"answer_tokens\",\"prompt_tokens\",\"elapsed_time\",\"created_at\"".to_string()
    ];

    for m in messages {
        let query = escape_csv(&m.query);
        let answer = escape_csv(&m.answer);
        let feedback = m.feedback.as_deref().unwrap_or("");
        lines.push(format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
            m.id, m.message_id, m.conversation_id, query, answer, feedback,
            m.answer_tokens, m.prompt_tokens, m.elapsed_time, m.created_at
        ));
    }

    Ok(lines.join("\n"))
}

fn escape_csv(s: &str) -> String {
    s.replace('"', "\"\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

pub fn export_to_jsonl(messages: &[MessageDetail], include_metadata: bool, include_agent_thoughts: bool) -> Result<String, String> {
    let lines: Vec<String> = messages
        .iter()
        .map(|m| {
            let obj = message_to_json(m, include_metadata, include_agent_thoughts);
            serde_json::to_string(&obj).unwrap_or_default()
        })
        .collect();
    Ok(lines.join("\n"))
}

fn message_to_json(m: &MessageDetail, include_metadata: bool, include_agent_thoughts: bool) -> serde_json::Value {
    let mut obj = json!({
        "id": m.id,
        "message_id": m.message_id,
        "conversation_id": m.conversation_id,
        "query": m.query,
        "answer": m.answer,
        "feedback": m.feedback,
        "created_at": m.created_at,
    });

    if include_metadata {
        obj["answer_tokens"] = json!(m.answer_tokens);
        obj["prompt_tokens"] = json!(m.prompt_tokens);
        obj["elapsed_time"] = json!(m.elapsed_time);
        obj["metadata"] = m.message_metadata.clone();
        obj["retriever_resources"] = m.retriever_resources.clone();
    }

    if include_agent_thoughts {
        obj["agent_thoughts"] = m.agent_thoughts.clone();
    }

    obj
}

pub fn save_export_file(content: &str, format: &str) -> Result<String, String> {
    let downloads_dir = dirs_download_dir()?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("dify_export_{}.{}", timestamp, format);
    let filepath = downloads_dir.join(&filename);

    fs::write(&filepath, content).map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(format!("已导出到: {}", filepath.display()))
}

fn dirs_download_dir() -> Result<PathBuf, String> {
    // Cross-platform download directory
    // Linux: ~/Downloads, macOS: ~/Downloads, Windows: C:\Users\<User>\Downloads
    if let Some(download_dir) = dirs::download_dir() {
        if download_dir.exists() {
            return Ok(download_dir);
        }
    }
    // Fallback to current dir
    Ok(PathBuf::from("."))
}
