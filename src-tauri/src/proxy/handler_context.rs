//! 请求上下文模块
//!
//! 提供请求生命周期的上下文管理，封装通用初始化逻辑

use crate::app_config::AppType;
use crate::provider::Provider;
use crate::proxy::{
    extract_session_id,
    forwarder::RequestForwarder,
    server::ProxyState,
    types::{AppProxyConfig, CopilotOptimizerConfig, OptimizerConfig, RectifierConfig},
    ProxyError,
};
use axum::http::HeaderMap;
use std::time::Instant;

/// 该 app 的模型名是否位于请求体（`body["model"]`）里。
///
/// `gemini == false`：模型名在 URL 路径中（`/v1beta/models/<model>:generateContent`），
/// 通过路径重写实现 alias 路由；其余 app 在 body 中。
pub(crate) fn model_name_lives_in_body(app_type_str: &str) -> bool {
    app_type_str != "gemini"
}

/// 该 app 是否支持 model alias 路由（选路过滤 + 转发换名成对）。
///
/// 与 `model_name_lives_in_body` 的区别：Gemini 的模型名虽然在 URL 路径里，
/// 但通过路径重写也能让「过滤」与「换名」成对，因此同样返回 true。
/// `model_name_lives_in_body` 仍用于判断模型名在 body 还是 path。
pub(crate) fn supports_model_alias(app_type_str: &str) -> bool {
    matches!(app_type_str, "claude" | "codex" | "gemini" | "grokbuild")
}

/// 流式超时配置
#[derive(Debug, Clone, Copy)]
pub struct StreamingTimeoutConfig {
    /// 首字节超时（秒），0 表示禁用
    pub first_byte_timeout: u64,
    /// 静默期超时（秒），0 表示禁用
    pub idle_timeout: u64,
}

/// 请求上下文
///
/// 贯穿整个请求生命周期，包含：
/// - 计时信息
/// - 应用级代理配置（per-app）
/// - 选中的 Provider 列表（用于故障转移）
/// - 请求模型名称
/// - 日志标签
/// - Session ID（用于日志关联）
pub struct RequestContext {
    /// 请求开始时间
    pub start_time: Instant,
    /// 应用级代理配置（per-app，包含重试次数和超时配置）
    pub app_config: AppProxyConfig,
    /// 选中的 Provider（故障转移链的第一个）
    pub provider: Provider,
    /// 完整的 Provider 列表（用于故障转移）
    providers: Vec<Provider>,
    /// 请求开始时的"当前供应商"（用于判断是否需要同步 UI/托盘）
    ///
    /// 这里使用本地 settings 的设备级 current provider。
    /// 代理模式下如果实际使用的 provider 与此不一致，会触发切换以确保 UI 始终准确。
    pub current_provider_id: String,
    /// 请求中的模型名称
    pub request_model: String,
    /// 实际发往上游的模型名（路由接管/模型映射后的真值，forward 成功后回填）。
    ///
    /// usage 归因的兜底顺序：上游响应回显 → outbound_model → request_model。
    /// 不能直接用 request_model 兜底：接管场景下它是映射前的客户端别名。
    pub outbound_model: Option<String>,
    /// 日志标签（如 "Claude"、"Codex"、"Gemini"）
    pub tag: &'static str,
    /// 应用类型字符串（如 "claude"、"codex"、"gemini"）
    pub app_type_str: &'static str,
    /// 应用类型（预留，目前通过 app_type_str 使用）
    #[allow(dead_code)]
    pub app_type: AppType,
    /// Session ID（从客户端请求提取或新生成）
    pub session_id: String,
    /// Session ID 是否由客户端提供。生成的 UUID 不能作为上游缓存 key，否则每个请求都会换 key。
    pub session_client_provided: bool,
    /// 整流器配置
    pub rectifier_config: RectifierConfig,
    /// 优化器配置
    pub optimizer_config: OptimizerConfig,
    /// Copilot 优化器配置
    pub copilot_optimizer_config: CopilotOptimizerConfig,
}

