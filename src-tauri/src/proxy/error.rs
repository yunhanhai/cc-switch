use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("上游响应体超过大小上限: {0} 字节")]
    ResponseBodyTooLarge(usize),

    #[error("服务器已在运行")]
    AlreadyRunning,

    #[error("服务器未运行")]
    NotRunning,

    #[error("地址绑定失败: {0}")]
    BindFailed(String),

    #[error("停止超时")]
    StopTimeout,

    #[error("停止失败: {0}")]
    StopFailed(String),

    #[error("请求转发失败: {0}")]
    ForwardFailed(String),

    #[error("无可用的Provider")]
    NoAvailableProvider,

    #[error("所有供应商已熔断，无可用渠道")]
    AllProvidersCircuitOpen,

    #[error("未配置供应商")]
    NoProvidersConfigured,

    #[allow(dead_code)]
    #[error("Provider不健康: {0}")]
    ProviderUnhealthy(String),

    #[error("上游错误 (状态码 {status}): {body:?}")]
    UpstreamError { status: u16, body: Option<String> },

    #[error("上游额度耗尽 (403): {0}")]
    BillingExhausted(String),

    /// 上游限流（429）：携带 Retry-After 提示（秒），供等待重试决策使用
    #[error("上游限流 (429), Retry-After: {retry_after_secs:?}秒")]
    RateLimited {
        retry_after_secs: Option<u64>,
        body: Option<String>,
    },

    #[error("超过最大重试次数")]
    MaxRetriesExceeded,

    #[error("数据库错误: {0}")]
    DatabaseError(String),

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[allow(dead_code)]
    #[error("格式转换错误: {0}")]
    TransformError(String),

    #[allow(dead_code)]
    #[error("无效的请求: {0}")]
    InvalidRequest(String),

    #[error("超时: {0}")]
    Timeout(String),

    /// 流式响应空闲超时
    #[allow(dead_code)]
    #[error("流式响应空闲超时: {0}秒无数据")]
    StreamIdleTimeout(u64),

    /// 认证错误
    #[error("认证失败: {0}")]
    AuthError(String),

    #[allow(dead_code)]
    #[error("内部错误: {0}")]
    Internal(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            ProxyError::UpstreamError {
                status: upstream_status,
                body: upstream_body,
            } => {
                let http_status =
                    StatusCode::from_u16(*upstream_status).unwrap_or(StatusCode::BAD_GATEWAY);

                // 尝试解析上游响应体为 JSON，如果失败则包装为字符串
                let error_body = if let Some(body_str) = upstream_body {
                    if let Ok(json_body) = serde_json::from_str::<serde_json::Value>(body_str) {
                        // 上游返回的是 JSON，直接透传
                        json_body
                    } else {
                        // 上游返回的不是 JSON，包装为错误消息
                        json!({
                            "error": {
                                "message": body_str,
                                "type": "upstream_error",
                            }
                        })
                    }
                } else {
                    json!({
                        "error": {
                            "message": format!("Upstream error (status {})", upstream_status),
                            "type": "upstream_error",
                        }
                    })
                };

                (http_status, error_body)
            }
            _ => {
                let (http_status, message) = match &self {
                    ProxyError::AlreadyRunning => (StatusCode::CONFLICT, self.to_string()),
                    ProxyError::NotRunning => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
                    ProxyError::BindFailed(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
                    }
                    ProxyError::StopTimeout => {
                        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
                    }
                    ProxyError::StopFailed(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
                    }
                    ProxyError::ForwardFailed(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
                    ProxyError::NoAvailableProvider => {
                        (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
                    }
                    ProxyError::AllProvidersCircuitOpen => {
                        (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
                    }
                    ProxyError::NoProvidersConfigured => {
                        (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
                    }
                    ProxyError::ProviderUnhealthy(_) => {
                        (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
                    }
                    ProxyError::MaxRetriesExceeded => {
                        (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
                    }
                    ProxyError::DatabaseError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
                    }
                    ProxyError::ConfigError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
                    ProxyError::TransformError(_) => {
                        (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
                    }
                    ProxyError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
                    ProxyError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, self.to_string()),
                    ProxyError::StreamIdleTimeout(_) => {
                        (StatusCode::GATEWAY_TIMEOUT, self.to_string())
                    }
                    ProxyError::AuthError(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
                    ProxyError::Internal(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
                    }
                    ProxyError::ResponseBodyTooLarge(_) => {
                        (StatusCode::BAD_GATEWAY, self.to_string())
                    }
                    ProxyError::UpstreamError { .. } => unreachable!(),
                    ProxyError::BillingExhausted(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
                    // 限流：透传 429 与 Retry-After，客户端可按规范自行等待重试
                    ProxyError::RateLimited {
                        retry_after_secs, ..
                    } => {
                        let mut resp = axum::Json(json!({
                            "error": {
                                "message": self.to_string(),
                                "type": "rate_limit_error",
                            }
                        }))
                        .into_response();
                        if let Some(secs) = retry_after_secs {
                            if let Ok(value) = axum::http::HeaderValue::from_str(&secs.to_string())
                            {
                                resp.headers_mut().insert("retry-after", value);
                            }
                        }
                        *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                        return resp;
                    }
                };

                let error_body = json!({
                    "error": {
                        "message": message,
                        "type": "proxy_error",
                    }
                });

                (http_status, error_body)
            }
        };

        (status, Json(body)).into_response()
    }
}

/// Detect quota/billing exhaustion without classifying ordinary permissions or rate limits.
pub fn is_billing_exhausted_body(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("billing_error")
        || lower.contains("credit balance is too low")
        || lower.contains("insufficient_quota")
        || lower.contains("billing_hard_limit_reached")
        || lower.contains("payment required")
        || (lower.contains("quota") && (lower.contains("exceeded") || lower.contains("exhausted")))
        || (lower.contains("spending limit")
            && (lower.contains("reached") || lower.contains("exceeded")))
        || (lower.contains("usage limit")
            && (lower.contains("reached") || lower.contains("exceeded")))
}

/// Structured quota errors must not enter the transient 429 wait loop. Message
/// text alone is not enough: some gateways mention quota in ordinary limits.
pub(crate) fn is_billing_error_object(error: &serde_json::Value) -> bool {
    ["code", "type"].iter().any(|field| {
        matches!(
            error.get(*field).and_then(serde_json::Value::as_str),
            Some(
                "insufficient_quota"
                    | "billing_hard_limit_reached"
                    | "billing_error"
                    | "payment_required"
            )
        )
    })
}

/// Take the longest valid repeated Retry-After header. Do not split on commas:
/// a valid HTTP-date itself contains a comma.
pub(crate) fn retry_after_from_headers(headers: &http::HeaderMap) -> Option<u64> {
    headers
        .get_all(http::header::RETRY_AFTER)
        .iter()
        .filter_map(|value| value.to_str().ok().and_then(parse_retry_after))
        .max()
}

pub(crate) fn retry_after_from_error(error: &serde_json::Value) -> Option<u64> {
    ["retry_after_seconds", "retry_after"]
        .iter()
        .filter_map(|field| error.get(*field))
        .filter_map(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str().and_then(parse_retry_after))
        })
        .max()
}

/// 解析上游 `Retry-After` 响应头
///
/// 支持两种规范格式：
/// - 秒数：`Retry-After: 30`
/// - HTTP-date：`Retry-After: Wed, 21 Oct 2026 07:28:00 GMT`（转为相对秒数）
///
/// 非法值返回 `None`，由调用方走指数退避兜底。
pub fn parse_retry_after(value: &str) -> Option<u64> {
    parse_retry_after_at(value, chrono::Utc::now())
}

fn parse_retry_after_at(value: &str, now: chrono::DateTime<chrono::Utc>) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 数字秒
    if trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        // A syntactically valid but enormous wait must not fall back to a
        // one-second retry. Saturation lets the existing wait cap reject it.
        return Some(trimmed.parse::<u64>().unwrap_or(u64::MAX));
    }

    // HTTP-date（RFC 7231）
    if let Ok(ts) = chrono::DateTime::parse_from_rfc2822(trimmed) {
        let remaining = ts.signed_duration_since(now).to_std().ok()?;
        return Some(
            remaining
                .as_secs()
                .saturating_add(u64::from(remaining.subsec_nanos() > 0)),
        );
    }

    None
}

#[cfg(test)]
mod rate_limit_tests {
    use super::*;

    #[test]
    fn parses_numeric_seconds() {
        assert_eq!(parse_retry_after("30"), Some(30));
        assert_eq!(parse_retry_after("  120  "), Some(120));
        assert_eq!(parse_retry_after("0"), Some(0));
    }

    #[test]
    fn parses_http_date_as_relative_seconds() {
        let future = chrono::Utc::now().timestamp() + 90;
        let date = chrono::DateTime::from_timestamp(future, 0)
            .unwrap()
            .to_rfc2822();
        let parsed = parse_retry_after(&date).unwrap_or(0);
        assert!((75..=95).contains(&parsed), "parsed={parsed}");
    }

    #[test]
    fn returns_none_for_invalid_or_past_values() {
        assert_eq!(parse_retry_after(""), None);
        assert_eq!(parse_retry_after("abc"), None);
        assert_eq!(parse_retry_after("-5"), None);
        assert_eq!(parse_retry_after("+5"), None);
    }

    #[test]
    fn repeated_headers_use_the_longest_valid_wait_and_ignore_bad_values() {
        let mut headers = http::HeaderMap::new();
        for value in ["invalid", "1", "30", "2"] {
            headers.append(http::header::RETRY_AFTER, value.parse().unwrap());
        }
        assert_eq!(retry_after_from_headers(&headers), Some(30));
        let error = serde_json::json!({"retry_after_seconds":null,"retry_after":"7"});
        assert_eq!(retry_after_from_error(&error), Some(7));
        assert_eq!(
            retry_after_from_error(&serde_json::json!({"retry_after_seconds":2,"retry_after":9})),
            Some(9)
        );
    }

    #[test]
    fn date_wait_does_not_undershoot_and_numeric_overflow_does_not_retry_early() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-11-14T00:00:00.250Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(
            parse_retry_after_at("Sat, 14 Nov 2026 00:00:02 GMT", now),
            Some(2)
        );
        assert_eq!(
            parse_retry_after_at("Sat, 14 Nov 2026 00:00:00 GMT", now),
            None
        );
        assert_eq!(
            parse_retry_after("9999999999999999999999999999999999"),
            Some(u64::MAX)
        );
    }
}

