//! Opt-in portability repair for native Codex Responses gateways.
//!
//! Adapted from agentrouter-responses-shim's sanitizer and yunhanhai's nested
//! encrypted-content fix (ff6277e05beff1bfc61e2a3abc0741deb0d42b3c).
//! See licenses/AgentRouter-Responses-Shim-MIT.txt and docs/PROVENANCE.md.
//! Only protocol carriers are inspected: arbitrary tool arguments, JSON tool
//! results, metadata, IDs inside user data, and literal text are never rewritten.

use super::error::ProxyError;
use crate::app_config::AppType;
use crate::provider::Provider;
use http::Method;
use serde_json::{Map, Value};

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RepairStats {
    pub reasoning_items: usize,
    pub item_ids: usize,
    pub encrypted_fields: usize,
    pub encrypted_blocks: usize,
}

impl RepairStats {
    pub fn changed(&self) -> bool {
        self.reasoning_items + self.item_ids + self.encrypted_fields + self.encrypted_blocks > 0
    }
}

/// No hostname allowlist: any API-key provider using native Responses can opt in.
/// Managed account and cross-protocol adapters keep their own history handling.
pub(crate) fn applies_to(
    enabled: bool,
    app: &AppType,
    method: &Method,
    endpoint: &str,
    provider: &Provider,
) -> bool {
    enabled
        && matches!(app, AppType::Codex)
        && *method == Method::POST
        && matches!(
            endpoint.split('?').next(),
            Some("/responses" | "/v1/responses")
        )
        && !super::providers::is_codex_official_provider(provider)
        && !matches!(
            provider
                .meta
                .as_ref()
                .and_then(|meta| meta.provider_type.as_deref()),
            Some("codex_oauth" | "xai_oauth" | "github_copilot")
        )
        && !super::providers::should_convert_codex_responses_to_chat(provider, endpoint)
        && !super::providers::should_convert_codex_responses_to_anthropic(provider, endpoint)
}

fn unsupported(param: &str, reason: &str) -> ProxyError {
    // Never interpolate request values: history, ciphertext and credentials are private.
    ProxyError::InvalidRequest(format!(
        "Codex Responses compatibility: {reason} ({param}). Replay complete explicit history, \
         or disable Codex encrypted-history compatibility and use the original upstream resource."
    ))
}

fn has_value(object: &Map<String, Value>, key: &str) -> bool {
    object.get(key).is_some_and(|value| !value.is_null())
}

fn is_string(object: &Map<String, Value>, key: &str) -> bool {
    object.get(key).is_some_and(Value::is_string)
}

fn has_explicit_content(object: &Map<String, Value>, key: &str) -> bool {
    object.get(key).is_some_and(|value| match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
    })
}

fn kind(object: &Map<String, Value>) -> Option<&str> {
    object.get("type").and_then(Value::as_str).or_else(|| {
        object
            .get("role")
            .and_then(Value::as_str)
            .map(|_| "message")
    })
}

fn inline_complete(object: &Map<String, Value>) -> Option<bool> {
    match kind(object) {
        Some("message") => Some(is_string(object, "role") && has_value(object, "content")),
        Some("function_call") => Some(
            is_string(object, "call_id")
                && is_string(object, "name")
                && is_string(object, "arguments"),
        ),
        Some("custom_tool_call") => Some(
            is_string(object, "call_id") && is_string(object, "name") && is_string(object, "input"),
        ),
        Some("function_call_output" | "custom_tool_call_output") => {
            Some(is_string(object, "call_id") && has_value(object, "output"))
        }
        _ => None,
    }
}

fn protocol_carrier(object: &Map<String, Value>) -> bool {
    matches!(
        kind(object),
        Some(
            "message"
                | "agent_message"
                | "function_call"
                | "function_call_output"
                | "custom_tool_call"
                | "custom_tool_call_output"
                | "input_text"
                | "output_text"
                | "text"
                | "input_image"
                | "output_image"
                | "input_file"
                | "refusal"
        )
    )
}

