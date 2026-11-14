use serde::{Deserialize, Serialize};

/// 代理服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 监听地址
    pub listen_address: String,
    /// 监听端口
    pub listen_port: u16,
    /// 最大重试次数
    pub max_retries: u8,
    /// 请求超时时间（秒）- 已废弃，保留兼容
    pub request_timeout: u64,
    /// 是否启用日志
    pub enable_logging: bool,
    /// 是否正在接管 Live 配置
    #[serde(default)]
    pub live_takeover_active: bool,
    /// 流式首字超时（秒）- 等待首个数据块的最大时间，范围 1-120 秒，默认 60 秒
    #[serde(default = "default_streaming_first_byte_timeout")]
    pub streaming_first_byte_timeout: u64,
    /// 流式静默超时（秒）- 两个数据块之间的最大间隔，范围 60-600 秒，填 0 禁用（防止中途卡住）
    #[serde(default = "default_streaming_idle_timeout")]
    pub streaming_idle_timeout: u64,
    /// 非流式总超时（秒）- 非流式请求的总超时时间，范围 60-1200 秒，默认 600 秒（10 分钟）
    #[serde(default = "default_non_streaming_timeout")]
    pub non_streaming_timeout: u64,
}

fn default_streaming_first_byte_timeout() -> u64 {
    60
}

fn default_streaming_idle_timeout() -> u64 {
    120
}

fn default_non_streaming_timeout() -> u64 {
    600
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            listen_address: "127.0.0.1".to_string(),
            listen_port: crate::config::default_proxy_port(),
            max_retries: 3,
            request_timeout: 600,
            enable_logging: true,
            live_takeover_active: false,
            streaming_first_byte_timeout: 60,
            streaming_idle_timeout: 120,
            non_streaming_timeout: 600,
        }
    }
}

/// 代理服务器状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyStatus {
    /// 是否运行中
    pub running: bool,
    /// 监听地址
    pub address: String,
    /// 监听端口
    pub port: u16,
    /// 活跃连接数
    pub active_connections: usize,
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub success_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 成功率 (0-100)
    pub success_rate: f32,
    /// 运行时间（秒）
    pub uptime_seconds: u64,
    /// 当前使用的Provider名称
    pub current_provider: Option<String>,
    /// 当前Provider的ID
    pub current_provider_id: Option<String>,
    /// 最后一次请求时间
    pub last_request_at: Option<String>,
    /// 最后一次错误信息
    pub last_error: Option<String>,
    /// Provider故障转移次数
    pub failover_count: u64,
    /// 429 限流等待重试次数（本次代理运行期累计）
    ///
    /// 这是**全局**累计值，只适合在「代理服务总览」里展示。
    /// 按应用区分的统计请读 `rate_limit_waits_by_app` —— 否则 claude 的
    /// 限流等待会被算进 codex 的韧性摘要。
    #[serde(default)]
    pub rate_limit_wait_count: u64,
    /// 429 等待重试累计等待秒数
    #[serde(default)]
    pub rate_limit_wait_total_secs: u64,
    /// 按应用类型分桶的 429 等待统计（本次代理运行期累计）
    ///
    /// key = app_type（"claude" / "codex" / ...），value = (次数, 累计等待秒数)。
    /// 空 map 表示本次运行期尚未发生任何 429 等待。
    #[serde(default)]
    pub rate_limit_waits_by_app: std::collections::HashMap<String, AppRateLimitWaits>,
    /// 当前是否有请求正在 429 等待循环中（本次轮询快照）。
    ///
    /// Shows the most recent live request wait. A concurrent success does not
    /// clear another request's entry; cancellation removes only its owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit_waiting: Option<RateLimitWaiting>,
    /// Request-owned waits; the public snapshot shows the most recent live wait.
    /// Runtime ownership must never be persisted or sent to the frontend.
    #[serde(skip)]
    pub(crate) active_rate_limit_waits: std::collections::BTreeMap<u64, RateLimitWaiting>,
    /// 当前活跃的代理目标列表
    #[serde(default)]
    pub active_targets: Vec<ActiveTarget>,
}

