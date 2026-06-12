use std::fs;
use std::path::PathBuf;

use rust_xlsxwriter::*;
use serde_json::json;

use crate::models::{DashboardStats, ExportMessageRecord, FeedbackMessage, NodeEvalRecord, PerformanceStats, StatDistribution};

pub fn export_to_json(messages: &[ExportMessageRecord], include_metadata: bool, include_agent_thoughts: bool) -> Result<String, String> {
    let data: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| message_to_json(m, include_metadata, include_agent_thoughts))
        .collect();
    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
}

pub fn export_to_csv(messages: &[ExportMessageRecord]) -> Result<String, String> {
    let mut lines: Vec<String> = vec![
        "\"标题\",\"用户或账户\",\"状态\",\"消息数\",\"用户反馈\",\"管理员反馈\",\"更新时间\",\"创建时间\",\"消息时间\",\"message_id\",\"conversation_id\",\"query\",\"answer\",\"feedback\",\"answer_tokens\",\"prompt_tokens\",\"elapsed_time\"".to_string()
    ];

    for m in messages {
        let query = escape_csv(&m.query);
        let answer = escape_csv(&m.answer);
        let feedback = m.feedback.as_deref().unwrap_or("");
        let user_feedback = escape_csv(&json_text(&m.user_feedback));
        let admin_feedback = escape_csv(&json_text(&m.admin_feedback));
        let updated_at = format_timestamp(m.updated_at);
        let created_at = format_timestamp(m.conversation_created_at);
        let message_time = format_timestamp(m.created_at);
        lines.push(format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
            escape_csv(&m.title),
            escape_csv(&m.user_or_account),
            escape_csv(&m.status),
            m.message_count,
            user_feedback,
            admin_feedback,
            updated_at,
            created_at,
            message_time,
            m.message_id,
            m.conversation_id,
            query,
            answer,
            feedback,
            m.answer_tokens,
            m.prompt_tokens,
            m.elapsed_time
        ));
    }

    Ok(lines.join("\n"))
}

