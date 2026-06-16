use std::collections::HashMap;

use serde_json::{json, Value};

use crate::models::NodeDslConfig;

/// Parse a Dify workflow DSL (YAML string) into a map of `node_id -> NodeDslConfig`.
///
/// Only fields relevant to reproducing the node's model call are extracted
/// (temperature, max_tokens, structured-output schema). A missing graph or
/// non-LLM nodes simply yield configs with None params — this only errors on
/// YAML parse failure.
pub fn parse_node_configs(yaml_str: &str) -> Result<HashMap<String, NodeDslConfig>, String> {
    let root: Value = serde_yaml::from_str(yaml_str)
        .map_err(|e| format!("解析 DSL YAML 失败: {}", e))?;

    let mut map = HashMap::new();

    let nodes = root
        .pointer("/workflow/graph/nodes")
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    for node in nodes {
        let id = match normalize_id(node.get("id")) {
            Some(s) => s,
            None => continue,
        };
        let data = match node.get("data") {
            Some(d) => d,
            None => continue,
        };

        let node_type = data
            .get("type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let completion_params = data.pointer("/model/completion_params");
        let temperature = completion_params
            .and_then(|cp| cp.get("temperature"))
            .and_then(|v| v.as_f64());
        let max_tokens = completion_params
            .and_then(|cp| cp.get("max_tokens"))
            .and_then(|v| v.as_i64());
        let enable_thinking = completion_params
            .and_then(|cp| cp.get("enable_thinking"))
            .and_then(|v| v.as_bool());

        let response_format = build_response_format(data);

        map.insert(
            id,
            NodeDslConfig {
                node_type,
                temperature,
                max_tokens,
                enable_thinking,
                response_format,
            },
        );
    }

    // An empty result means the DSL has no recognizable workflow nodes (e.g. a
    // chat-mode app, or an incompatible DSL version). Treat it as an error so
    // the export never silently produces parameter-less records — consistent
    // with the "abort on incomplete params" requirement.
    if map.is_empty() {
        return Err("未从 DSL 解析到任何工作流节点（该应用可能不是工作流类型，或 DSL 结构不兼容此版本）".to_string());
    }

    Ok(map)
}

/// Convert a node's `structured_output` into an OpenAI `response_format`
/// (json_schema). Returns None when structured output is disabled or absent.
fn build_response_format(data: &Value) -> Option<Value> {
    let enabled = data
        .get("structured_output_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !enabled {
        return None;
    }

    let schema = data.pointer("/structured_output/schema")?;

    let name = data
        .get("title")
        .and_then(|v| v.as_str())
        .map(sanitize_schema_name)
        .unwrap_or_else(|| "structured_output".to_string());

    let strict = is_strict_schema(schema);

    Some(json!({
        "type": "json_schema",
        "json_schema": {
            "name": name,
            "schema": schema,
            "strict": strict,
        }
    }))
}

/// Whether a JSON schema satisfies OpenAI's *strict* structured-output rules.
/// OpenAI requires `additionalProperties: false` and all properties in
/// `required` at **every** object level (including objects nested inside
/// `properties` or `array.items`), and the root must be an object. We check
/// recursively and conservatively return false for anything we can't fully
/// verify, so we never emit `strict: true` on a schema OpenAI would reject.
fn is_strict_schema(schema: &Value) -> bool {
    schema_strict_compliant(schema)
}

fn schema_strict_compliant(schema: &Value) -> bool {
    match schema.get("type").and_then(|v| v.as_str()) {
        Some("object") => object_strict_compliant(schema),
        Some("array") => match schema.get("items") {
            // strict requires `items`; the item subschema must itself be compliant
            Some(items) => schema_strict_compliant(items),
            None => false,
        },
        Some("string") | Some("number") | Some("integer") | Some("boolean") | Some("null") => true,
        // unknown types / anyOf / oneOf etc. are not reliably supported under strict
        _ => false,
    }
}

fn object_strict_compliant(schema: &Value) -> bool {
    if schema.get("additionalProperties").and_then(|v| v.as_bool()) != Some(false) {
        return false;
    }
    let props = match schema.get("properties").and_then(|v| v.as_object()) {
        Some(p) => p,
        None => return false,
    };
    let required = match schema.get("required").and_then(|v| v.as_array()) {
        Some(r) => r,
        None => return false,
    };
    let req_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    if !props.keys().all(|k| req_strs.contains(&k.as_str())) {
        return false;
    }
    // every property's subschema must itself be strict-compliant
    props.values().all(|sub| schema_strict_compliant(sub))
}

/// Normalize a YAML node id (may be parsed as number or string) to a stable
/// string for matching against trace `node_id`.
fn normalize_id(v: Option<&Value>) -> Option<String> {
    let v = v?;
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    if let Some(n) = v.as_i64() {
        return Some(n.to_string());
    }
    if let Some(n) = v.as_f64() {
        if n.fract() == 0.0 {
            return Some(format!("{}", n as i64));
        }
        return Some(n.to_string());
    }
    None
}

/// OpenAI schema names must match `^[a-zA-Z0-9_-]+$` and be <=64 chars.
/// Map any other characters (e.g. Chinese titles) to `_`; if nothing
/// alphabetic remains, fall back to a generic name.
fn sanitize_schema_name(s: &str) -> String {
    let mapped: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut name: String = if mapped.chars().any(|c| c.is_ascii_alphabetic()) {
        mapped
    } else {
        "structured_output".to_string()
    };
    // After sanitization the name is ASCII-only, so byte length == char count.
    if name.len() > 64 {
        name = name.chars().take(64).collect();
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DSL: &str = r#"
workflow:
  graph:
    nodes:
      - id: '1765348278003'
        data:
          type: llm
          title: 【市场行情】建议问题
          model:
            mode: chat
            name: GLM-4.7-Flash
            completion_params:
              enable_thinking: false
              temperature: 0.3
          structured_output:
            schema:
              type: object
              additionalProperties: false
              properties:
                provide_temperture:
                  type: boolean
                questions:
                  type: array
                  items:
                    type: string
              required:
                - provide_temperture
                - questions
          structured_output_enabled: true
      - id: '1770000000000'
        data:
          type: code
          title: 后处理
"#;

    #[test]
    fn parses_llm_node_params() {
        let configs = parse_node_configs(SAMPLE_DSL).unwrap();
        let cfg = configs.get("1765348278003").expect("llm node present");
        assert_eq!(cfg.node_type.as_deref(), Some("llm"));
        let temp = cfg.temperature.expect("temperature");
        assert!((temp - 0.3).abs() < 1e-9, "temperature = {}", temp);
        assert_eq!(cfg.enable_thinking, Some(false));
    }

    #[test]
    fn builds_strict_response_format() {
        let configs = parse_node_configs(SAMPLE_DSL).unwrap();
        let cfg = configs.get("1765348278003").unwrap();
        let rf = cfg.response_format.as_ref().expect("response_format");
        assert_eq!(rf.pointer("/type"), Some(&serde_json::json!("json_schema")));
        assert_eq!(rf.pointer("/json_schema/strict"), Some(&serde_json::json!(true)));
        assert_eq!(
            rf.pointer("/json_schema/schema/properties/questions/type"),
            Some(&serde_json::json!("array"))
        );
    }

    #[test]
    fn non_llm_node_has_no_response_format() {
        let configs = parse_node_configs(SAMPLE_DSL).unwrap();
        let cfg = configs.get("1770000000000").expect("code node present");
        assert_eq!(cfg.node_type.as_deref(), Some("code"));
        assert!(cfg.response_format.is_none());
        assert!(cfg.temperature.is_none());
    }

    #[test]
    fn strict_false_when_required_incomplete() {
        let dsl = r#"
workflow:
  graph:
    nodes:
      - id: 'n1'
        data:
          type: llm
          structured_output:
            schema:
              type: object
              additionalProperties: false
              properties:
                a: { type: string }
                b: { type: string }
              required:
                - a
          structured_output_enabled: true
"#;
        let configs = parse_node_configs(dsl).unwrap();
        let cfg = configs.get("n1").unwrap();
        let rf = cfg.response_format.as_ref().unwrap();
        assert_eq!(rf.pointer("/json_schema/strict"), Some(&serde_json::json!(false)));
    }

    #[test]
    fn numeric_node_id_normalized() {
        // An unquoted numeric id in YAML is parsed as a number; ensure it still
        // matches the string node_id carried by traces.
        let dsl = "workflow:\n  graph:\n    nodes:\n      - id: 1765348278003\n        data:\n          type: llm\n";
        let configs = parse_node_configs(dsl).unwrap();
        assert!(configs.contains_key("1765348278003"));
    }

    #[test]
    fn strict_false_when_nested_object_not_compliant() {
        // The nested object `b` lacks additionalProperties:false, so even though
        // the root is compliant, the whole schema must NOT be marked strict.
        let dsl = r#"
workflow:
  graph:
    nodes:
      - id: 'n2'
        data:
          type: llm
          structured_output:
            schema:
              type: object
              additionalProperties: false
              properties:
                a: { type: string }
                b:
                  type: object
                  properties:
                    x: { type: string }
                  required: [x]
              required: [a, b]
          structured_output_enabled: true
"#;
        let configs = parse_node_configs(dsl).unwrap();
        let cfg = configs.get("n2").unwrap();
        let rf = cfg.response_format.as_ref().unwrap();
        assert_eq!(rf.pointer("/json_schema/strict"), Some(&serde_json::json!(false)));
    }

    #[test]
    fn errors_when_no_nodes_parsed() {
        // A chat-mode app (no workflow.graph) must surface an error rather than
        // silently yielding an empty config map.
        let dsl = "app:\n  mode: chat\n  name: x\n";
        let result = parse_node_configs(dsl);
        assert!(result.is_err(), "expected error for node-less DSL");
    }
}