/// 快照：当前是否有一个请求正处在 429 等待重试循环中。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitWaiting {
    /// 正在等待的 provider 展示名
    pub provider_name: String,
    /// 正在等待的 provider id
    pub provider_id: String,
    /// 所属应用类型
    pub app_type: String,
    /// 本次等待的秒数（Retry-After 或退避值）
    pub wait_secs: u64,
    /// 本轮 429 的第几次等待（1 起）
    pub attempt: u32,
    /// 快照写入时的 ISO8601 时间（供前端做倒计时起点）
    pub started_at: String,
}

/// 单个应用的 429 等待统计
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRateLimitWaits {
    /// 429 等待重试次数
    pub count: u64,
    /// 累计等待秒数
    pub total_secs: u64,
}

impl ProxyStatus {
    pub(crate) fn begin_rate_limit_wait(&mut self, id: u64, waiting: RateLimitWaiting) {
        self.record_rate_limit_wait(&waiting.app_type, waiting.wait_secs);
        self.active_rate_limit_waits.insert(id, waiting);
        self.refresh_rate_limit_waiting();
    }

    pub(crate) fn end_rate_limit_wait(&mut self, id: u64) {
        self.active_rate_limit_waits.remove(&id);
        self.refresh_rate_limit_waiting();
    }

    /// A shared cooldown extension is still the same wait, not another retry.
    pub(crate) fn extend_rate_limit_wait(&mut self, id: u64, total_secs: u64) {
        if let Some(waiting) = self.active_rate_limit_waits.get_mut(&id) {
            let extra = total_secs.saturating_sub(waiting.wait_secs);
            if extra == 0 {
                return;
            }
            waiting.wait_secs = total_secs;
            self.rate_limit_wait_total_secs = self.rate_limit_wait_total_secs.saturating_add(extra);
            let bucket = self
                .rate_limit_waits_by_app
                .entry(waiting.app_type.clone())
                .or_default();
            bucket.total_secs = bucket.total_secs.saturating_add(extra);
            self.refresh_rate_limit_waiting();
        }
    }

    fn refresh_rate_limit_waiting(&mut self) {
        self.rate_limit_waiting = self
            .active_rate_limit_waits
            .last_key_value()
            .map(|(_, waiting)| waiting.clone());
    }

    /// 读取指定应用的 429 等待统计；无记录时返回全零。
    pub fn rate_limit_waits_for(&self, app_type: &str) -> AppRateLimitWaits {
        self.rate_limit_waits_by_app
            .get(app_type)
            .copied()
            .unwrap_or_default()
    }

    /// 累加一次 429 等待：同时更新全局累计与 per-app 分桶。
    ///
    /// 两处都更新是为了让「总览」和「按应用」两个视图保持一致，
    /// 避免调用方各自维护一份计数。
    pub fn record_rate_limit_wait(&mut self, app_type: &str, wait_secs: u64) {
        self.rate_limit_wait_count = self.rate_limit_wait_count.saturating_add(1);
        self.rate_limit_wait_total_secs = self.rate_limit_wait_total_secs.saturating_add(wait_secs);

        let bucket = self
            .rate_limit_waits_by_app
            .entry(app_type.to_string())
            .or_default();
        bucket.count = bucket.count.saturating_add(1);
        bucket.total_secs = bucket.total_secs.saturating_add(wait_secs);
    }
}

/// 活跃的代理目标信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTarget {
    pub app_type: String, // "Claude" | "Codex" | "Gemini"
    pub provider_name: String,
    pub provider_id: String,
}

/// 代理服务器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyServerInfo {
    pub address: String,
    pub port: u16,
    pub started_at: String,
}

/// 各应用的接管状态（是否改写该应用的 Live 配置指向本地代理）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyTakeoverStatus {
    pub claude: bool,
    pub codex: bool,
    pub gemini: bool,
    pub grokbuild: bool,
    pub opencode: bool,
    pub openclaw: bool,
}