/// Work on a clone, committing no partial changes when opaque state is encountered.
pub(crate) fn sanitize(request: &Value) -> Result<(Value, RepairStats), ProxyError> {
    let original = request
        .as_object()
        .ok_or_else(|| unsupported("body", "request must be a JSON object"))?;
    for field in ["previous_response_id", "conversation"] {
        if has_value(original, field) {
            return Err(unsupported(
                field,
                "resource-bound conversation reference cannot be repaired",
            ));
        }
    }
    if let Some(management) = original.get("context_management") {
        let is_compaction = |value: &Value| {
            value.as_object().is_some_and(|object| {
                kind(object) == Some("compaction") || object.contains_key("compact_threshold")
            })
        };
        if is_compaction(management)
            || management
                .as_array()
                .is_some_and(|entries| entries.iter().any(is_compaction))
        {
            return Err(unsupported(
                "context_management",
                "encrypted compaction contains conversation history",
            ));
        }
    }
    if original
        .get("input")
        .is_some_and(|input| !input.is_string() && !input.is_array())
    {
        return Err(unsupported("input", "input must be a string or an array"));
    }
    let mut body = request.clone();
    let mut stats = RepairStats::default();
    if let Some(items) = body.get_mut("input").and_then(Value::as_array_mut) {
        let mut cleaned = Vec::with_capacity(items.len());
        for (index, mut item) in std::mem::take(items).into_iter().enumerate() {
            let param = format!("input[{index}]");
            if clean_item(&mut item, &param, 0, true, &mut stats)? {
                cleaned.push(item);
            }
        }
        if cleaned.is_empty() && stats.changed() {
            return Err(unsupported(
                "input",
                "removing encrypted state would leave no replayable input",
            ));
        }
        *items = cleaned;
    }
    Ok((body, stats))
}

fn clean_item(
    item: &mut Value,
    param: &str,
    depth: usize,
    input_item: bool,
    stats: &mut RepairStats,
) -> Result<bool, ProxyError> {
    if depth > 64 {
        return Err(unsupported(
            "input",
            "protocol content nesting exceeds the repair limit",
        ));
    }
    if let Some(items) = item.as_array_mut() {
        let before = stats.reasoning_items + stats.encrypted_blocks + stats.encrypted_fields;
        clean_array(items, param, depth + 1, stats)?;
        if items.is_empty()
            && stats.reasoning_items + stats.encrypted_blocks + stats.encrypted_fields > before
        {
            return Err(unsupported(
                param,
                "encrypted-only nested content cannot be reconstructed",
            ));
        }
        return Ok(true);
    }
    let Some(object) = item.as_object_mut() else {
        return Ok(true);
    };
    match kind(object) {
        Some("compaction" | "compaction_trigger") => {
            return Err(unsupported(
                param,
                "encrypted compaction contains conversation history",
            ));
        }
        Some("item_reference") => {
            return Err(unsupported(
                param,
                "stored item reference has no explicit replayable content",
            ));
        }
        Some("reasoning") => {
            stats.reasoning_items += 1;
            return Ok(false);
        }
        Some("encrypted_content") if is_string(object, "encrypted_content") => {
            stats.encrypted_blocks += 1;
            return Ok(false);
        }
        _ => {}
    }
    if input_item
        && kind(object).is_none()
        && has_value(object, "id")
        && !has_value(object, "content")
    {
        return Err(unsupported(
            param,
            "ID-only history cannot be reconstructed",
        ));
    }
    if input_item && has_value(object, "id") && inline_complete(object) == Some(false) {
        return Err(unsupported(
            param,
            "incomplete history item requires stored upstream state",
        ));
    }
    if !protocol_carrier(object) {
        if input_item && has_value(object, "encrypted_content") {
            return Err(unsupported(
                param,
                "unknown encrypted input cannot be discarded safely",
            ));
        }
        // Opaque JSON tool data and unknown extension objects are user content.
        return Ok(true);
    }
    if has_value(object, "encrypted_content") {
        if !is_string(object, "encrypted_content") {
            return Err(unsupported(
                param,
                "encrypted protocol content must be a string",
            ));
        }
        // A field-level carrier may contain the only content of a tool output.
        let explicit = ["content", "output", "text", "arguments", "input", "refusal"]
            .iter()
            .any(|field| has_explicit_content(object, field));
        if !explicit {
            return Err(unsupported(
                param,
                "encrypted item has no explicit content to replay",
            ));
        }
        object.remove("encrypted_content");
        stats.encrypted_fields += 1;
    }
    // Descend only protocol content/output arrays, never arbitrary object fields,
    // metadata, tool arguments or schemas (even if their keys have the same names).
    for field in ["content", "output"] {
        if let Some(items) = object.get_mut(field).and_then(Value::as_array_mut) {
            let before = stats.reasoning_items + stats.encrypted_blocks + stats.encrypted_fields;
            clean_array(items, param, depth + 1, stats)?;
            let removed =
                stats.reasoning_items + stats.encrypted_blocks + stats.encrypted_fields > before;
            if items.is_empty() && removed {
                return Err(unsupported(
                    param,
                    "encrypted-only message or tool output cannot be reconstructed",
                ));
            }
        }
    }
    if input_item && inline_complete(object) == Some(true) && object.remove("id").is_some() {
        stats.item_ids += 1;
    }
    Ok(true)
}