impl RequestContext {
    /// 创建请求上下文
    ///
    /// # Arguments
    /// * `state` - 代理服务器状态
    /// * `body` - 请求体 JSON
    /// * `headers` - 请求头（用于提取 Session ID）
    /// * `app_type` - 应用类型
    /// * `tag` - 日志标签
    /// * `app_type_str` - 应用类型字符串
    ///
    /// # Errors
    /// 返回 `ProxyError` 如果 Provider 选择失败
    pub async fn new(
        state: &ProxyState,
        body: &serde_json::Value,
        headers: &HeaderMap,
        app_type: AppType,
        tag: &'static str,
        app_type_str: &'static str,
    ) -> Result<Self, ProxyError> {
        let start_time = Instant::now();

        // 从数据库读取应用级代理配置（per-app）
        let app_config = state
            .db
            .get_proxy_config_for_app(app_type_str)
            .await
            .map_err(|e| ProxyError::DatabaseError(e.to_string()))?;

        // 从数据库读取整流器配置
        let rectifier_config = state.db.get_rectifier_config().unwrap_or_default();
        let optimizer_config = state.db.get_optimizer_config().unwrap_or_default();
        let copilot_optimizer_config = state.db.get_copilot_optimizer_config().unwrap_or_default();

        let current_provider_id =
            crate::settings::get_current_provider(&app_type).unwrap_or_default();

        // 从请求体提取模型名称
        let request_model = body
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();

        // 提取 Session ID
        let session_result = extract_session_id(headers, body, app_type_str);
        let session_id = session_result.session_id.clone();

        log::debug!(
            "[{}] Session ID: {} (from {:?}, client_provided: {})",
            tag,
            session_id,
            session_result.source,
            session_result.client_provided
        );

        // 使用共享的 ProviderRouter 选择 Provider（熔断器状态跨请求保持）
        // 注意：只在这里调用一次，结果传递给 forwarder，避免重复消耗 HalfOpen 名额
        //
        // Gemini（模型名在 URL 路径）的选路推迟到 with_model_from_uri 解析出
        // 真实模型后再执行初始过滤与可选的二次纠正；其余 app 直接按 body["model"] 过滤。
        let alias_model = if supports_model_alias(app_type_str)
            && !model_name_lives_in_body(app_type_str)
        {
            // Gemini：new 阶段尚未拿到 URI，此处不按模型过滤，交由 with_model_from_uri 纠正。
            None
        } else if supports_model_alias(app_type_str) {
            Some(request_model.as_str())
        } else {
            None
        };
        let providers = state
            .provider_router
            // 传入请求模型，让 model alias 能在选路阶段跳过不认识该模型的渠道。
            // 没有配 alias 时该方法等价于 select_providers（安全阀）。
            .select_providers_for_model(app_type_str, alias_model)
            .await
            .map_err(|e| match e {
                crate::error::AppError::AllProvidersCircuitOpen => {
                    ProxyError::AllProvidersCircuitOpen
                }
                crate::error::AppError::NoProvidersConfigured => ProxyError::NoProvidersConfigured,
                _ => ProxyError::DatabaseError(e.to_string()),
            })?;

        let provider = providers
            .first()
            .cloned()
            .ok_or(ProxyError::NoAvailableProvider)?;

        log::debug!(
            "[{}] Provider: {}, model: {}, failover chain: {} providers, session: {}",
            tag,
            provider.name,
            request_model,
            providers.len(),
            session_id
        );

        Ok(Self {
            start_time,
            app_config,
            provider,
            providers,
            current_provider_id,
            request_model,
            outbound_model: None,
            tag,
            app_type_str,
            app_type,
            session_id,
            session_client_provided: session_result.client_provided,
            rectifier_config,
            optimizer_config,
            copilot_optimizer_config,
        })
    }

    /// 从 URI 提取模型名称（Gemini 专用）
    ///
    /// Gemini API 的模型名称在 URI 中，格式如：
    /// `/v1beta/models/gemini-pro:generateContent`
    pub fn with_model_from_uri(mut self, uri: &axum::http::Uri) -> Self {
        // 用 path() 而不是 path_and_query()：模型名必须从路径段中解析，
        // 否则 GET /v1beta/models/<id>?key=... 会把 query 拼到 request_model 上。
        let endpoint = uri.path();

        self.request_model =
            extract_gemini_model_from_path(endpoint).unwrap_or_else(|| "unknown".to_string());

        self
    }