/// Provider健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider_id: String,
    pub app_type: String,
    pub is_healthy: bool,
    pub consecutive_failures: u32,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_error: Option<String>,
    pub updated_at: String,
    /// 熔断器当前状态（`closed` / `open` / `half_open`）。
    ///
    /// 仅当代理正在运行且该 provider 已产生过流量时有值 —— 熔断器是纯内存态，
    /// 数据库里的 `provider_health` 只记录持久化的失败计数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circuit_state: Option<String>,
    /// 熔断自动恢复探测的剩余秒数（0 = 无需等待或未熔断）
    #[serde(default)]
    pub recovery_remaining_secs: u64,
    /// 429 限流冷却的剩余秒数（0 = 未冷却）
    #[serde(default)]
    pub rate_limit_cooldown_secs: u64,
    /// 本次熔断是否由额度耗尽（致命失败）触发
    #[serde(default)]
    pub fatal: bool,
    /// 半开状态下已占用的探测名额。
    ///
    /// 半开只允许 1 个探测请求（见 `allow_half_open_probe`）。若某条请求路径
    /// 忘了在结束时释放名额，该 Provider 会永久卡在「半开但拒绝所有请求」——
    /// 熔断器状态显示 half_open、恢复倒计时归零、却永远不放行。这个字段是
    /// 唯一能在 UI 上观测到该泄漏的信号。
    #[serde(default)]
    pub half_open_probes: u32,
}

/// Live 配置备份记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveBackup {
    /// 应用类型 (claude/codex/gemini)
    pub app_type: String,
    /// 原始配置 JSON
    pub original_config: String,
    /// 备份时间
    pub backed_up_at: String,
}

/// 全局代理配置（统一字段，三行镜像）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalProxyConfig {
    /// 代理总开关
    pub proxy_enabled: bool,
    /// 监听地址
    pub listen_address: String,
    /// 监听端口
    pub listen_port: u16,
    /// 是否启用日志
    pub enable_logging: bool,
}

/// 应用级代理配置（每个 app 独立）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppProxyConfig {
    /// 应用类型 (claude/codex/gemini)
    pub app_type: String,
    /// 该 app 代理启用开关
    pub enabled: bool,
    /// 该 app 自动故障转移开关
    pub auto_failover_enabled: bool,
    /// 最大重试次数
    pub max_retries: u32,
    /// 流式首字超时（秒）
    pub streaming_first_byte_timeout: u32,
    /// 流式静默超时（秒）
    pub streaming_idle_timeout: u32,
    /// 非流式总超时（秒）
    pub non_streaming_timeout: u32,
    /// 熔断失败阈值
    pub circuit_failure_threshold: u32,
    /// 熔断恢复阈值
    pub circuit_success_threshold: u32,
    /// 熔断恢复等待时间（秒）
    pub circuit_timeout_seconds: u32,
    /// 错误率阈值
    pub circuit_error_rate_threshold: f64,
    /// 计算错误率的最小请求数
    pub circuit_min_requests: u32,
    /// 429 限流等待重试开关：开启后 429 时等待 Retry-After（或指数退避）后重试同一 Provider
    #[serde(default)]
    pub rate_limit_retry_enabled: bool,
    /// 429 等待重试的最长等待时间（秒），超过则放弃等待、切换下一家 Provider
    #[serde(default = "default_rate_limit_max_wait_secs")]
    pub rate_limit_max_wait_secs: u32,
    /// 同一 Provider 在「首字节返回前」失败时的额外重试次数。
    ///
    /// 与 `max_retries` 正交：`max_retries` 数的是「最多换几家 Provider」，
    /// 本字段数的是「同一家再多试几次」。0 = 不重试（默认，与升级前行为一致）。
    /// 重试不消耗 `attempted_providers` 配额，也不写熔断器健康度——前者是
    /// 「换家」的预算，后者衡量 Provider 健康，一次成功前的内部重试不算多次失败。
    #[serde(default = "default_same_provider_retries")]
    pub same_provider_retries: u32,
    /// 同一 Provider 重试的退避基数（毫秒）：第 n 次等待 base * 2^(n-1)，受上限约束
    #[serde(default = "default_same_provider_retry_backoff_ms")]
    pub same_provider_retry_backoff_ms: u32,
    /// 同一 Provider 重试的退避上限（毫秒）
    #[serde(default = "default_same_provider_retry_max_backoff_ms")]
    pub same_provider_retry_max_backoff_ms: u32,
}