fn clean_array(
    items: &mut Vec<Value>,
    param: &str,
    depth: usize,
    stats: &mut RepairStats,
) -> Result<(), ProxyError> {
    let mut cleaned = Vec::with_capacity(items.len());
    for mut item in std::mem::take(items) {
        if clean_item(&mut item, param, depth, false, stats)? {
            cleaned.push(item);
        }
    }
    *items = cleaned;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn applies_across_api_hosts_but_not_accounts_bridges_or_other_endpoints() {
        for base in [
            "https://api-a.example/v1",
            "https://api-b.example/relay",
            "https://api.openai.com/v1",
        ] {
            let mut provider = Provider::with_id(
                "api-key".to_string(),
                "API Key".to_string(),
                json!({"auth":{"OPENAI_API_KEY":"fake"},"base_url":base}),
                None,
            );
            assert!(applies_to(
                true,
                &AppType::Codex,
                &Method::POST,
                "/responses?api-version=test",
                &provider
            ));
            assert!(!applies_to(
                false,
                &AppType::Codex,
                &Method::POST,
                "/responses",
                &provider
            ));
            assert!(!applies_to(
                true,
                &AppType::Claude,
                &Method::POST,
                "/responses",
                &provider
            ));
            assert!(!applies_to(
                true,
                &AppType::GrokBuild,
                &Method::POST,
                "/responses",
                &provider
            ));
            for path in [
                "/models",
                "/responses/compact",
                "/v1/responses/compact",
                "/alpha/search",
                "/chat/completions",
            ] {
                assert!(!applies_to(
                    true,
                    &AppType::Codex,
                    &Method::POST,
                    path,
                    &provider
                ));
            }
            assert!(!applies_to(
                true,
                &AppType::Codex,
                &Method::GET,
                "/responses",
                &provider
            ));
            for format in ["openai_chat", "anthropic"] {
                provider.meta = Some(crate::provider::ProviderMeta {
                    api_format: Some(format.to_string()),
                    ..Default::default()
                });
                assert!(!applies_to(
                    true,
                    &AppType::Codex,
                    &Method::POST,
                    "/responses",
                    &provider
                ));
            }
            for account_type in ["codex_oauth", "github_copilot", "xai_oauth"] {
                provider.meta = Some(crate::provider::ProviderMeta {
                    provider_type: Some(account_type.to_string()),
                    ..Default::default()
                });
                assert!(!applies_to(
                    true,
                    &AppType::Codex,
                    &Method::POST,
                    "/responses",
                    &provider
                ));
            }
        }
        let mut official = Provider::with_id(
            crate::database::CODEX_OFFICIAL_PROVIDER_ID.to_string(),
            "Official".to_string(),
            json!({"auth":{},"config":""}),
            None,
        );
        official.category = Some("official".to_string());
        assert!(!applies_to(
            true,
            &AppType::Codex,
            &Method::POST,
            "/responses",
            &official
        ));
    }

    #[test]
    fn repairs_pooled_history_and_keeps_plaintext_tool_pairing_and_phase() {
        let request = json!({
            "model":"any-model", "stream":true, "store":false,
            "include":["reasoning.encrypted_content"], "reasoning":{"effort":"high"},
            "prompt_cache_key":"stable",
            "input":[
                {"type":"message","id":"msg_1","role":"assistant","phase":"commentary","content":[{"type":"output_text","text":"hello"}]},
                {"type":"reasoning","id":"rs_1","encrypted_content":"opaque","summary":[]},
                {"type":"agent_message","author":"agent","recipient":"parent","content":[{"type":"input_text","text":"handoff"},{"type":"encrypted_content","encrypted_content":"opaque"}]},
                {"type":"function_call","id":"fc_1","call_id":"call_1","name":"tool","arguments":"{\"encrypted_content\":\"business data\",\"id\":3}"},
                {"type":"function_call_output","id":"fo_1","call_id":"call_1","output":[{"type":"input_text","text":"result"},{"type":"encrypted_content","encrypted_content":"opaque"}]}
            ]
        });
        let original = request.clone();
        let (cleaned, stats) = sanitize(&request).unwrap();
        assert_eq!(request, original);
        assert_eq!(
            stats,
            RepairStats {
                reasoning_items: 1,
                item_ids: 3,
                encrypted_fields: 0,
                encrypted_blocks: 2
            }
        );
        assert_eq!(cleaned["input"][0]["phase"], "commentary");
        assert_eq!(
            cleaned["input"][1]["content"],
            json!([{"type":"input_text","text":"handoff"}])
        );
        assert_eq!(
            cleaned["input"][2]["call_id"],
            cleaned["input"][3]["call_id"]
        );
        assert_eq!(
            cleaned["input"][2]["arguments"],
            request["input"][3]["arguments"]
        );
        for field in [
            "model",
            "stream",
            "store",
            "include",
            "reasoning",
            "prompt_cache_key",
        ] {
            assert_eq!(cleaned[field], request[field]);
        }
        let (again, second_stats) = sanitize(&cleaned).unwrap();
        assert_eq!(again, cleaned);
        assert!(!second_stats.changed());
    }

    #[test]
    fn keeps_application_json_nested_ids_schemas_and_text() {
        let payload = json!({"encrypted_content":"business data","id":"application-id","content":[{"type":"encrypted_content","encrypted_content":"literal data"}]});
        let request = json!({
            "metadata":payload,"tools":[{"type":"function","name":"tool","parameters":payload}],
            "input":[{"type":"function_call_output","call_id":"c","output":payload},
                {"role":"user","content":[{"type":"input_text","text":"encrypted_content: gAAAAA literal"}]}]
        });
        let (cleaned, stats) = sanitize(&request).unwrap();
        assert_eq!(cleaned, request);
        assert!(!stats.changed());
    }

    #[test]
    fn repairs_nested_protocol_arrays_without_walking_untyped_json() {
        let request = json!({"input":[{"type":"agent_message","content":[
            {"type":"message","role":"assistant","content":[[{"type":"input_text","text":"keep","encrypted_content":"opaque"},{"type":"encrypted_content","encrypted_content":"opaque"}]]},
            {"metadata":{"encrypted_content":"keep"}}
        ]}]});
        let (cleaned, stats) = sanitize(&request).unwrap();
        assert_eq!(stats.encrypted_fields, 1);
        assert_eq!(stats.encrypted_blocks, 1);
        assert_eq!(
            cleaned["input"][0]["content"][0]["content"][0],
            json!([{"type":"input_text","text":"keep"}])
        );
        assert_eq!(
            cleaned["input"][0]["content"][1],
            request["input"][0]["content"][1]
        );
    }

    #[test]
    fn rejects_resource_bound_or_encrypted_only_history_without_mutation() {
        let cases = [
            json!({"previous_response_id":"resp_secret","input":"continue"}),
            json!({"conversation":{"id":"secret"},"input":"continue"}),
            json!({"context_management":[{"type":"compaction"}],"input":"hello"}),
            json!({"input":[{"type":"compaction","encrypted_content":"secret"}]}),
            json!({"input":[{"type":"item_reference","id":"secret"}]}),
            json!({"input":[{"id":"secret"}]}),
            json!({"input":[{"type":"function_call","id":"secret","name":"tool"}]}),
            json!({"input":[{"type":"future","encrypted_content":"secret"}]}),
            json!({"input":[{"type":"agent_message","content":[{"type":"encrypted_content","encrypted_content":"secret"}]}]}),
            json!({"input":[{"type":"function_call_output","call_id":"c","output":[{"type":"encrypted_content","encrypted_content":"secret"}]}]}),
            json!({"input":[{"type":"reasoning","encrypted_content":"secret"}]}),
            json!({"input":[{"role":"assistant","content":[],"encrypted_content":"secret"}]}),
            json!({"input":[{"type":"function_call_output","call_id":"c","output":"","encrypted_content":"secret"}]}),
            json!({"input":[{"type":"agent_message","content":[[{"type":"encrypted_content","encrypted_content":"secret"}]]}]}),
        ];
        for request in cases {
            let original = request.clone();
            let error = sanitize(&request).unwrap_err();
            assert!(matches!(error, ProxyError::InvalidRequest(_)));
            assert!(!error.to_string().contains("secret"));
            assert_eq!(request, original);
        }
    }

    #[test]
    fn leaves_string_input_and_plain_history_unchanged() {
        for request in [
            json!({"input":"hello"}),
            json!({"input":[{"role":"user","content":"hello"}]}),
        ] {
            let (cleaned, stats) = sanitize(&request).unwrap();
            assert_eq!(cleaned, request);
            assert!(!stats.changed());
        }
    }
}
