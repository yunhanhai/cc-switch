//! Bounded inspection of native Codex Responses before exposing any response.
//!
//! Only explicit, recognised early failures enter the existing retry policy.
//! Unknown events, output/tool activity and exhausted inspection budgets commit
//! the stream. Supported compression is decoded with bounded incremental reads.

use super::{content_encoding::get_content_encoding, hyper_client::ProxyResponse, ProxyError};
use bytes::BytesMut;
use futures::StreamExt;
use http::HeaderMap;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::Instant;

const MAX_PREFIX_BYTES: usize = 256 * 1024;
const MAX_PREFIX_CHUNKS: usize = 1024;
const MAX_INSPECTION_TIME: Duration = Duration::from_secs(5);

pub(crate) fn applies_to(
    app: &crate::app_config::AppType,
    method: &http::Method,
    endpoint: &str,
    provider: &crate::provider::Provider,
    body: &Value,
) -> bool {
    // Share only the native API-key transport filter; response inspection does
    // not depend on opting into lossy encrypted-history cleanup.
    super::codex_responses_compat::applies_to(true, app, method, endpoint, provider)
        && request_is_replayable(body)
}

/// Stateful/background responses and server-run tools may have side effects
/// before any visible output. Leave their errors to the upstream/client.
pub(crate) fn request_is_replayable(body: &Value) -> bool {
    fn client_tool(tool: &Value, depth: usize) -> bool {
        if depth > 16 {
            return false;
        }
        match tool.get("type").and_then(Value::as_str) {
            Some("function" | "custom") => true,
            Some("namespace") => tool
                .get("tools")
                .and_then(Value::as_array)
                .is_some_and(|tools| tools.iter().all(|tool| client_tool(tool, depth + 1))),
            _ => false,
        }
    }
    fn explicit_history(input: &Value, depth: usize) -> bool {
        if depth > 64 {
            return false;
        }
        match input {
            Value::Array(items) => items.iter().all(|item| explicit_history(item, depth + 1)),
            Value::Object(item) => {
                if matches!(
                    item.get("type").and_then(Value::as_str),
                    Some("compaction" | "compaction_trigger" | "item_reference")
                ) || (item.contains_key("id")
                    && !item.contains_key("type")
                    && !item.contains_key("content"))
                {
                    return false;
                }
                // Inspect protocol containers, never JSON inside tool arguments
                // or an arbitrary tool-result object supplied by the client.
                ["content", "output"].iter().all(|field| {
                    item.get(*field)
                        .is_none_or(|value| !value.is_array() || explicit_history(value, depth + 1))
                })
            }
            _ => true,
        }
    }
    body.is_object()
        && body
            .get("background")
            .is_none_or(|value| value == false || value.is_null())
        && ["previous_response_id", "conversation", "context_management"]
            .iter()
            .all(|field| body.get(*field).is_none_or(Value::is_null))
        && body.get("tools").is_none_or(|tools| {
            tools.is_null()
                || tools
                    .as_array()
                    .is_some_and(|tools| tools.iter().all(|tool| client_tool(tool, 0)))
        })
        && body
            .get("input")
            .is_none_or(|input| explicit_history(input, 0))
}

enum Decision {
    Wait,
    Commit,
    Fail(ProxyError),
}

/// The configured timeout can shorten the observation window. At the guard's
/// own time/size limit we commit the original stream; normal stream timeouts
/// still belong to the response processor. Heartbeats never reset this deadline.
pub(crate) async fn inspect(
    response: ProxyResponse,
    configured_timeout: Duration,
) -> Result<ProxyResponse, ProxyError> {
    let response = super::response_decompression::decode_response(
        response,
        super::hyper_client::MAX_RESPONSE_BODY_BYTES,
    );
    let window = if configured_timeout.is_zero() {
        MAX_INSPECTION_TIME
    } else {
        configured_timeout.min(MAX_INSPECTION_TIME)
    };
    inspect_with_limits(response, window, MAX_PREFIX_BYTES, MAX_PREFIX_CHUNKS).await
}