    /// Gemini 选路纠正：在 new 阶段尚未拿到 URI，此处用真实模型名重筛一次。
    ///
    /// 调用方需在 with_model_from_uri 之后、create_forwarder 之前调用；
    /// 失败时保持原链不变（安全阀），避免把可用链纠成空链。
    pub async fn correct_providers_for_gemini_model(
        &mut self,
        state: &ProxyState,
    ) -> Result<(), ProxyError> {
        if !supports_model_alias(self.app_type_str)
            || model_name_lives_in_body(self.app_type_str)
        {
            return Ok(());
        }
        if self.request_model.is_empty() || self.request_model == "unknown" {
            return Ok(());
        }
        let providers = state
            .provider_router
            .select_providers_for_model(self.app_type_str, Some(self.request_model.as_str()))
            .await
            .map_err(|e| match e {
                crate::error::AppError::AllProvidersCircuitOpen => {
                    ProxyError::AllProvidersCircuitOpen
                }
                crate::error::AppError::NoProvidersConfigured => ProxyError::NoProvidersConfigured,
                _ => ProxyError::DatabaseError(e.to_string()),
            })?;
        if providers.is_empty() {
            return Ok(());
        }
        // 若纠正后链为空（理论上 select 已做回退），保留原链。
        self.provider = providers.first().cloned().unwrap_or_else(|| self.provider.clone());
        self.providers = providers;
        Ok(())
    }

    /// 创建 RequestForwarder
    ///
    /// 使用共享的 ProviderRouter，确保熔断器状态跨请求保持
    ///
    /// 配置生效规则：
    /// - 故障转移开启：超时配置正常生效（0 表示禁用超时）
    /// - 故障转移关闭：超时配置不生效（全部传入 0）
    pub fn create_forwarder(&self, state: &ProxyState) -> RequestForwarder {
        let (non_streaming_timeout, first_byte_timeout, idle_timeout) =
            if self.app_config.auto_failover_enabled {
                // 故障转移开启：使用配置的值（0 = 禁用超时）
                (
                    self.app_config.non_streaming_timeout as u64,
                    self.app_config.streaming_first_byte_timeout as u64,
                    self.app_config.streaming_idle_timeout as u64,
                )
            } else {
                // 故障转移关闭：不启用超时配置
                log::debug!(
                    "[{}] Failover disabled, timeout configs are bypassed",
                    self.tag
                );
                (0, 0, 0)
            };

        // 故障转移关闭时强制 max_retries=0（仅尝试 1 个 provider），与「不超时 + 不切换」语义一致。
        let max_retries = if self.app_config.auto_failover_enabled {
            self.app_config.max_retries
        } else {
            0
        };

        // 读一次 model alias 索引并注入 forwarder：转发阶段按当前渠道解析物理名。
        // 读取失败或无配置时注入空索引，forwarder 会走原有映射链路（行为不变）。
        let model_alias_index = if supports_model_alias(self.app_type_str) {
            state
                .db
                .model_alias_index(self.app_type_str)
                .unwrap_or_else(|e| {
                    log::warn!(
                        "[{}] 读取 model alias 失败，本次请求不使用 alias: {e}",
                        self.tag
                    );
                    std::collections::HashMap::new()
                })
        } else {
            std::collections::HashMap::new()
        };

        RequestForwarder::new(
            state.provider_router.clone(),
            non_streaming_timeout,
            state.status.clone(),
            state.current_providers.clone(),
            state.gemini_shadow.clone(),
            state.codex_chat_history.clone(),
            state.failover_manager.clone(),
            state.app_handle.clone(),
            self.current_provider_id.clone(),
            self.session_id.clone(),
            self.session_client_provided,
            first_byte_timeout,
            idle_timeout,
            self.rectifier_config.clone(),
            self.optimizer_config.clone(),
            self.copilot_optimizer_config.clone(),
            max_retries,
            self.app_config.rate_limit_retry_enabled,
            self.app_config.rate_limit_max_wait_secs as u64,
        )
        .with_model_aliases(model_alias_index, self.app_type_str)
    }