fn default_rate_limit_max_wait_secs() -> u32 {
    120
}

/// 默认不重试：升级前后行为完全一致，需要时由用户在面板里显式开启。
fn default_same_provider_retries() -> u32 {
    0
}

fn default_same_provider_retry_backoff_ms() -> u32 {
    1000
}

fn default_same_provider_retry_max_backoff_ms() -> u32 {
    8000
}

/// 整流器配置
///
/// 存储在 settings 表中
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RectifierConfig {
    /// 总开关：是否启用整流器（默认开启）
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 请求整流：启用 thinking 签名整流器（默认开启）
    ///
    /// 处理错误：Invalid 'signature' in 'thinking' block
    #[serde(default = "default_true")]
    pub request_thinking_signature: bool,
    /// 请求整流：启用 thinking budget 整流器（默认开启）
    ///
    /// 处理错误：budget_tokens + thinking 相关约束
    #[serde(default = "default_true")]
    pub request_thinking_budget: bool,
    /// 请求整流：不支持的图片降级（默认开启）
    ///
    /// 上游拒绝图片输入时，把图片块替换为 [Unsupported Image] 标记，
    /// 让对话不中断。总开关，管辖「显式声明 text-only」与「上游报错后兜底」两条事实驱动路径。
    #[serde(default = "default_true")]
    pub request_media_fallback: bool,
    /// 请求整流：确认纯文本注册表的发送前降级（默认开启）
    ///
    /// 在模型未声明能力时，按内置的确认纯文本注册表预先剥离图片。
    /// 受 request_media_fallback 管辖；单独关闭只停用代理的注册表预判，
    /// 仍保留「显式声明」与「上游兜底」，且不改变 Codex 模型目录声明。
    #[serde(default = "default_true")]
    pub request_media_heuristic: bool,
    /// Optional history portability repair for API-key Codex Responses providers.
    /// Disabled by default: dropping reasoning ciphertext can change context.
    #[serde(default)]
    pub codex_responses_compatibility: bool,
}

fn default_true() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for RectifierConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            request_thinking_signature: true,
            request_thinking_budget: true,
            request_media_fallback: true,
            request_media_heuristic: true,
            codex_responses_compatibility: false,
        }
    }
}

/// 请求优化器配置
///
/// 存储在 settings 表中，key = "optimizer_config"
/// 仅对 Bedrock provider 生效（CLAUDE_CODE_USE_BEDROCK = "1"）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizerConfig {
    /// 总开关（默认关闭，用户需手动启用）
    #[serde(default)]
    pub enabled: bool,
    /// Thinking 优化子开关（总开关开启后默认生效）
    #[serde(default = "default_true")]
    pub thinking_optimizer: bool,
    /// Cache 注入子开关（总开关开启后默认生效）
    #[serde(default = "default_true")]
    pub cache_injection: bool,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            thinking_optimizer: true,
            cache_injection: true,
        }
    }
}