async fn inspect_with_limits(
    response: ProxyResponse,
    window: Duration,
    max_bytes: usize,
    max_chunks: usize,
) -> Result<ProxyResponse, ProxyError> {
    // Unknown or excessive content codings remain observable to the client.
    if get_content_encoding(response.headers()).is_some() {
        return Ok(response);
    }
    let status = response.status();
    let headers = response.headers().clone();
    let mut stream = Box::pin(response.bytes_stream());
    let mut prefix = BytesMut::new();
    let mut cursor = 0;
    let deadline = Instant::now() + window;
    let mut chunks = 0;
    loop {
        let next = match tokio::time::timeout_at(deadline, stream.next()).await {
            Ok(next) => next,
            Err(_) => break,
        };
        let Some(chunk) = next else {
            match inspect_prefix(&prefix, &mut cursor, true, &headers) {
                Decision::Fail(error) => return Err(error),
                Decision::Wait => {
                    return Err(ProxyError::ForwardFailed(
                        "Codex Responses stream ended before output or a terminal event".into(),
                    ));
                }
                Decision::Commit => break,
            }
        };
        let chunk = chunk.map_err(|error| {
            if let Some(limit) = super::response_decompression::exceeded_byte_budget(&error) {
                ProxyError::ResponseBodyTooLarge(limit)
            } else {
                ProxyError::ForwardFailed("Codex Responses stream failed before output".into())
            }
        })?;
        chunks += 1;
        if prefix.len().saturating_add(chunk.len()) > max_bytes {
            let replay = futures::stream::iter([Ok(prefix.freeze()), Ok(chunk)]).chain(stream);
            return Ok(ProxyResponse::streamed(status, headers, replay));
        }
        prefix.extend_from_slice(&chunk);
        match inspect_prefix(&prefix, &mut cursor, false, &headers) {
            Decision::Fail(error) => return Err(error),
            Decision::Commit => break,
            Decision::Wait => {}
        }
        if chunks >= max_chunks || prefix.len() >= max_bytes {
            break;
        }
    }
    let replay = futures::stream::once(async move { Ok(prefix.freeze()) }).chain(stream);
    Ok(ProxyResponse::streamed(status, headers, replay))
}

fn inspect_prefix(bytes: &[u8], cursor: &mut usize, eof: bool, headers: &HeaderMap) -> Decision {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    if bytes.is_empty() || (bytes.len() < 3 && b"\xef\xbb\xbf".starts_with(bytes)) {
        return Decision::Wait;
    }
    let trimmed = bytes.trim_ascii_start();
    if trimmed.is_empty() {
        return Decision::Wait;
    }
    if matches!(trimmed.first(), Some(b'{' | b'[')) {
        return match serde_json::from_slice::<Value>(trimmed) {
            Ok(value) => {
                failure_from_document(&value, headers).map_or(Decision::Commit, Decision::Fail)
            }
            Err(error) if error.is_eof() && !eof => Decision::Wait,
            Err(_) => Decision::Commit,
        };
    }
    // Sniff only SSE syntax. Opaque bodies must remain observable to the client.
    if ![b":".as_slice(), b"data:", b"event:", b"id:", b"retry:"]
        .iter()
        .any(|field| trimmed.starts_with(field) || field.starts_with(trimmed))
    {
        return Decision::Commit;
    }
    while *cursor < bytes.len() {
        let rest = &bytes[*cursor..];
        let boundary = [(b"\r\n\r\n".as_slice(), 4), (b"\n\n", 2), (b"\r\r", 2)]
            .iter()
            .filter_map(|(marker, length)| {
                rest.windows(marker.len())
                    .position(|window| window == *marker)
                    .map(|position| (position, *length))
            })
            .min_by_key(|(position, _)| *position);
        let (end, delimiter) = match boundary {
            Some(boundary) => boundary,
            None if eof => (rest.len(), 0),
            None => return Decision::Wait,
        };
        let block = match std::str::from_utf8(&rest[..end]) {
            Ok(block) => block,
            Err(_) => return Decision::Commit,
        };
        *cursor += end + delimiter;
        match inspect_event(block, headers) {
            Decision::Wait => {}
            outcome => return outcome,
        }
    }
    Decision::Wait
}

