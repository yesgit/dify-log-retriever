use std::fs;
use std::path::PathBuf;

use rust_xlsxwriter::*;
use serde_json::json;

use crate::models::{FeedbackMessage, MessageDetail, NodeEvalRecord};

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

/// Save export file using a native save dialog, with fallback to Downloads directory.
pub fn save_export_file_with_dialog(content: &str, default_filename: &str, _ext: &str) -> Result<String, String> {
    // Try to save using native dialog first
    // Since dialog is async and requires AppHandle, we fall back to auto-save.
    // The dialog approach will be handled at the command level in lib.rs for async context.
    let save_path = pick_save_path(default_filename)?;
    fs::write(&save_path, content).map_err(|e| format!("写入文件失败: {}", e))?;
    Ok(format!("已导出到: {}", save_path.display()))
}

fn pick_save_path(default_filename: &str) -> Result<PathBuf, String> {
    // Use Downloads directory as default save location
    let downloads_dir = dirs_download_dir()?;
    Ok(downloads_dir.join(default_filename))
}

fn dirs_download_dir() -> Result<PathBuf, String> {
    // Cross-platform download directory
    if let Some(download_dir) = dirs::download_dir() {
        if download_dir.exists() {
            return Ok(download_dir);
        }
    }
    // Fallback to home directory
    if let Some(home_dir) = dirs::home_dir() {
        return Ok(home_dir);
    }
    // Last resort: current directory
    Ok(PathBuf::from("."))
}

// ===== Feedback Export Functions =====