/// Copilot 优化器配置
///
/// 存储在 settings 表中，key = "copilot_optimizer_config"
/// 解决 Copilot 代理消耗量异常问题（Issue #1813）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotOptimizerConfig {
    /// 总开关（默认开启 — 对 Copilot 用户至关重要）
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// x-initiator 请求分类（默认开启，P0 优先级）
    #[serde(default = "default_true")]
    pub request_classification: bool,
    /// Tool result 消息合并（默认开启，P1 优先级）
    #[serde(default = "default_true")]
    pub tool_result_merging: bool,
    /// Compact 请求识别（默认开启，P2 优先级）
    #[serde(default = "default_true")]
    pub compact_detection: bool,
    /// 确定性 Request ID（默认开启，P3 优先级）
    #[serde(default = "default_true")]
    pub deterministic_request_id: bool,
    /// Subagent 检测（默认开启）— 识别 Claude Code 子代理请求，
    /// 设置 x-initiator=agent + x-interaction-type=conversation-subagent，避免子代理计费
    #[serde(default = "default_true")]
    pub subagent_detection: bool,
    /// Warmup 小模型降级（默认开启 — 与参考实现对齐，避免探针请求消耗 premium quota）
    #[serde(default = "default_true")]
    pub warmup_downgrade: bool,
    /// Warmup 降级使用的模型（默认 "gpt-5-mini"）
    #[serde(default = "default_warmup_model")]
    pub warmup_model: String,
    /// 请求前主动剥离 assistant 消息里的 thinking / redacted_thinking block
    ///
    /// Copilot 走 OpenAI 兼容端点，thinking block 会被上游拒绝并触发 rectifier 反应式
    /// 重试，那时第一次请求已经消耗了一次 premium quota。主动剥离避免这次浪费。
    #[serde(default = "default_true")]
    pub strip_thinking: bool,
}

fn default_warmup_model() -> String {
    "gpt-5-mini".to_string()
}

impl Default for CopilotOptimizerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            request_classification: true,
            tool_result_merging: true,
            compact_detection: true,
            deterministic_request_id: true,
            subagent_detection: true,
            warmup_downgrade: true,
            warmup_model: "gpt-5-mini".to_string(),
            strip_thinking: true,
        }
    }
}

/// 日志配置
///
/// 存储在 settings 表的 log_config 字段中（JSON 格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogConfig {
    /// 总开关：是否启用日志
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 日志级别: error, warn, info, debug, trace
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "info".to_string(),
        }
    }
}