#[cfg(test)]
mod billing_tests {
    use super::{is_billing_error_object, is_billing_exhausted_body};

    #[test]
    fn detects_known_billing_payloads() {
        assert!(is_billing_exhausted_body(
            "billing_error: credit balance is too low"
        ));
        assert!(is_billing_exhausted_body("insufficient_quota"));
        assert!(is_billing_exhausted_body("monthly quota exceeded"));
    }

    #[test]
    fn ignores_rate_limits_permissions_and_empty_payloads() {
        assert!(!is_billing_exhausted_body(
            "rate limit exceeded; retry after 30 seconds"
        ));
        assert!(!is_billing_exhausted_body("permission denied"));
        assert!(!is_billing_exhausted_body(""));
    }

    #[test]
    fn structured_quota_classification_ignores_untrusted_message_text() {
        assert!(is_billing_error_object(
            &serde_json::json!({"type":"rate_limit_error","code":"insufficient_quota"})
        ));
        assert!(is_billing_error_object(
            &serde_json::json!({"type":"billing_error"})
        ));
        assert!(!is_billing_error_object(
            &serde_json::json!({"code":"rate_limit_exceeded","message":"insufficient_quota mentioned by a gateway"})
        ));
        assert!(!is_billing_error_object(
            &serde_json::json!({"metadata":{"code":"insufficient_quota"}})
        ));
    }
}

/// 错误分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// 可重试错误（网络问题、5xx）
    Retryable, // 网络超时、5xx 错误
    /// 不可重试错误（4xx、认证失败）
    NonRetryable, // 认证失败、参数错误、4xx 错误
    #[allow(dead_code)]
    ClientAbort, // 客户端主动中断
}

/// 判断错误是否可重试
#[allow(dead_code)]
pub fn categorize_error(error: &reqwest::Error) -> ErrorCategory {
    if error.is_timeout() || error.is_connect() {
        return ErrorCategory::Retryable;
    }

    if let Some(status) = error.status() {
        if status.is_server_error() {
            ErrorCategory::Retryable
        } else if status.is_client_error() {
            ErrorCategory::NonRetryable
        } else {
            ErrorCategory::Retryable
        }
    } else {
        ErrorCategory::Retryable
    }
}