fn inspect_event(block: &str, headers: &HeaderMap) -> Decision {
    let mut event = "";
    let mut data = Vec::new();
    for line in block.split(['\r', '\n']) {
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        if let Some(value) = super::sse::strip_sse_field(line, "event") {
            event = value.trim();
        } else if let Some(value) = super::sse::strip_sse_field(line, "data") {
            data.push(value);
        } else if super::sse::strip_sse_field(line, "id").is_none()
            && super::sse::strip_sse_field(line, "retry").is_none()
        {
            return Decision::Commit;
        }
    }
    if data.is_empty() {
        return if event.is_empty() {
            Decision::Wait
        } else {
            Decision::Commit
        };
    }
    let value: Value = match serde_json::from_str(&data.join("\n")) {
        Ok(value) => value,
        Err(_) => return Decision::Commit,
    };
    let payload_type = value.get("type").and_then(Value::as_str).unwrap_or("");
    if event.is_empty() || event == "message" {
        event = payload_type;
    } else if !payload_type.is_empty() && event != payload_type {
        return Decision::Commit;
    }
    let response = value.get("response").unwrap_or(&value);
    if has_output(response) {
        return Decision::Commit;
    }
    match event {
        "response.created" | "response.in_progress" | "response.queued" => {
            if response.get("error").is_some_and(|value| !value.is_null())
                || response
                    .get("background")
                    .is_some_and(|value| value == true)
                || response
                    .get("status")
                    .and_then(Value::as_str)
                    .is_some_and(|status| !matches!(status, "queued" | "in_progress"))
            {
                return Decision::Commit;
            }
            Decision::Wait
        }
        "response.failed" | "error" => {
            failure_from_document(response, headers).map_or(Decision::Commit, Decision::Fail)
        }
        // Includes output, tool activity, incomplete/cancelled/completed and
        // unknown extensions. Once seen, later errors never replay this request.
        _ => Decision::Commit,
    }
}

fn has_output(value: &Value) -> bool {
    value.get("output").is_some_and(|output| match output {
        Value::Null => false,
        Value::Array(items) => !items.is_empty(),
        _ => true,
    }) || value
        .pointer("/usage/output_tokens")
        .and_then(Value::as_u64)
        .is_some_and(|count| count > 0)
}