fn format_timestamp(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

fn extract_feedback_content(feedbacks: &serde_json::Value) -> String {
    feedbacks
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|f| {
                    let label = f.get("label").and_then(|v| v.as_str()).unwrap_or("");
                    let content = f
                        .get("content")
                        .or_else(|| f.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let parts: Vec<&str> = match (label, content) {
                        ("", "") => return None,
                        ("", c) => vec![c],
                        (l, "") => vec![l],
                        (l, c) => vec![l, c],
                    };
                    Some(parts.join(": "))
                })
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default()
}

pub fn export_feedback_to_excel(messages: &[FeedbackMessage], save_path: Option<&std::path::Path>) -> Result<String, String> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet().set_name("用户反馈").map_err(|e| e.to_string())?;

    // Header format
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_text_wrap();

    // Like format (green)
    let like_format = Format::new()
        .set_font_color(Color::RGB(0x008000))
        .set_border(FormatBorder::Thin);

    // Dislike format (red)
    let dislike_format = Format::new()
        .set_font_color(Color::RGB(0xFF0000))
        .set_border(FormatBorder::Thin);

    // Normal format
    let normal_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_text_wrap()
        .set_align(FormatAlign::Top);

    // Headers
    let headers = [
        ("应用名称", 18.0),
        ("反馈类型", 10.0),
        ("用户提问", 40.0),
        ("AI 回答", 40.0),
        ("反馈内容", 25.0),
        ("Prompt Tokens", 13.0),
        ("Answer Tokens", 13.0),
        ("耗时(秒)", 10.0),
        ("创建时间", 20.0),
    ];

    for (col, (header, width)) in headers.iter().enumerate() {
        worksheet.set_column_width(col as u16, *width).map_err(|e| e.to_string())?;
        worksheet.write_string_with_format(0, col as u16, *header, &header_format)
            .map_err(|e| e.to_string())?;
    }

    // Data rows
    for (row_idx, msg) in messages.iter().enumerate() {
        let row = (row_idx + 1) as u32;
        let feedback_str = match msg.feedback.as_deref() {
            Some("like") => "👍 赞",
            Some("dislike") => "👎 踩",
            _ => msg.feedback.as_deref().unwrap_or("-"),
        };

        worksheet.write_string_with_format(row, 0, &msg.app_name, &normal_format)
            .map_err(|e| e.to_string())?;

        let fb_format = match msg.feedback.as_deref() {
            Some("like") => &like_format,
            Some("dislike") => &dislike_format,
            _ => &normal_format,
        };
        worksheet.write_string_with_format(row, 1, feedback_str, fb_format)
            .map_err(|e| e.to_string())?;

        worksheet.write_string_with_format(row, 2, &msg.query, &normal_format)
            .map_err(|e| e.to_string())?;
        worksheet.write_string_with_format(row, 3, &msg.answer, &normal_format)
            .map_err(|e| e.to_string())?;

        let content = extract_feedback_content(&msg.feedbacks);
        worksheet.write_string_with_format(row, 4, &content, &normal_format)
            .map_err(|e| e.to_string())?;

        worksheet.write_number_with_format(row, 5, msg.prompt_tokens as f64, &normal_format)
            .map_err(|e| e.to_string())?;
        worksheet.write_number_with_format(row, 6, msg.answer_tokens as f64, &normal_format)
            .map_err(|e| e.to_string())?;
        worksheet.write_number_with_format(row, 7, msg.elapsed_time, &normal_format)
            .map_err(|e| e.to_string())?;

        let time_str = format_timestamp(msg.created_at);
        worksheet.write_string_with_format(row, 8, &time_str, &normal_format)
            .map_err(|e| e.to_string())?;
    }

    // Auto-filter
    if !messages.is_empty() {
        let last_col = 8u16;
        let last_row = messages.len() as u32;
        worksheet.autofilter(0, 0, last_row, last_col).map_err(|e| e.to_string())?;
    }

    // Freeze header row
    worksheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;

    let default_filename = format!("feedback_export_{}.xlsx", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let path = save_path.map(|p| p.to_path_buf()).unwrap_or_else(|| pick_save_path(&default_filename).unwrap_or_else(|_| std::path::PathBuf::from(&default_filename)));
    workbook.save(&path).map_err(|e| format!("保存 Excel 失败: {}", e))?;

    Ok(format!("已导出到: {}", path.display()))
}

pub fn export_feedback_to_csv(messages: &[FeedbackMessage], save_path: Option<&std::path::Path>) -> Result<String, String> {
    let mut lines: Vec<String> = vec![
        "\"应用名称\",\"反馈类型\",\"用户提问\",\"AI回答\",\"反馈内容\",\"Prompt Tokens\",\"Answer Tokens\",\"耗时(秒)\",\"创建时间\"".to_string()
    ];

    for m in messages {
        let feedback_str = match m.feedback.as_deref() {
            Some("like") => "赞",
            Some("dislike") => "踩",
            _ => m.feedback.as_deref().unwrap_or(""),
        };
        let content = extract_feedback_content(&m.feedbacks);
        let time_str = format_timestamp(m.created_at);
        lines.push(format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
            escape_csv(&m.app_name),
            feedback_str,
            escape_csv(&m.query),
            escape_csv(&m.answer),
            escape_csv(&content),
            m.prompt_tokens,
            m.answer_tokens,
            m.elapsed_time,
            time_str,
        ));
    }

    let default_filename = format!("feedback_export_{}.csv", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let path = save_path.map(|p| p.to_path_buf()).unwrap_or_else(|| pick_save_path(&default_filename).unwrap_or_else(|_| std::path::PathBuf::from(&default_filename)));
    fs::write(&path, lines.join("\n")).map_err(|e| format!("写入文件失败: {}", e))?;
    Ok(format!("已导出到: {}", path.display()))
}

// ===== Node Eval Export Functions =====

/// Extract prompt messages from LLM/Agent node inputs
/// Dify LLM node inputs typically have a "prompt" array with role/text pairs
/// Agent nodes may have "instruction" field and "query" from the user
fn extract_prompt_messages(inputs: &serde_json::Value) -> Vec<(String, String)> {
    let mut messages = Vec::new();

    // Try inputs.prompt (array of {role, text} objects) — standard LLM node format
    if let Some(prompt_arr) = inputs.get("prompt").and_then(|p| p.as_array()) {
        for item in prompt_arr {
            let role = item.get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("user")
                .to_string();
            let text = item.get("text")
                .or_else(|| item.get("content"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !text.is_empty() {
                messages.push((role, text));
            }
        }
    }

    // Agent node: try system_prompt/system as system prompt, fallback to instruction
    if messages.is_empty() {
        let sys_text = inputs.get("system_prompt")
            .or_else(|| inputs.get("system"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| {
                inputs.get("instruction")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
            });
        if let Some(sys) = sys_text {
            messages.push(("system".to_string(), sys));
        }
    }

    // If still no messages, try other common structures
    if messages.is_empty() {
        if let Some(query) = inputs.get("query").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            messages.push(("user".to_string(), query.to_string()));
        } else if let Some(input) = inputs.get("input").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            messages.push(("user".to_string(), input.to_string()));
        }
    }

    // Add user query ONLY if no user message exists yet (prevents duplicates for LLM nodes
    // where the prompt array already contains a rendered user message with template variables)
    let has_any_user = messages.iter().any(|(role, _)| role == "user");
    if !has_any_user {
        if let Some(query) = inputs.get("query").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            let insert_pos = messages.iter()
                .position(|(role, _)| role == "assistant")
                .unwrap_or(messages.len());
            messages.insert(insert_pos, ("user".to_string(), query.to_string()));
        }
    }

    messages
}

/// Extract the output text from LLM/Agent node outputs
fn extract_output_text(outputs: &serde_json::Value) -> String {
    // Try common output field names
    let candidates = ["text", "output", "result", "answer", "content"];
    for key in &candidates {
        if let Some(val) = outputs.get(key) {
            if let Some(s) = val.as_str() {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
            // Handle nested object: {"output": {"text": "..."}}
            if val.is_object() {
                for inner_key in &candidates {
                    if let Some(inner) = val.get(inner_key).and_then(|v| v.as_str()) {
                        if !inner.is_empty() {
                            return inner.to_string();
                        }
                    }
                }
            }
        }
    }
    String::new()
}

/// Extract model name from node inputs
fn extract_model(inputs: &serde_json::Value) -> String {
    inputs.get("model")
        .or_else(|| inputs.get("model_provider"))
        .and_then(|v| {
            if v.is_string() {
                v.as_str()
            } else if v.is_object() {
                v.get("model").and_then(|m| m.as_str()).or_else(|| v.get("name").and_then(|n| n.as_str()))
            } else {
                None
            }
        })
        .unwrap_or("")
        .to_string()
}

/// Assemble a flat prompt string from structured messages
fn assemble_prompt_text(messages: &[(String, String)]) -> String {
    messages.iter().map(|(role, text)| {
        match role.as_str() {
            "system" => format!("## System\n{}", text),
            "user" => format!("## User\n{}", text),
            "assistant" => format!("## Assistant\n{}", text),
            _ => format!("## {}\n{}", role, text),
        }
    }).collect::<Vec<_>>().join("\n\n")
}

/// Build a context string from process_data (for knowledge-retrieval upstream nodes)
fn extract_context(process_data: &serde_json::Value) -> String {
    if let Some(arr) = process_data.as_array() {
        arr.iter()
            .filter_map(|item| {
                item.get("content")
                    .or_else(|| item.get("text"))
                    .or_else(|| item.get("segment"))
                    .and_then(|v| v.as_str())
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    } else if let Some(s) = process_data.as_str() {
        s.to_string()
    } else {
        String::new()
    }
}

pub fn export_node_eval(
    records: &[NodeEvalRecord],
    format: &str,
) -> Result<String, String> {
    if records.is_empty() {
        return Err("没有找到匹配的节点执行数据".to_string());
    }

    let lines: Vec<String> = records.iter().map(|rec| {
        let prompt_messages = extract_prompt_messages(&rec.inputs);
        let output_text = extract_output_text(&rec.outputs);
        let model = extract_model(&rec.inputs);
        let context = extract_context(&rec.process_data);
        let system_msg = prompt_messages.iter()
            .find(|(role, _)| role == "system")
            .map(|(_, text)| text.clone())
            .unwrap_or_default();
        let user_msg = prompt_messages.iter()
            .find(|(role, _)| role == "user")
            .map(|(_, text)| text.clone())
            .unwrap_or_else(|| rec.query.clone());
        let assembled_prompt = assemble_prompt_text(&prompt_messages);

        let line = match format {
            "openai-eval" => {
                // OpenAI Evals format: {"messages": [...], "ideal": "..."}
                let mut msgs = serde_json::json!([]);
                if let Some(arr) = msgs.as_array_mut() {
                    if !system_msg.is_empty() {
                        arr.push(json!({"role": "system", "content": system_msg}));
                    }
                    if !user_msg.is_empty() {
                        arr.push(json!({"role": "user", "content": user_msg}));
                    }
                }
                let mut obj = json!({
                    "messages": msgs,
                });
                if !output_text.is_empty() {
                    obj["ideal"] = json!(output_text);
                }
                serde_json::to_string(&obj).unwrap_or_default()
            }
            "openai-finetune" => {
                // OpenAI Fine-tuning format: {"messages": [...including assistant response...]}
                let mut msgs = serde_json::json!([]);
                if let Some(arr) = msgs.as_array_mut() {
                    if !system_msg.is_empty() {
                        arr.push(json!({"role": "system", "content": system_msg}));
                    }
                    if !user_msg.is_empty() {
                        arr.push(json!({"role": "user", "content": user_msg}));
                    }
                    if !output_text.is_empty() {
                        arr.push(json!({"role": "assistant", "content": output_text}));
                    }
                }
                serde_json::to_string(&json!({"messages": msgs})).unwrap_or_default()
            }
            "alpaca" => {
                // AlpacaEval / Instruction format
                let mut obj = json!({
                    "instruction": user_msg,
                });
                if !system_msg.is_empty() {
                    obj["system"] = json!(system_msg);
                }
                if !context.is_empty() {
                    obj["input"] = json!(context);
                }
                if !output_text.is_empty() {
                    obj["output"] = json!(output_text);
                }
                if !model.is_empty() {
                    obj["generator"] = json!(model);
                }
                serde_json::to_string(&obj).unwrap_or_default()
            }
            "qa" => {
                // Generic QA format
                let mut obj = json!({
                    "query": user_msg,
                });
                if !system_msg.is_empty() {
                    obj["system_prompt"] = json!(system_msg);
                }
                if !context.is_empty() {
                    obj["context"] = json!(context);
                }
                if !output_text.is_empty() {
                    obj["expected_output"] = json!(output_text);
                }
                if !rec.query.is_empty() && rec.query != user_msg {
                    obj["original_query"] = json!(rec.query);
                }
                serde_json::to_string(&obj).unwrap_or_default()
            }
            "raw" => {
                // Raw format with full details
                serde_json::to_string(&json!({
                    "execution_id": rec.execution_id,
                    "workflow_run_id": rec.workflow_run_id,
                    "node_id": rec.node_id,
                    "node_type": rec.node_type,
                    "node_title": rec.node_title,
                    "query": rec.query,
                    "prompt_messages": prompt_messages.iter().map(|(r, t)| json!({"role": r, "content": t})).collect::<Vec<_>>(),
                    "assembled_prompt": assembled_prompt,
                    "output": output_text,
                    "context": context,
                    "model": model,
                    "inputs": rec.inputs,
                    "outputs": rec.outputs,
                    "process_data": rec.process_data,
                    "status": rec.status,
                    "elapsed_time": rec.elapsed_time,
                    "created_at": rec.created_at,
                })).unwrap_or_default()
            }
            _ => {
                // Default: same as raw
                serde_json::to_string(&json!({
                    "query": rec.query,
                    "assembled_prompt": assembled_prompt,
                    "output": output_text,
                    "inputs": rec.inputs,
                    "outputs": rec.outputs,
                })).unwrap_or_default()
            }
        };
        line
    }).collect();

    Ok(lines.join("\n"))
}

pub fn export_node_eval_to_file(
    records: &[NodeEvalRecord],
    format: &str,
) -> Result<String, String> {
    let content = export_node_eval(records, format)?;

    let default_filename = format!(
        "node_eval_{}_{}_{}records.jsonl",
        format,
        chrono::Local::now().format("%Y%m%d_%H%M%S"),
        records.len()
    );

    save_export_file_with_dialog(&content, &default_filename, "jsonl")
}

pub fn export_feedback_to_json(messages: &[FeedbackMessage], save_path: Option<&std::path::Path>) -> Result<String, String> {
    let data: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            json!({
                "app_name": m.app_name,
                "feedback": m.feedback,
                "query": m.query,
                "answer": m.answer,
                "feedbacks": m.feedbacks,
                "prompt_tokens": m.prompt_tokens,
                "answer_tokens": m.answer_tokens,
                "elapsed_time": m.elapsed_time,
                "created_at": m.created_at,
                "created_at_str": format_timestamp(m.created_at),
            })
        })
        .collect();
    let content = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;

    let default_filename = format!("feedback_export_{}.json", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let path = save_path.map(|p| p.to_path_buf()).unwrap_or_else(|| pick_save_path(&default_filename).unwrap_or_else(|_| std::path::PathBuf::from(&default_filename)));
    fs::write(&path, content).map_err(|e| format!("写入文件失败: {}", e))?;
    Ok(format!("已导出到: {}", path.display()))
}