    /// 获取 Provider 列表（用于故障转移）
    ///
    /// 返回在创建上下文时已选择的 providers，避免重复调用 select_providers()
    pub fn get_providers(&self) -> Vec<Provider> {
        self.providers.clone()
    }

    /// 计算请求延迟（毫秒）
    #[inline]
    pub fn latency_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// 获取流式超时配置
    ///
    /// 配置生效规则：
    /// - 故障转移开启：返回配置的值（0 表示禁用超时检查）
    /// - 故障转移关闭：返回 0（禁用超时检查）
    #[inline]
    pub fn streaming_timeout_config(&self) -> StreamingTimeoutConfig {
        if self.app_config.auto_failover_enabled {
            // 故障转移开启：使用配置的值（0 = 禁用超时）
            StreamingTimeoutConfig {
                first_byte_timeout: self.app_config.streaming_first_byte_timeout as u64,
                idle_timeout: self.app_config.streaming_idle_timeout as u64,
            }
        } else {
            // 故障转移关闭：禁用流式超时检查
            StreamingTimeoutConfig {
                first_byte_timeout: 0,
                idle_timeout: 0,
            }
        }
    }
}

/// Pull the Gemini model name out of an API path.
///
/// Accepts forms like `/v1beta/models/gemini-pro:generateContent`,
/// `/v1/models/gemini-1.5-flash`, `gemini/v1beta/models/<model>:streamGenerateContent`.
/// Returns `None` when no `models/<name>` segment is present.
pub(crate) fn extract_gemini_model_from_path(endpoint: &str) -> Option<String> {
    let segments: Vec<&str> = endpoint.split('/').collect();
    segments
        .iter()
        .position(|s| *s == "models")
        .and_then(|i| segments.get(i + 1).copied())
        // 防御性裁剪：即便调用方传入带 ? 或 :action 的字符串，也只保留 model id 本身
        .map(|s| s.split('?').next().unwrap_or(s))
        .map(|s| s.split(':').next().unwrap_or(s))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Replace the Gemini model id in a path with `new_model`, preserving `:action` and `?query`.
///
/// Returns `None` when the path has no `models/<id>` segment (e.g. list endpoint).
pub(crate) fn rewrite_gemini_model_in_path(endpoint: &str, new_model: &str) -> Option<String> {
    let trimmed = new_model.trim();
    if trimmed.is_empty() {
        return None;
    }
    let q_pos = endpoint.find('?');
    let (path, query) = match q_pos {
        Some(pos) => (&endpoint[..pos], Some(&endpoint[pos..])),
        None => (endpoint, None),
    };
    let colon_pos = path.rfind(':');
    let (path_no_action, action) = match colon_pos {
        Some(pos) if path[pos + 1..].chars().all(|c| c.is_ascii_alphabetic()) => {
            (&path[..pos], Some(&path[pos..]))
        }
        _ => (path, None),
    };
    let models_tag = "/models/";
    let idx = path_no_action.find(models_tag)?;
    let after_models = idx + models_tag.len();
    if after_models >= path_no_action.len() {
        return None;
    }
    // 模型段到下一个 '/' 为止（路径中不应再有 '/'，但防御性处理）
    let model_end = path_no_action[after_models..]
        .find('/')
        .map(|off| after_models + off)
        .unwrap_or(path_no_action.len());
    let mut out = String::with_capacity(path.len() + trimmed.len() + 8);
    out.push_str(&path_no_action[..after_models]);
    out.push_str(trimmed);
    out.push_str(&path_no_action[model_end..]);
    if let Some(action) = action {
        out.push_str(action);
    }
    if let Some(query) = query {
        out.push_str(query);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{
        extract_gemini_model_from_path, model_name_lives_in_body, rewrite_gemini_model_in_path,
        supports_model_alias,
    };

    #[test]
    fn gemini_is_excluded_from_body_based_model_routing() {
        // Gemini 的模型名在 URL 路径里：改 body 不生效，所以必须排除在
        // alias 路由之外，否则会「筛掉渠道却不换名」。
        assert!(!model_name_lives_in_body("gemini"));
        assert!(model_name_lives_in_body("claude"));
        assert!(model_name_lives_in_body("codex"));
        assert!(model_name_lives_in_body("grokbuild"));
        // claude-desktop 走独立的 map_proxy_request_model，但模型名也在 body 里
        assert!(model_name_lives_in_body("claude-desktop"));
    }

    #[test]
    fn gemini_supports_alias_via_path_rewrite() {
        assert!(supports_model_alias("gemini"));
        assert!(supports_model_alias("claude"));
        assert!(!supports_model_alias("claude-desktop"));
    }

    #[test]
    fn rewrite_preserves_action_and_query() {
        assert_eq!(
            rewrite_gemini_model_in_path(
                "/v1beta/models/gemini-pro:generateContent?key=abc",
                "gemini-2.0-flash"
            )
            .as_deref(),
            Some("/v1beta/models/gemini-2.0-flash:generateContent?key=abc")
        );
    }

    #[test]
    fn rewrite_without_action() {
        assert_eq!(
            rewrite_gemini_model_in_path("/v1/models/gemini-1.5-pro", "gemini-pro").as_deref(),
            Some("/v1/models/gemini-pro")
        );
    }

    #[test]
    fn rewrite_with_proxy_prefix() {
        assert_eq!(
            rewrite_gemini_model_in_path(
                "/gemini/v1beta/models/gemini-1.0-pro:countTokens",
                "x"
            )
            .as_deref(),
            Some("/gemini/v1beta/models/x:countTokens")
        );
    }

    #[test]
    fn rewrite_returns_none_on_list_endpoint() {
        assert_eq!(
            rewrite_gemini_model_in_path("/v1beta/models", "x"),
            None
        );
    }

    #[test]
    fn extract_model_with_action() {
        assert_eq!(
            extract_gemini_model_from_path("/v1beta/models/gemini-pro:generateContent").as_deref(),
            Some("gemini-pro"),
        );
    }

    #[test]
    fn extract_model_with_dotted_version() {
        assert_eq!(
            extract_gemini_model_from_path("/v1beta/models/gemini-1.5-flash:streamGenerateContent")
                .as_deref(),
            Some("gemini-1.5-flash"),
        );
    }

    #[test]
    fn extract_model_without_action() {
        assert_eq!(
            extract_gemini_model_from_path("/v1/models/gemini-1.5-pro").as_deref(),
            Some("gemini-1.5-pro"),
        );
    }

    #[test]
    fn extract_model_with_proxy_prefix() {
        assert_eq!(
            extract_gemini_model_from_path("/gemini/v1beta/models/gemini-2.0-flash:countTokens")
                .as_deref(),
            Some("gemini-2.0-flash"),
        );
    }

    #[test]
    fn extract_model_with_query_string() {
        assert_eq!(
            extract_gemini_model_from_path("/v1beta/models/gemini-pro:generateContent?key=abc")
                .as_deref(),
            Some("gemini-pro"),
        );
    }

    #[test]
    fn extract_model_missing_segment() {
        assert_eq!(extract_gemini_model_from_path("/v1beta/operations"), None);
    }

    #[test]
    fn extract_model_trailing_models_segment() {
        // `/v1beta/models` (list endpoint) has no following segment → None.
        assert_eq!(extract_gemini_model_from_path("/v1beta/models"), None);
    }

    #[test]
    fn extract_model_get_with_query_only() {
        // GET /v1beta/models/<id>?key=... 无 action verb，仅靠 ':' 拆分会把 query 带进 model 名。
        // 修复后应该把 query 剥掉。
        assert_eq!(
            extract_gemini_model_from_path("/v1beta/models/gemini-pro?key=abc").as_deref(),
            Some("gemini-pro"),
        );
    }

    #[test]
    fn extract_model_get_with_proxy_prefix_and_query() {
        assert_eq!(
            extract_gemini_model_from_path("/gemini/v1beta/models/gemini-2.0-flash?key=abc")
                .as_deref(),
            Some("gemini-2.0-flash"),
        );
    }
}