impl LogConfig {
    /// 将配置转换为 log::LevelFilter
    pub fn to_level_filter(&self) -> log::LevelFilter {
        if !self.enabled {
            return log::LevelFilter::Off;
        }
        match self.level.to_lowercase().as_str() {
            "error" => log::LevelFilter::Error,
            "warn" => log::LevelFilter::Warn,
            "info" => log::LevelFilter::Info,
            "debug" => log::LevelFilter::Debug,
            "trace" => log::LevelFilter::Trace,
            _ => log::LevelFilter::Info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_wait_extension_adds_duration_without_counting_another_retry() {
        let mut status = ProxyStatus::default();
        status.begin_rate_limit_wait(
            1,
            RateLimitWaiting {
                provider_id: "a".into(),
                provider_name: "A".into(),
                app_type: "codex".into(),
                wait_secs: 1,
                attempt: 1,
                started_at: "2026-11-14T00:00:00Z".into(),
            },
        );
        status.extend_rate_limit_wait(1, 3);
        status.extend_rate_limit_wait(1, 2);
        assert_eq!(status.rate_limit_wait_count, 1);
        assert_eq!(status.rate_limit_wait_total_secs, 3);
        assert_eq!(status.rate_limit_waits_for("codex").count, 1);
        assert_eq!(status.rate_limit_waits_for("codex").total_secs, 3);
        assert_eq!(status.rate_limit_waiting.as_ref().unwrap().wait_secs, 3);
        status.end_rate_limit_wait(1);
        status.extend_rate_limit_wait(1, 10);
        assert_eq!(status.rate_limit_wait_total_secs, 3);
        assert!(status.rate_limit_waiting.is_none());
    }

    #[test]
    fn concurrent_waits_restore_the_previous_live_snapshot_without_stale_errors() {
        let mut status = ProxyStatus::default();
        let waiting = |provider: &str| RateLimitWaiting {
            provider_id: provider.into(),
            provider_name: provider.into(),
            app_type: "codex".into(),
            wait_secs: 2,
            attempt: 1,
            started_at: "2026-11-13T00:00:00Z".into(),
        };
        status.begin_rate_limit_wait(1, waiting("a"));
        status.begin_rate_limit_wait(2, waiting("b"));
        assert_eq!(status.rate_limit_waiting.as_ref().unwrap().provider_id, "b");
        status.end_rate_limit_wait(2);
        assert_eq!(status.rate_limit_waiting.as_ref().unwrap().provider_id, "a");
        status.end_rate_limit_wait(2); // Duplicate cleanup must not affect a.
        assert_eq!(status.rate_limit_waiting.as_ref().unwrap().provider_id, "a");
        assert_eq!(status.rate_limit_waits_for("codex").count, 2);
        assert!(status.last_error.is_none());
        let serialized = serde_json::to_value(&status).unwrap();
        assert!(serialized.get("active_rate_limit_waits").is_none());
        status.end_rate_limit_wait(1);
        assert!(status.rate_limit_waiting.is_none());
    }

    /// 429 等待必须按 app 分桶：韧性摘要按应用展示，只记全局会让
    /// claude 的限流等待出现在 codex 的卡片上。
    #[test]
    fn rate_limit_waits_are_bucketed_per_app() {
        let mut status = ProxyStatus::default();

        status.record_rate_limit_wait("claude", 30);
        status.record_rate_limit_wait("claude", 10);
        status.record_rate_limit_wait("codex", 5);

        let claude = status.rate_limit_waits_for("claude");
        assert_eq!(claude.count, 2);
        assert_eq!(claude.total_secs, 40);

        let codex = status.rate_limit_waits_for("codex");
        assert_eq!(codex.count, 1);
        assert_eq!(codex.total_secs, 5);

        // 未发生限流的应用读出来是全零，而不是别的应用的数字
        let gemini = status.rate_limit_waits_for("gemini");
        assert_eq!(gemini.count, 0);
        assert_eq!(gemini.total_secs, 0);
    }

    /// 全局累计与 per-app 分桶必须同时更新且彼此自洽。
    #[test]
    fn global_and_per_app_rate_limit_totals_stay_consistent() {
        let mut status = ProxyStatus::default();

        status.record_rate_limit_wait("claude", 30);
        status.record_rate_limit_wait("codex", 20);

        // 全局 = 各应用之和
        assert_eq!(status.rate_limit_wait_count, 2);
        assert_eq!(status.rate_limit_wait_total_secs, 50);

        let sum_count: u64 = status
            .rate_limit_waits_by_app
            .values()
            .map(|waits| waits.count)
            .sum();
        let sum_secs: u64 = status
            .rate_limit_waits_by_app
            .values()
            .map(|waits| waits.total_secs)
            .sum();
        assert_eq!(sum_count, status.rate_limit_wait_count);
        assert_eq!(sum_secs, status.rate_limit_wait_total_secs);
    }

    #[test]
    fn test_rectifier_config_default_enabled() {
        // General rectifiers stay enabled; lossy Codex history repair is opt-in.
        let config = RectifierConfig::default();
        assert!(!config.codex_responses_compatibility);
        assert!(config.enabled, "整流器总开关默认应为 true");
        assert!(
            config.request_thinking_signature,
            "thinking 签名整流器默认应为 true"
        );
        assert!(
            config.request_thinking_budget,
            "thinking budget 整流器默认应为 true"
        );
        assert!(
            config.request_media_fallback,
            "media 降级总开关默认应为 true"
        );
        assert!(
            config.request_media_heuristic,
            "启发式 text-only 模型识别默认应为 true"
        );
    }

    #[test]
    fn test_rectifier_config_serde_default() {
        // 验证反序列化缺字段时使用默认值 true
        let json = "{}";
        let config: RectifierConfig = serde_json::from_str(json).unwrap();
        assert!(!config.codex_responses_compatibility);
        assert!(config.enabled);
        assert!(config.request_thinking_signature);
        assert!(config.request_thinking_budget);
        assert!(
            config.request_media_fallback,
            "缺 requestMediaFallback 时应回退默认值 true"
        );
        assert!(
            config.request_media_heuristic,
            "缺 requestMediaHeuristic 时应回退默认值 true"
        );
    }

    #[test]
    fn test_rectifier_config_serde_explicit_true() {
        // 验证显式设置 true 时正确反序列化
        let json =
            r#"{"enabled": true, "requestThinkingSignature": true, "requestThinkingBudget": true}"#;
        let config: RectifierConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert!(config.request_thinking_signature);
        assert!(config.request_thinking_budget);
    }

    #[test]
    fn test_rectifier_config_serde_partial_fields() {
        // 验证只设置部分字段时，缺失字段使用默认值 true
        let json = r#"{"enabled": true, "requestThinkingSignature": false}"#;
        let config: RectifierConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert!(!config.request_thinking_signature);
        assert!(config.request_thinking_budget);
    }

    #[test]
    fn test_rectifier_config_serde_media_explicit_false() {
        // 验证 media 两字段显式 false 时被如实反序列化（用户主动关闭须生效，不能被默认值覆盖）
        let json = r#"{"requestMediaFallback": false, "requestMediaHeuristic": false}"#;
        let config: RectifierConfig = serde_json::from_str(json).unwrap();
        assert!(!config.request_media_fallback);
        assert!(!config.request_media_heuristic);
        // 其余字段仍走默认 true
        assert!(config.enabled);
        assert!(config.request_thinking_signature);
        assert!(config.request_thinking_budget);
    }

    #[test]
    fn codex_responses_compatibility_setting_round_trips_without_changing_other_rectifiers() {
        let config: RectifierConfig = serde_json::from_str(
            r#"{"codexResponsesCompatibility":true,"requestThinkingSignature":false}"#,
        )
        .unwrap();
        assert!(config.codex_responses_compatibility);
        assert!(!config.request_thinking_signature);
        assert!(config.request_thinking_budget);
        let saved = serde_json::to_value(config).unwrap();
        assert_eq!(saved["codexResponsesCompatibility"], true);
        assert_eq!(saved["requestThinkingSignature"], false);
    }

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert!(config.enabled);
        assert_eq!(config.level, "info");
    }

    #[test]
    fn test_log_config_serde_default() {
        let json = "{}";
        let config: LogConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.level, "info");
    }