fn escape_csv(s: &str) -> String {
    s.replace('"', "\"\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

pub fn export_to_jsonl(messages: &[ExportMessageRecord], include_metadata: bool, include_agent_thoughts: bool) -> Result<String, String> {
    let lines: Vec<String> = messages
        .iter()
        .map(|m| {
            let obj = message_to_json(m, include_metadata, include_agent_thoughts);
            serde_json::to_string(&obj).unwrap_or_default()
        })
        .collect();
    Ok(lines.join("\n"))
}

fn message_to_json(m: &ExportMessageRecord, include_metadata: bool, include_agent_thoughts: bool) -> serde_json::Value {
    let created_at_human = format_timestamp(m.created_at);
    let updated_at_human = format_timestamp(m.updated_at);
    let conversation_created_at_human = format_timestamp(m.conversation_created_at);

    let mut obj = json!({
        "id": m.id,
        "title": m.title,
        "user_or_account": m.user_or_account,
        "status": m.status,
        "message_count": m.message_count,
        "user_feedback": m.user_feedback,
        "admin_feedback": m.admin_feedback,
        "updated_at": m.updated_at,
        "updated_at_human": updated_at_human,
        "conversation_updated_at_human": updated_at_human,
        "conversation_created_at": m.conversation_created_at,
        "conversation_created_at_human": conversation_created_at_human,
        "message_id": m.message_id,
        "conversation_id": m.conversation_id,
        "query": m.query,
        "answer": m.answer,
        "feedback": m.feedback,
        "created_at": m.created_at,
        "created_at_human": created_at_human,
        "message_created_at_human": created_at_human,
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

fn json_text(v: &serde_json::Value) -> String {
    if v.is_null() {
        return String::new();
    }
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    serde_json::to_string(v).unwrap_or_default()
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

/// Extract prompt messages from LLM/Agent node.
/// Priority: process_data.prompts (runtime actual prompts) > inputs (template config)
fn extract_prompt_messages(inputs: &serde_json::Value, process_data: &serde_json::Value) -> Vec<(String, String)> {
    let mut messages = Vec::new();

    // === Priority 1: process_data.prompts (runtime actual prompts with full conversation history) ===
    // Dify LLM/Agent nodes store the actual rendered prompts in process_data.prompts as an array
    // of {role, text, files} objects, which includes the full multi-turn conversation history.
    if let Some(prompt_arr) = process_data.get("prompts").and_then(|p| p.as_array()) {
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
        if !messages.is_empty() {
            return messages;
        }
    }

    // === Priority 2: inputs.prompt (template prompt config) ===
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

/// Extract model name from process_data (runtime) and inputs (config).
/// Priority: process_data.model_name > process_data.model_provider > inputs.model
fn extract_model(inputs: &serde_json::Value, process_data: &serde_json::Value) -> String {
    // Priority 1: process_data.model_name (runtime actual model name)
    if let Some(name) = process_data.get("model_name").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
        return name.to_string();
    }

    // Priority 2: process_data.model_provider (may contain provider/model info)
    if let Some(provider) = process_data.get("model_provider").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
        return provider.to_string();
    }

    // Priority 3: inputs.model or inputs.model_provider (config)
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
        let prompt_messages = extract_prompt_messages(&rec.inputs, &rec.process_data);
        let output_text = extract_output_text(&rec.outputs);
        let model = extract_model(&rec.inputs, &rec.process_data);
        let context = extract_context(&rec.process_data);
        let system_msg = prompt_messages.iter()
            .find(|(role, _)| role == "system")
            .map(|(_, text)| text.clone())
            .unwrap_or_default();
        // Use the LAST user message for single-turn formats (alpaca/qa),
        // as multi-turn conversations have multiple user messages and the
        // last one is the actual query for this node execution.
        let user_msg = prompt_messages.iter()
            .rev()
            .find(|(role, _)| role == "user")
            .map(|(_, text)| text.clone())
            .unwrap_or_else(|| rec.query.clone());
        let assembled_prompt = assemble_prompt_text(&prompt_messages);

        let line = match format {
            "openai-eval" => {
                // OpenAI Evals format: {"messages": [...full multi-turn conversation...], "ideal": "..."}
                // Include the complete multi-turn conversation history as context,
                // with the model's response as the "ideal" expected output.
                let msgs: Vec<serde_json::Value> = prompt_messages.iter()
                    .map(|(r, t)| json!({"role": r, "content": t}))
                    .collect();
                let mut obj = json!({
                    "messages": msgs,
                });
                if !output_text.is_empty() {
                    obj["ideal"] = json!(output_text);
                }
                if !model.is_empty() {
                    obj["model"] = json!(model);
                }
                serde_json::to_string(&obj).unwrap_or_default()
            }
            "openai-finetune" => {
                // OpenAI Fine-tuning format: {"messages": [...including full history + assistant response...]}
                // Include the complete multi-turn conversation history plus the model's response.
                let mut msgs: Vec<serde_json::Value> = prompt_messages.iter()
                    .map(|(r, t)| json!({"role": r, "content": t}))
                    .collect();
                if !output_text.is_empty() {
                    msgs.push(json!({"role": "assistant", "content": output_text}));
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

// ===== Dashboard Export Functions =====

pub fn export_dashboard_to_excel(stats: &DashboardStats, app_name: &str, save_path: Option<&std::path::Path>) -> Result<String, String> {
    let mut workbook = Workbook::new();

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_text_wrap();

    let label_format = Format::new()
        .set_bold()
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::RGB(0xF2F2F2));

    let value_format = Format::new()
        .set_border(FormatBorder::Thin);

    let percent_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_num_format("0.0%");

    let number_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0");

    // ===== Sheet 1: Overview =====
    let overview = workbook.add_worksheet().set_name("概览").map_err(|e| e.to_string())?;
    overview.set_column_width(0, 30.0).map_err(|e| e.to_string())?;
    overview.set_column_width(1, 20.0).map_err(|e| e.to_string())?;

    // Title
    overview.merge_range(0, 0, 0, 1, "数据看板概览", &header_format).map_err(|e| e.to_string())?;
    overview.write_string_with_format(1, 0, "筛选应用", &label_format).map_err(|e| e.to_string())?;
    overview.write_string_with_format(1, 1, if app_name.is_empty() { "全部应用" } else { app_name }, &value_format).map_err(|e| e.to_string())?;
    overview.write_string_with_format(2, 0, "导出时间", &label_format).map_err(|e| e.to_string())?;
    overview.write_string_with_format(2, 1, &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(), &value_format).map_err(|e| e.to_string())?;

    // Basic metrics
    let mut row = 4u32;
    let basic_metrics: [(&str, i64); 4] = [
        ("活跃用户数", stats.total_users),
        ("全部会话数", stats.total_conversations),
        ("全部消息数", stats.total_messages),
        ("应用数", stats.total_apps),
    ];
    overview.write_string_with_format(row, 0, "基本指标", &header_format).map_err(|e| e.to_string())?;
    overview.write_blank(row, 1, &header_format).map_err(|e| e.to_string())?;
    row += 1;
    for (label, value) in &basic_metrics {
        overview.write_string_with_format(row, 0, *label, &label_format).map_err(|e| e.to_string())?;
        overview.write_number_with_format(row, 1, *value as f64, &number_format).map_err(|e| e.to_string())?;
        row += 1;
    }

    // Key ratios
    row += 1;
    overview.write_string_with_format(row, 0, "关键指标", &header_format).map_err(|e| e.to_string())?;
    overview.write_blank(row, 1, &header_format).map_err(|e| e.to_string())?;
    row += 1;
    let ratio_metrics: [(&str, f64); 4] = [
        ("平均会话互动数", stats.avg_conversation_interactions),
        ("用户满意度 (‰)", stats.satisfaction_rate),
        ("好评率 (%)", stats.feedback_like_rate),
        ("异常率 (%)", stats.error_rate),
    ];
    for (label, value) in &ratio_metrics {
        overview.write_string_with_format(row, 0, *label, &label_format).map_err(|e| e.to_string())?;
        overview.write_number_with_format(row, 1, *value, &Format::new().set_border(FormatBorder::Thin).set_num_format("0.0")).map_err(|e| e.to_string())?;
        row += 1;
    }

    // Token stats
    row += 1;
    overview.write_string_with_format(row, 0, "Token 消耗", &header_format).map_err(|e| e.to_string())?;
    overview.write_blank(row, 1, &header_format).map_err(|e| e.to_string())?;
    row += 1;
    let token_metrics: [(&str, i64); 3] = [
        ("Prompt Tokens", stats.total_prompt_tokens),
        ("Answer Tokens", stats.total_answer_tokens),
        ("总 Token 量", stats.total_tokens),
    ];
    for (label, value) in &token_metrics {
        overview.write_string_with_format(row, 0, *label, &label_format).map_err(|e| e.to_string())?;
        overview.write_number_with_format(row, 1, *value as f64, &number_format).map_err(|e| e.to_string())?;
        row += 1;
    }
    overview.write_string_with_format(row, 0, "日均 Token 消耗", &label_format).map_err(|e| e.to_string())?;
    overview.write_number_with_format(row, 1, stats.daily_avg_tokens, &number_format).map_err(|e| e.to_string())?;

    // Feedback stats
    row += 2;
    overview.write_string_with_format(row, 0, "反馈统计", &header_format).map_err(|e| e.to_string())?;
    overview.write_blank(row, 1, &header_format).map_err(|e| e.to_string())?;
    row += 1;
    let feedback_metrics: [(&str, i64); 5] = [
        ("赞数", stats.feedback_like),
        ("踩数", stats.feedback_dislike),
        ("无反馈", stats.feedback_none),
        ("反馈总数", stats.feedback_total),
        ("有内容反馈数", stats.feedback_with_content),
    ];
    for (label, value) in &feedback_metrics {
        overview.write_string_with_format(row, 0, *label, &label_format).map_err(|e| e.to_string())?;
        overview.write_number_with_format(row, 1, *value as f64, &number_format).map_err(|e| e.to_string())?;
        row += 1;
    }

    // Error stats
    row += 1;
    overview.write_string_with_format(row, 0, "异常统计", &header_format).map_err(|e| e.to_string())?;
    overview.write_blank(row, 1, &header_format).map_err(|e| e.to_string())?;
    row += 1;
    overview.write_string_with_format(row, 0, "异常消息数", &label_format).map_err(|e| e.to_string())?;
    overview.write_number_with_format(row, 1, stats.error_count as f64, &number_format).map_err(|e| e.to_string())?;
    row += 1;
    overview.write_string_with_format(row, 0, "异常率", &label_format).map_err(|e| e.to_string())?;
    overview.write_number_with_format(row, 1, stats.error_rate / 100.0, &percent_format).map_err(|e| e.to_string())?;

    overview.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;

    // ===== Sheet 2: Distributions =====
    let dist_sheet = workbook.add_worksheet().set_name("分布统计").map_err(|e| e.to_string())?;
    for (col, width) in [(0, 22.0), (1, 12.0), (2, 12.0), (3, 12.0), (4, 12.0), (5, 12.0), (6, 12.0), (7, 12.0)] {
        dist_sheet.set_column_width(col, width).map_err(|e| e.to_string())?;
    }

    let dist_headers = ["指标名称", "样本数", "最小值", "最大值", "平均值", "P50", "P80", "P95"];
    for (col, h) in dist_headers.iter().enumerate() {
        dist_sheet.write_string_with_format(0, col as u16, *h, &header_format).map_err(|e| e.to_string())?;
    }

    let distributions: [(&str, Option<&StatDistribution>); 10] = [
        ("会话消息数分布", stats.messages_per_conversation_distribution.as_ref()),
        ("用户会话数分布", stats.conversations_per_user_distribution.as_ref()),
        ("用户消息数分布", stats.messages_per_user_distribution.as_ref()),
        ("首 Token 时间 (TTFT)", stats.ttft_distribution.as_ref()),
        ("总响应时间", stats.elapsed_time_distribution.as_ref()),
        ("每条消息 Token 消耗", stats.token_per_message_distribution.as_ref()),
        ("Token 生成速度", stats.token_speed_distribution.as_ref()),
        ("用户反馈数分布", stats.user_feedback_count_distribution.as_ref()),
        ("会话反馈数分布", stats.conversation_feedback_count_distribution.as_ref()),
        ("消息反馈数分布", stats.message_feedback_count_distribution.as_ref()),
    ];

    let mut dist_row = 1u32;
    for (name, dist_opt) in &distributions {
        if let Some(dist) = dist_opt {
            dist_sheet.write_string_with_format(dist_row, 0, *name, &label_format).map_err(|e| e.to_string())?;
            dist_sheet.write_number_with_format(dist_row, 1, dist.count as f64, &number_format).map_err(|e| e.to_string())?;
            dist_sheet.write_number_with_format(dist_row, 2, dist.min, &value_format).map_err(|e| e.to_string())?;
            dist_sheet.write_number_with_format(dist_row, 3, dist.max, &value_format).map_err(|e| e.to_string())?;
            dist_sheet.write_number_with_format(dist_row, 4, dist.avg, &Format::new().set_border(FormatBorder::Thin).set_num_format("0.00")).map_err(|e| e.to_string())?;
            dist_sheet.write_number_with_format(dist_row, 5, dist.p50, &Format::new().set_border(FormatBorder::Thin).set_num_format("0.00")).map_err(|e| e.to_string())?;
            dist_sheet.write_number_with_format(dist_row, 6, dist.p80, &Format::new().set_border(FormatBorder::Thin).set_num_format("0.00")).map_err(|e| e.to_string())?;
            dist_sheet.write_number_with_format(dist_row, 7, dist.p95, &Format::new().set_border(FormatBorder::Thin).set_num_format("0.00")).map_err(|e| e.to_string())?;
            dist_row += 1;
        }
    }
    dist_sheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;

    // ===== Sheet 3: Daily Trend =====
    let daily_sheet = workbook.add_worksheet().set_name("每日趋势").map_err(|e| e.to_string())?;
    let daily_headers = ["日期", "会话数", "消息数", "用户数", "输入 Token", "输出 Token", "异常数", "赞数", "踩数", "平均响应时间(s)", "平均 TTFT(s)", "Token 速度(t/s)"];
    for (col, h) in daily_headers.iter().enumerate() {
        daily_sheet.set_column_width(col as u16, 16.0).map_err(|e| e.to_string())?;
        daily_sheet.write_string_with_format(0, col as u16, *h, &header_format).map_err(|e| e.to_string())?;
    }

    for (idx, day) in stats.recent_daily.iter().enumerate() {
        let r = (idx + 1) as u32;
        daily_sheet.write_string_with_format(r, 0, &day.date, &value_format).map_err(|e| e.to_string())?;
        daily_sheet.write_number_with_format(r, 1, day.conversations as f64, &number_format).map_err(|e| e.to_string())?;
        daily_sheet.write_number_with_format(r, 2, day.messages as f64, &number_format).map_err(|e| e.to_string())?;
        daily_sheet.write_number_with_format(r, 3, day.users as f64, &number_format).map_err(|e| e.to_string())?;
        daily_sheet.write_number_with_format(r, 4, day.total_prompt_tokens as f64, &number_format).map_err(|e| e.to_string())?;
        daily_sheet.write_number_with_format(r, 5, day.total_answer_tokens as f64, &number_format).map_err(|e| e.to_string())?;
        daily_sheet.write_number_with_format(r, 6, day.errors as f64, &number_format).map_err(|e| e.to_string())?;
        daily_sheet.write_number_with_format(r, 7, day.likes as f64, &number_format).map_err(|e| e.to_string())?;
        daily_sheet.write_number_with_format(r, 8, day.dislikes as f64, &number_format).map_err(|e| e.to_string())?;
        daily_sheet.write_number_with_format(r, 9, day.avg_elapsed_time, &Format::new().set_border(FormatBorder::Thin).set_num_format("0.00")).map_err(|e| e.to_string())?;
        daily_sheet.write_number_with_format(r, 10, day.avg_ttft, &Format::new().set_border(FormatBorder::Thin).set_num_format("0.00")).map_err(|e| e.to_string())?;
        daily_sheet.write_number_with_format(r, 11, day.avg_token_speed, &Format::new().set_border(FormatBorder::Thin).set_num_format("0.00")).map_err(|e| e.to_string())?;
    }

    if !stats.recent_daily.is_empty() {
        let last_row = stats.recent_daily.len() as u32;
        let last_col = 11u16;
        daily_sheet.autofilter(0, 0, last_row, last_col).map_err(|e| e.to_string())?;
    }
    daily_sheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;

    // ===== Sheet 4: Top Apps =====
    if !stats.top_apps.is_empty() {
        let apps_sheet = workbook.add_worksheet().set_name("应用排名").map_err(|e| e.to_string())?;
        apps_sheet.set_column_width(0, 8.0).map_err(|e| e.to_string())?;
        apps_sheet.set_column_width(1, 30.0).map_err(|e| e.to_string())?;
        apps_sheet.set_column_width(2, 15.0).map_err(|e| e.to_string())?;
        apps_sheet.set_column_width(3, 15.0).map_err(|e| e.to_string())?;

        let apps_headers = ["排名", "应用名称", "会话数", "消息数"];
        for (col, h) in apps_headers.iter().enumerate() {
            apps_sheet.write_string_with_format(0, col as u16, *h, &header_format).map_err(|e| e.to_string())?;
        }

        for (idx, app) in stats.top_apps.iter().enumerate() {
            let r = (idx + 1) as u32;
            apps_sheet.write_number_with_format(r, 0, (idx + 1) as f64, &value_format).map_err(|e| e.to_string())?;
            apps_sheet.write_string_with_format(r, 1, &app.app_name, &value_format).map_err(|e| e.to_string())?;
            apps_sheet.write_number_with_format(r, 2, app.conversation_count as f64, &number_format).map_err(|e| e.to_string())?;
            apps_sheet.write_number_with_format(r, 3, app.message_count as f64, &number_format).map_err(|e| e.to_string())?;
        }
        apps_sheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;
    }

    // Save file
    let default_filename = format!("dashboard_export_{}.xlsx", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let path = save_path.map(|p| p.to_path_buf()).unwrap_or_else(|| pick_save_path(&default_filename).unwrap_or_else(|_| std::path::PathBuf::from(&default_filename)));
    workbook.save(&path).map_err(|e| format!("保存 Excel 失败: {}", e))?;

    Ok(format!("已导出到: {}", path.display()))
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

// ===== Performance Export Functions =====

pub fn export_performance_to_excel(stats: &PerformanceStats, app_name: &str, save_path: Option<&std::path::Path>) -> Result<String, String> {
    let mut workbook = Workbook::new();

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_text_wrap();

    let value_format = Format::new()
        .set_border(FormatBorder::Thin);

    let number_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0");

    let decimal_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_num_format("0.00");

    let percent_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_num_format("0.0%");

    // ===== Sheet 1: Model Performance =====
    let model_sheet = workbook.add_worksheet().set_name("模型性能统计").map_err(|e| e.to_string())?;
    let model_headers: [(&str, f64); 8] = [
        ("模型", 30.0), ("消息数", 12.0), ("总 Tokens", 14.0),
        ("平均耗时(s)", 14.0), ("平均 TTFT(s)", 14.0), ("速度(tokens/s)", 14.0),
        ("错误数", 10.0), ("错误率", 10.0),
    ];
    for (col, (_, w)) in model_headers.iter().enumerate() {
        model_sheet.set_column_width(col as u16, *w).map_err(|e| e.to_string())?;
    }
    // Title row (row 0)
    model_sheet.merge_range(0, 0, 0, 7, "性能分析报告", &header_format).map_err(|e| e.to_string())?;
    model_sheet.write_string_with_format(1, 0, "筛选应用", &Format::new().set_bold().set_border(FormatBorder::Thin).set_background_color(Color::RGB(0xF2F2F2))).map_err(|e| e.to_string())?;
    model_sheet.write_string_with_format(1, 1, if app_name.is_empty() { "全部应用" } else { app_name }, &value_format).map_err(|e| e.to_string())?;
    model_sheet.write_string_with_format(2, 0, "导出时间", &Format::new().set_bold().set_border(FormatBorder::Thin).set_background_color(Color::RGB(0xF2F2F2))).map_err(|e| e.to_string())?;
    model_sheet.write_string_with_format(2, 1, &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(), &value_format).map_err(|e| e.to_string())?;

    // Headers
    for (col, (h, _)) in model_headers.iter().enumerate() {
        model_sheet.write_string_with_format(3, col as u16, *h, &header_format).map_err(|e| e.to_string())?;
    }

    for (idx, m) in stats.model_performance.iter().enumerate() {
        let r = (idx + 4) as u32;
        model_sheet.write_string_with_format(r, 0, &m.model, &value_format).map_err(|e| e.to_string())?;
        model_sheet.write_number_with_format(r, 1, m.message_count as f64, &number_format).map_err(|e| e.to_string())?;
        model_sheet.write_number_with_format(r, 2, m.total_tokens as f64, &number_format).map_err(|e| e.to_string())?;
        model_sheet.write_number_with_format(r, 3, m.avg_elapsed_time, &decimal_format).map_err(|e| e.to_string())?;
        model_sheet.write_number_with_format(r, 4, m.avg_ttft, &decimal_format).map_err(|e| e.to_string())?;
        model_sheet.write_number_with_format(r, 5, m.avg_token_speed, &decimal_format).map_err(|e| e.to_string())?;
        model_sheet.write_number_with_format(r, 6, m.error_count as f64, &number_format).map_err(|e| e.to_string())?;
        model_sheet.write_number_with_format(r, 7, m.error_rate / 100.0, &percent_format).map_err(|e| e.to_string())?;
    }
    if !stats.model_performance.is_empty() {
        let last_row = (stats.model_performance.len() + 3) as u32;
        model_sheet.autofilter(3, 0, last_row, 7).map_err(|e| e.to_string())?;
    }
    model_sheet.set_freeze_panes(4, 0).map_err(|e| e.to_string())?;

    // ===== Sheet 2: Model Token Speed Trend =====
    if !stats.model_token_speed_daily.is_empty() {
        let trend_sheet = workbook.add_worksheet().set_name("模型Token速度趋势").map_err(|e| e.to_string())?;
        let trend_headers: [(&str, f64); 4] = [
            ("模型", 30.0), ("日期", 14.0), ("平均速度(tokens/s)", 16.0), ("消息数", 12.0),
        ];
        for (col, (h, w)) in trend_headers.iter().enumerate() {
            trend_sheet.set_column_width(col as u16, *w).map_err(|e| e.to_string())?;
            trend_sheet.write_string_with_format(0, col as u16, *h, &header_format).map_err(|e| e.to_string())?;
        }
        for (idx, d) in stats.model_token_speed_daily.iter().enumerate() {
            let r = (idx + 1) as u32;
            trend_sheet.write_string_with_format(r, 0, &d.model, &value_format).map_err(|e| e.to_string())?;
            trend_sheet.write_string_with_format(r, 1, &d.date, &value_format).map_err(|e| e.to_string())?;
            trend_sheet.write_number_with_format(r, 2, d.avg_token_speed, &decimal_format).map_err(|e| e.to_string())?;
            trend_sheet.write_number_with_format(r, 3, d.message_count as f64, &number_format).map_err(|e| e.to_string())?;
        }
        let last_row = stats.model_token_speed_daily.len() as u32;
        trend_sheet.autofilter(0, 0, last_row, 3).map_err(|e| e.to_string())?;
        trend_sheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;
    }

    // ===== Sheet 3: Node Performance =====
    if !stats.node_performance.is_empty() {
        let node_sheet = workbook.add_worksheet().set_name("节点性能统计").map_err(|e| e.to_string())?;
        let node_headers: [(&str, f64); 6] = [
            ("节点类型", 18.0), ("标题", 25.0), ("执行次数", 12.0),
            ("平均耗时(s)", 14.0), ("成功率", 10.0), ("错误数", 10.0),
        ];
        for (col, (h, w)) in node_headers.iter().enumerate() {
            node_sheet.set_column_width(col as u16, *w).map_err(|e| e.to_string())?;
            node_sheet.write_string_with_format(0, col as u16, *h, &header_format).map_err(|e| e.to_string())?;
        }
        for (idx, n) in stats.node_performance.iter().enumerate() {
            let r = (idx + 1) as u32;
            node_sheet.write_string_with_format(r, 0, &n.node_type, &value_format).map_err(|e| e.to_string())?;
            node_sheet.write_string_with_format(r, 1, &n.title, &value_format).map_err(|e| e.to_string())?;
            node_sheet.write_number_with_format(r, 2, n.execution_count as f64, &number_format).map_err(|e| e.to_string())?;
            node_sheet.write_number_with_format(r, 3, n.avg_elapsed_time, &decimal_format).map_err(|e| e.to_string())?;
            node_sheet.write_number_with_format(r, 4, n.success_rate / 100.0, &percent_format).map_err(|e| e.to_string())?;
            node_sheet.write_number_with_format(r, 5, n.error_count as f64, &number_format).map_err(|e| e.to_string())?;
        }
        let last_row = stats.node_performance.len() as u32;
        node_sheet.autofilter(0, 0, last_row, 5).map_err(|e| e.to_string())?;
        node_sheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;
    }

    // ===== Sheet 4: Node Daily Performance =====
    if !stats.node_daily_performance.is_empty() {
        let daily_sheet = workbook.add_worksheet().set_name("节点每日性能趋势").map_err(|e| e.to_string())?;
        let daily_headers: [(&str, f64); 7] = [
            ("节点类型", 18.0), ("标题", 25.0), ("日期", 14.0),
            ("执行次数", 12.0), ("平均耗时(s)", 14.0), ("成功数", 10.0), ("错误数", 10.0),
        ];
        for (col, (h, w)) in daily_headers.iter().enumerate() {
            daily_sheet.set_column_width(col as u16, *w).map_err(|e| e.to_string())?;
            daily_sheet.write_string_with_format(0, col as u16, *h, &header_format).map_err(|e| e.to_string())?;
        }
        for (idx, n) in stats.node_daily_performance.iter().enumerate() {
            let r = (idx + 1) as u32;
            daily_sheet.write_string_with_format(r, 0, &n.node_type, &value_format).map_err(|e| e.to_string())?;
            daily_sheet.write_string_with_format(r, 1, &n.title, &value_format).map_err(|e| e.to_string())?;
            daily_sheet.write_string_with_format(r, 2, &n.date, &value_format).map_err(|e| e.to_string())?;
            daily_sheet.write_number_with_format(r, 3, n.execution_count as f64, &number_format).map_err(|e| e.to_string())?;
            daily_sheet.write_number_with_format(r, 4, n.avg_elapsed_time, &decimal_format).map_err(|e| e.to_string())?;
            daily_sheet.write_number_with_format(r, 5, n.success_count as f64, &number_format).map_err(|e| e.to_string())?;
            daily_sheet.write_number_with_format(r, 6, n.error_count as f64, &number_format).map_err(|e| e.to_string())?;
        }
        let last_row = stats.node_daily_performance.len() as u32;
        daily_sheet.autofilter(0, 0, last_row, 6).map_err(|e| e.to_string())?;
        daily_sheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;
    }

    // Save file
    let default_filename = format!("performance_export_{}.xlsx", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let path = save_path.map(|p| p.to_path_buf()).unwrap_or_else(|| pick_save_path(&default_filename).unwrap_or_else(|_| std::path::PathBuf::from(&default_filename)));
    workbook.save(&path).map_err(|e| format!("保存 Excel 失败: {}", e))?;

    Ok(format!("已导出到: {}", path.display()))
}