fn failure_from_document(value: &Value, headers: &HeaderMap) -> Option<ProxyError> {
    if has_output(value)
        || value.get("background").is_some_and(|value| value == true)
        || matches!(
            value.get("status").and_then(Value::as_str),
            Some("completed" | "incomplete" | "cancelled")
        )
    {
        return None;
    }
    let error = value
        .get("error")
        .filter(|value| value.is_object())
        .or_else(|| {
            (value.get("type").and_then(Value::as_str) == Some("error")).then_some(value)
        })?;
    let code = error
        .get("code")
        .and_then(Value::as_str)
        .filter(|code| !code.is_empty());
    let kind = error
        .get("type")
        .and_then(Value::as_str)
        .filter(|kind| !kind.is_empty());
    let tag = code.or(kind)?;
    let http_status = error
        .get("status_code")
        .or_else(|| error.get("status"))
        .and_then(Value::as_u64)
        .filter(|status| (400..600).contains(status));
    // Classification uses protocol fields, never substrings in user-supplied
    // messages. Only fixed diagnostics are returned/logged, not upstream bodies.
    if super::error::is_billing_error_object(error) {
        return Some(ProxyError::BillingExhausted(
            "Responses upstream reports exhausted billing quota".into(),
        ));
    }
    let (status, error_type, error_code, message) = match tag {
        "rate_limit_exceeded" | "rate_limit_error" | "too_many_requests" => (429, "rate_limit_error", tag, "Responses upstream is rate limited."),
        "server_error" | "internal_server_error" | "internal_error" => (502, "server_error", tag, "Responses upstream reported an internal error before output."),
        "overloaded_error" | "service_unavailable" => (503, "server_error", tag, "Responses upstream is temporarily unavailable."),
        "invalid_api_key" | "authentication_error" | "unauthorized" => (401, "authentication_error", tag, "Responses upstream rejected the API credential."),
        "permission_denied" | "permission_error" | "forbidden" => (403, "permission_error", tag, "Responses upstream denied access."),
        "model_not_found" => (404, "invalid_request_error", tag, "Responses upstream does not have access to the requested model."),
        "invalid_encrypted_content" => (400, "invalid_request_error", tag, "Responses upstream rejected encrypted history. For an affected gateway, use Codex encrypted-history compatibility with complete explicit history."),
        "invalid_request_error" | "invalid_prompt" | "context_length_exceeded" | "invalid_value" | "missing_required_parameter" | "unsupported_parameter" | "invalid_parameter" => (400, "invalid_request_error", tag, "Responses upstream rejected the request. Check its parameters and conversation history."),
        _ => match http_status {
            Some(status) => (status as u16, "upstream_error", "upstream_error", "Responses upstream returned an explicit error status before output."),
            None => return None,
        },
    };
    if status == 429 {
        let retry_after_secs = super::error::retry_after_from_headers(headers)
            .or_else(|| super::error::retry_after_from_error(error));
        return Some(ProxyError::RateLimited {
            retry_after_secs,
            body: None,
        });
    }
    Some(ProxyError::UpstreamError {
        status,
        body: Some(
            json!({"error":{"type":error_type,"code":error_code,"message":message}}).to_string(),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn response(chunks: Vec<Bytes>, headers: HeaderMap) -> ProxyResponse {
        ProxyResponse::streamed(
            http::StatusCode::OK,
            headers,
            futures::stream::iter(chunks.into_iter().map(Ok)),
        )
    }

    async fn replay(response: ProxyResponse) -> Bytes {
        inspect(response, Duration::ZERO)
            .await
            .expect("passthrough")
            .bytes_with_limit(1024 * 1024)
            .await
            .expect("replayed body")
    }

    const CREATED: &str = "event: response.created\r\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_test\",\"status\":\"in_progress\",\"output\":[]}}\r\n\r\n";
    const LIMITED: &str = "event: response.failed\r\ndata: {\"type\":\"response.failed\",\"response\":{\"status\":\"failed\",\"error\":{\"code\":\"rate_limit_exceeded\",\"message\":\"private-upstream-value\",\"retry_after_seconds\":2}}}\r\n\r\n";

    #[test]
    fn only_replayable_api_key_requests_are_inspected() {
        use crate::{app_config::AppType, provider::Provider};
        let provider = Provider::with_id(
            "api".into(),
            "API".into(),
            json!({"auth":{"OPENAI_API_KEY":"fake"},"base_url":"https://custom.example/v1"}),
            None,
        );
        for request in [
            json!({"input":"hi"}),
            json!({"tools":[{"type":"function","name":"f"},{"type":"custom","name":"c"}]}),
            json!({"tools":[{"type":"namespace","tools":[{"type":"function","name":"f"}]}]}),
        ] {
            assert!(applies_to(
                &AppType::Codex,
                &http::Method::POST,
                "/v1/responses",
                &provider,
                &request
            ));
            assert!(!applies_to(
                &AppType::Claude,
                &http::Method::POST,
                "/v1/responses",
                &provider,
                &request
            ));
            assert!(!applies_to(
                &AppType::Codex,
                &http::Method::POST,
                "/responses/compact",
                &provider,
                &request
            ));
        }
        for request in [
            json!({"previous_response_id":"resp_1"}),
            json!({"conversation":"conv_1"}),
            json!({"background":true}),
            json!({"context_management":[]}),
            json!({"tools":[{"type":"web_search"}]}),
            json!({"tools":[{"type":"mcp"}]}),
            json!({"tools":[{"type":"namespace","tools":[{"type":"computer"}]}]}),
            json!({"input":[{"type":"item_reference","id":"stored"}]}),
            json!({"input":[{"type":"compaction","encrypted_content":"opaque"}]}),
            json!({"input":[{"id":"stored"}]}),
        ] {
            assert!(!request_is_replayable(&request), "{request}");
        }
    }

    #[tokio::test]
    async fn fragmented_sse_rate_limit_is_detected_and_does_not_expose_private_values() {
        let input = format!("\u{feff}: heartbeat\r\n\r\n{CREATED}{LIMITED}");
        let chunks = input
            .as_bytes()
            .chunks(3)
            .map(Bytes::copy_from_slice)
            .collect();
        let mut headers = HeaderMap::new();
        headers.insert("retry-after", "7".parse().unwrap());
        let error = inspect(response(chunks, headers), Duration::ZERO)
            .await
            .err()
            .expect("early error");
        assert!(matches!(
            error,
            ProxyError::RateLimited {
                retry_after_secs: Some(7),
                ..
            }
        ));
        assert!(!error.to_string().contains("private-upstream-value"));
    }

    #[test]
    fn error_codes_take_priority_over_generic_types_and_messages() {
        let headers = HeaderMap::new();
        let billing = json!({"error":{"type":"rate_limit_error","code":"insufficient_quota","message":"retry later"}});
        assert!(matches!(
            failure_from_document(&billing, &headers),
            Some(ProxyError::BillingExhausted(_))
        ));
        for (code, expected) in [
            ("server_error", 502),
            ("overloaded_error", 503),
            ("invalid_encrypted_content", 400),
            ("context_length_exceeded", 400),
            ("invalid_api_key", 401),
            ("permission_denied", 403),
        ] {
            let error = failure_from_document(&json!({"error":{"type":"server_error","code":code,"message":"private-upstream-value"}}), &headers).unwrap();
            if let ProxyError::UpstreamError {
                body: Some(body), ..
            } = &error
            {
                assert_eq!(
                    serde_json::from_str::<Value>(body).unwrap()["error"]["code"],
                    code
                );
            }
            assert!(matches!(error, ProxyError::UpstreamError {status,..} if status == expected));
            assert!(!error.to_string().contains("private-upstream-value"));
        }
        assert!(failure_from_document(
            &json!({"error":{"code":"unknown","message":"rate_limit_exceeded"}}),
            &headers
        )
        .is_none());
        assert!(
            failure_from_document(&json!({"error":{"message":"server_error"}}), &headers).is_none()
        );
        let flat = "event: error\ndata: {\"type\":\"error\",\"code\":\"rate_limit_exceeded\",\"retry_after\":\"3\"}";
        assert!(matches!(
            inspect_event(flat, &headers),
            Decision::Fail(ProxyError::RateLimited {
                retry_after_secs: Some(3),
                ..
            })
        ));
    }

    #[tokio::test]
    async fn stream_wait_hints_share_http_header_precedence_and_body_fallback() {
        let body = Bytes::from_static(b"data: {\"type\":\"error\",\"code\":\"rate_limit_exceeded\",\"retry_after_seconds\":null,\"retry_after\":\"17\"}\n\n");
        for (values, expected) in [(vec![], 17), (vec!["invalid", "11", "23"], 23)] {
            let mut headers = HeaderMap::new();
            for value in values {
                headers.append(http::header::RETRY_AFTER, value.parse().unwrap());
            }
            assert!(
                matches!(inspect(response(vec![body.clone()], headers), Duration::ZERO).await, Err(ProxyError::RateLimited { retry_after_secs: Some(seconds), .. }) if seconds == expected)
            );
        }
    }

    #[tokio::test]
    async fn output_unknown_events_and_terminal_results_commit_before_later_errors() {
        for event in [
            "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"你好\"}\n\n",
            "event: response.output_item.added\ndata: {\"type\":\"response.output_item.added\",\"item\":{\"type\":\"function_call\"}}\n\n",
            "data: {\"type\":\"response.web_search_call.in_progress\"}\n\n",
            "data: {\"type\":\"future.extension\"}\n\n",
            "data: [DONE]\n\n",
            "data: {\"type\":\"response.incomplete\",\"response\":{\"status\":\"incomplete\"}}\n\n",
            "data: {\"type\":\"response.cancelled\",\"response\":{\"status\":\"cancelled\"}}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[]}}\n\n",
            "event: response.failed\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"mismatch\"}\n\n",
        ] {
            let input = format!("{CREATED}{event}{LIMITED}");
            let chunks = input.as_bytes().chunks(7).map(Bytes::copy_from_slice).collect();
            assert_eq!(replay(response(chunks, HeaderMap::new())).await.as_ref(), input.as_bytes());
        }
        let failed_with_output = json!({"type":"response.failed","response":{"status":"failed","output":[{"type":"function_call","call_id":"c"}],"error":{"code":"server_error"}}});
        assert!(matches!(
            inspect_event(&format!("data: {failed_with_output}"), &HeaderMap::new()),
            Decision::Commit
        ));
    }

    #[tokio::test]
    async fn json_errors_are_inspected_but_success_and_partial_results_are_unchanged() {
        for object in [
            json!({"status":"completed","output":[]}),
            json!({"status":"incomplete","error":{"code":"server_error"}}),
            json!({"status":"failed","output":[{"type":"message"}],"error":{"code":"server_error"}}),
            json!({"error":{"code":"unknown"}}),
        ] {
            let raw = serde_json::to_vec_pretty(&object).unwrap();
            assert_eq!(
                replay(response(
                    raw.chunks(5).map(Bytes::copy_from_slice).collect(),
                    HeaderMap::new()
                ))
                .await
                .as_ref(),
                raw
            );
        }
        let raw = br#"{"status":"failed","error":{"code":"server_error"},"output":[]}"#;
        assert!(matches!(
            inspect(
                response(vec![Bytes::from_static(raw)], HeaderMap::new()),
                Duration::ZERO
            )
            .await,
            Err(ProxyError::UpstreamError { status: 502, .. })
        ));
    }

    #[tokio::test]
    async fn unknown_compression_and_over_budget_prefixes_are_replayed_without_parsing() {
        let mut headers = HeaderMap::new();
        headers.insert("content-encoding", "snappy".parse().unwrap());
        assert_eq!(
            replay(response(
                vec![Bytes::from_static(LIMITED.as_bytes())],
                headers
            ))
            .await
            .as_ref(),
            LIMITED.as_bytes()
        );
        let raw = Bytes::from(format!("{CREATED}{LIMITED}"));
        let result = inspect_with_limits(
            response(vec![raw.clone()], HeaderMap::new()),
            Duration::from_secs(1),
            10,
            10,
        )
        .await
        .unwrap();
        assert_eq!(result.bytes_with_limit(1024 * 1024).await.unwrap(), raw);
        let result = inspect_with_limits(
            response(
                vec![
                    Bytes::from_static(b": ping\n\n"),
                    Bytes::from_static(LIMITED.as_bytes()),
                ],
                HeaderMap::new(),
            ),
            Duration::from_secs(1),
            1024,
            1,
        )
        .await
        .unwrap();
        assert_eq!(
            result.bytes_with_limit(1024 * 1024).await.unwrap(),
            Bytes::from(format!(": ping\n\n{LIMITED}"))
        );
    }

    #[tokio::test]
    async fn heartbeat_does_not_extend_inspection_deadline_or_lose_buffered_bytes() {
        let after_deadline_polled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let observed = after_deadline_polled.clone();
        let chunks = futures::stream::unfold(0, move |index| {
            let observed = observed.clone();
            async move {
                if index == 2 {
                    return None;
                }
                if index == 1 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    observed.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                Some((Ok(Bytes::from_static(b": ping\n\n")), index + 1))
            }
        });
        let guarded = inspect_with_limits(
            ProxyResponse::streamed(http::StatusCode::OK, HeaderMap::new(), chunks),
            Duration::from_millis(25),
            1024,
            100,
        )
        .await
        .unwrap();
        assert!(
            !after_deadline_polled.load(std::sync::atomic::Ordering::SeqCst),
            "heartbeats reset the deadline"
        );
        let bytes = guarded.bytes_with_limit(1024).await.unwrap();
        assert_eq!(bytes.as_ref(), b": ping\n\n".repeat(2));
    }

    #[tokio::test]
    async fn completed_output_is_not_delayed_by_a_later_stalled_stream() {
        let productive = Bytes::from_static(
            b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"hi\"}\n\n",
        );
        let chunks = futures::stream::once({
            let first = productive.clone();
            async move { Ok(first) }
        })
        .chain(futures::stream::pending());
        let guarded = tokio::time::timeout(
            Duration::from_secs(1),
            inspect(
                ProxyResponse::streamed(http::StatusCode::OK, HeaderMap::new(), chunks),
                Duration::ZERO,
            ),
        )
        .await
        .expect("output must not wait")
        .unwrap();
        let mut replay = Box::pin(guarded.bytes_stream());
        assert_eq!(replay.next().await.unwrap().unwrap(), productive);
    }

    #[tokio::test]
    async fn gzip_responses_failure_reaches_guard_as_decoded_early_rate_limit() {
        use std::io::Write;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(LIMITED.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("content-encoding", "gzip".parse().unwrap());
        headers.insert("content-type", "text/event-stream".parse().unwrap());
        let upstream = response(
            compressed.chunks(7).map(Bytes::copy_from_slice).collect(),
            headers,
        );
        // Normal forwarding must decode a supported compressed Responses stream
        // before the guard decides whether an early failure can be recovered.
        let inspected = inspect(upstream, Duration::ZERO).await;
        assert!(
            matches!(
                inspected,
                Err(ProxyError::RateLimited {
                    retry_after_secs: Some(2),
                    ..
                })
            ),
            "compressed rate limit bypassed response inspection"
        );
    }

    #[tokio::test]
    async fn decoding_budget_failure_stays_non_retryable_before_output() {
        use std::io::Write;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&vec![b'a'; 4096]).unwrap();
        let encoded = encoder.finish().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("content-encoding", "gzip".parse().unwrap());
        let decoded = super::super::response_decompression::decode_response(
            response(vec![Bytes::from(encoded)], headers),
            128,
        );
        assert!(matches!(
            inspect(decoded, Duration::ZERO).await,
            Err(ProxyError::ResponseBodyTooLarge(128))
        ));
    }

    #[tokio::test]
    async fn empty_or_lifecycle_only_stream_fails_before_response_delivery() {
        for data in ["", CREATED] {
            assert!(matches!(
                inspect(
                    response(
                        vec![Bytes::copy_from_slice(data.as_bytes())],
                        HeaderMap::new()
                    ),
                    Duration::ZERO
                )
                .await,
                Err(ProxyError::ForwardFailed(_))
            ));
        }
    }
}