    #[test]
    fn test_log_config_to_level_filter() {
        let config = LogConfig {
            level: "error".to_string(),
            ..Default::default()
        };
        assert_eq!(config.to_level_filter(), log::LevelFilter::Error);

        let config = LogConfig {
            level: "warn".to_string(),
            ..Default::default()
        };
        assert_eq!(config.to_level_filter(), log::LevelFilter::Warn);

        let config = LogConfig {
            level: "info".to_string(),
            ..Default::default()
        };
        assert_eq!(config.to_level_filter(), log::LevelFilter::Info);

        let config = LogConfig {
            level: "debug".to_string(),
            ..Default::default()
        };
        assert_eq!(config.to_level_filter(), log::LevelFilter::Debug);

        let config = LogConfig {
            level: "trace".to_string(),
            ..Default::default()
        };
        assert_eq!(config.to_level_filter(), log::LevelFilter::Trace);

        // 无效级别回退到 info
        let config = LogConfig {
            level: "invalid".to_string(),
            ..Default::default()
        };
        assert_eq!(config.to_level_filter(), log::LevelFilter::Info);

        // 禁用时返回 Off
        let config = LogConfig {
            enabled: false,
            level: "debug".to_string(),
        };
        assert_eq!(config.to_level_filter(), log::LevelFilter::Off);
    }

    #[test]
    fn test_log_config_serde_roundtrip() {
        let config = LogConfig {
            enabled: true,
            level: "debug".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: LogConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.level, "debug");
    }
}
