//! 熔断器模块
//!
//! 实现熔断器模式，用于防止向不健康的供应商发送请求

use super::log_codes::cb as log_cb;
use super::types::AppProxyConfig;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    /// 关闭状态 - 正常工作
    Closed,
    /// 打开状态 - 熔断激活，拒绝请求
    Open,
    /// 半开状态 - 尝试恢复，允许部分请求通过
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half_open"),
        }
    }
}

/// 熔断器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerConfig {
    /// 失败阈值 - 连续失败多少次后打开熔断器
    pub failure_threshold: u32,
    /// 成功阈值 - 半开状态下成功多少次后关闭熔断器
    pub success_threshold: u32,
    /// 超时时间 - 熔断器打开后多久尝试半开（秒）
    pub timeout_seconds: u64,
    /// 错误率阈值 - 错误率超过此值时打开熔断器 (0.0-1.0)
    pub error_rate_threshold: f64,
    /// 最小请求数 - 计算错误率前的最小请求数
    pub min_requests: u32,
}

impl From<&AppProxyConfig> for CircuitBreakerConfig {
    fn from(config: &AppProxyConfig) -> Self {
        Self {
            failure_threshold: config.circuit_failure_threshold,
            success_threshold: config.circuit_success_threshold,
            timeout_seconds: config.circuit_timeout_seconds as u64,
            error_rate_threshold: config.circuit_error_rate_threshold,
            min_requests: config.circuit_min_requests,
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 4,
            success_threshold: 2,
            timeout_seconds: 60,
            error_rate_threshold: 0.6,
            min_requests: 10,
        }
    }
}

/// 熔断器实例
pub struct CircuitBreaker {
    /// 当前状态
    state: Arc<RwLock<CircuitState>>,
    /// 连续失败计数
    consecutive_failures: Arc<AtomicU32>,
    /// 连续成功计数（半开状态）
    consecutive_successes: Arc<AtomicU32>,
    /// 总请求计数
    total_requests: Arc<AtomicU32>,
    /// 失败请求计数
    failed_requests: Arc<AtomicU32>,
    /// 上次打开时间
    last_opened_at: Arc<RwLock<Option<Instant>>>,
    /// 配置（支持热更新）
    config: Arc<RwLock<CircuitBreakerConfig>>,
    /// The active probe's unique ticket; zero means no probe is in flight.
    half_open_ticket: Arc<AtomicU64>,
    next_probe_ticket: AtomicU64,
    /// 最近一次打开是否由致命失败（额度耗尽等）触发。
    ///
    /// 单 Provider 场景下熔断器被 bypass（避免唯一通道被永久阻断），但额度
    /// 耗尽属于终态：恢复窗口内重复打上游只会再拿 403。该标记用于在 bypass
    /// 路径下仍能快速失败并给出明确错误。
    fatal_open: Arc<std::sync::atomic::AtomicBool>,
}

/// 熔断器放行结果
///
/// Owns its probe slot, including when the request future is cancelled.
#[derive(Debug)]
pub struct AllowResult {
    pub allowed: bool,
    pub used_half_open_permit: bool,
    probe: Option<HalfOpenPermit>,
}

#[derive(Debug)]
struct HalfOpenPermit {
    slot: Arc<AtomicU64>,
    ticket: u64,
}

impl Drop for HalfOpenPermit {
    fn drop(&mut self) {
        // A reset or a later probe changes the ticket. An old request must not
        // release a new request's slot, even when dropped after a manual reset.
        let _ = self
            .slot
            .compare_exchange(self.ticket, 0, Ordering::SeqCst, Ordering::SeqCst);
    }
}

impl AllowResult {
    pub(crate) fn without_probe(allowed: bool) -> Self {
        Self {
            allowed,
            used_half_open_permit: false,
            probe: None,
        }
    }

    /// Release neutrally, without recording any upstream success or failure.
    pub(crate) fn release(&mut self) {
        self.probe.take();
        self.used_half_open_permit = false;
    }
}

impl CircuitBreaker {
    /// 创建新的熔断器
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            consecutive_failures: Arc::new(AtomicU32::new(0)),
            consecutive_successes: Arc::new(AtomicU32::new(0)),
            total_requests: Arc::new(AtomicU32::new(0)),
            failed_requests: Arc::new(AtomicU32::new(0)),
            last_opened_at: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(config)),
            half_open_ticket: Arc::new(AtomicU64::new(0)),
            next_probe_ticket: AtomicU64::new(1),
            fatal_open: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 更新熔断器配置（热更新，不重置状态）
    pub async fn update_config(&self, new_config: CircuitBreakerConfig) {
        *self.config.write().await = new_config;
    }

    /// 判断当前 Provider 是否“可被纳入候选链路”
    ///
    /// 这个方法不会占用 HalfOpen 探测名额，仅用于路由选择阶段的“可用性判断”：
    /// - Closed / HalfOpen：可用（返回 true）
    /// - Open：若超时到达则切到 HalfOpen 并返回 true，否则返回 false
    ///
    /// 注意：真正发起请求前仍需调用 `allow_request()` 来获取 HalfOpen 探测名额，
    /// 并在请求结束后通过 `record_success()` / `record_failure()` 释放。
    pub async fn is_available(&self) -> bool {
        let state = *self.state.read().await;
        let config = self.config.read().await;

        match state {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => {
                let opened_at = *self.last_opened_at.read().await;
                if let Some(opened_at) = opened_at {
                    if opened_at.elapsed().as_secs() >= config.timeout_seconds {
                        drop(config); // 释放读锁再转换状态
                        log::info!(
                            "[{}] 熔断器 Open → HalfOpen (超时恢复)",
                            log_cb::OPEN_TO_HALF_OPEN
                        );
                        self.transition_to_half_open().await;
                        return true;
                    }
                }
                false
            }
        }
    }

    /// 检查是否允许请求通过
    pub async fn allow_request(&self) -> AllowResult {
        let state = self.state.read().await;

        match *state {
            CircuitState::Closed => AllowResult::without_probe(true),
            CircuitState::Open => {
                drop(state);
                let config = self.config.read().await;
                // 检查是否应该尝试半开
                let opened_at = *self.last_opened_at.read().await;
                if let Some(opened_at) = opened_at {
                    if opened_at.elapsed().as_secs() >= config.timeout_seconds {
                        drop(config); // 释放读锁再转换状态
                        log::info!(
                            "[{}] 熔断器 Open → HalfOpen (超时恢复)",
                            log_cb::OPEN_TO_HALF_OPEN
                        );
                        self.transition_to_half_open().await;

                        // 转换后按当前状态决定是否需要获取 HalfOpen 探测名额
                        let current_state = self.state.read().await;
                        return match *current_state {
                            CircuitState::Closed => AllowResult::without_probe(true),
                            CircuitState::HalfOpen => self.allow_half_open_probe(),
                            CircuitState::Open => AllowResult::without_probe(false),
                        };
                    }
                }

                AllowResult::without_probe(false)
            }
            CircuitState::HalfOpen => self.allow_half_open_probe(),
        }
    }

    /// 记录成功
    pub async fn record_success(&self, permit: Option<&mut AllowResult>) {
        let state = *self.state.read().await;
        let config = self.config.read().await;

        if let Some(permit) = permit {
            permit.release();
        }

        // 重置失败计数；成功意味着额度/可用性已恢复，清掉致命标记
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.fatal_open.store(false, Ordering::SeqCst);
        self.total_requests.fetch_add(1, Ordering::SeqCst);

        if state == CircuitState::HalfOpen {
            let successes = self.consecutive_successes.fetch_add(1, Ordering::SeqCst) + 1;

            if successes >= config.success_threshold {
                drop(config); // 释放读锁再转换状态
                log::info!(
                    "[{}] 熔断器 HalfOpen → Closed (恢复正常)",
                    log_cb::HALF_OPEN_TO_CLOSED
                );
                self.transition_to_closed().await;
            }
        }
    }

    /// 记录失败
    pub async fn record_failure(&self, permit: Option<&mut AllowResult>) {
        let state = *self.state.read().await;
        // 普通失败不保留致命标记；随后的 transition_to_open 会正常打开熔断
        self.fatal_open.store(false, Ordering::SeqCst);
        let config = self.config.read().await;

        if let Some(permit) = permit {
            permit.release();
        }

        // 更新计数器
        let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
        self.total_requests.fetch_add(1, Ordering::SeqCst);
        self.failed_requests.fetch_add(1, Ordering::SeqCst);

        // 重置成功计数
        self.consecutive_successes.store(0, Ordering::SeqCst);

        // 检查是否应该打开熔断器
        match state {
            CircuitState::HalfOpen => {
                // HalfOpen 状态下失败，立即转为 Open
                log::warn!(
                    "[{}] 熔断器 HalfOpen 探测失败 → Open",
                    log_cb::HALF_OPEN_PROBE_FAILED
                );
                drop(config);
                self.transition_to_open().await;
            }
            CircuitState::Closed => {
                // 检查连续失败次数
                if failures >= config.failure_threshold {
                    log::warn!(
                        "[{}] 熔断器触发: 连续失败 {failures} 次 → Open",
                        log_cb::TRIGGERED_FAILURES
                    );
                    drop(config); // 释放读锁再转换状态
                    self.transition_to_open().await;
                } else {
                    // 检查错误率
                    let total = self.total_requests.load(Ordering::SeqCst);
                    let failed = self.failed_requests.load(Ordering::SeqCst);

                    if total >= config.min_requests {
                        let error_rate = failed as f64 / total as f64;

                        if error_rate >= config.error_rate_threshold {
                            log::warn!(
                                "[{}] 熔断器触发: 错误率 {:.1}% → Open",
                                log_cb::TRIGGERED_ERROR_RATE,
                                error_rate * 100.0
                            );
                            drop(config); // 释放读锁再转换状态
                            self.transition_to_open().await;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Record an unrecoverable provider failure and open the circuit immediately.
    ///
    /// Billing exhaustion must not wait for the normal failure threshold: retrying
    /// the same provider only repeats the upstream 403 and may surface a payment
    /// flow to the client.
    pub async fn record_fatal_failure(&self, permit: Option<&mut AllowResult>) {
        if let Some(permit) = permit {
            permit.release();
        }

        let config = self.config.read().await;
        self.consecutive_failures
            .store(config.failure_threshold, Ordering::SeqCst);
        drop(config);

        self.total_requests.fetch_add(1, Ordering::SeqCst);
        self.failed_requests.fetch_add(1, Ordering::SeqCst);
        self.consecutive_successes.store(0, Ordering::SeqCst);
        self.fatal_open.store(true, Ordering::SeqCst);
        self.transition_to_open().await;
    }

    /// 获取当前状态
    /// 熔断是否由致命失败（额度耗尽）触发，且仍处于 Open 窗口内。
    ///
    /// 供单 Provider bypass 路径使用：普通熔断不阻断唯一通道，但额度耗尽
    /// 期间应快速失败，避免每次请求都重复打到已耗尽的上游。
    pub async fn is_fatally_open(&self) -> bool {
        if *self.state.read().await != CircuitState::Open {
            return false;
        }
        if !self.fatal_open.load(Ordering::SeqCst) {
            return false;
        }
        let config = self.config.read().await;
        match *self.last_opened_at.read().await {
            Some(opened_at) => opened_at.elapsed().as_secs() < config.timeout_seconds,
            None => false,
        }
    }

    #[allow(dead_code)]
    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// 获取统计信息
    ///
    /// 额外暴露 `recovery_remaining_secs` / `fatal` / `half_open_probes`，
    /// 让前端能把「还要等多久」直接显示出来，而不是只有一个 "熔断" 标签。
    pub async fn get_stats(&self) -> CircuitBreakerStats {
        let state = *self.state.read().await;
        let config = self.config.read().await;
        let opened_at = *self.last_opened_at.read().await;

        let recovery_remaining_secs = match (state, opened_at) {
            (CircuitState::Open, Some(opened_at)) => config
                .timeout_seconds
                .saturating_sub(opened_at.elapsed().as_secs()),
            _ => 0,
        };

        CircuitBreakerStats {
            state,
            consecutive_failures: self.consecutive_failures.load(Ordering::SeqCst),
            consecutive_successes: self.consecutive_successes.load(Ordering::SeqCst),
            total_requests: self.total_requests.load(Ordering::SeqCst),
            failed_requests: self.failed_requests.load(Ordering::SeqCst),
            recovery_remaining_secs,
            fatal: self.fatal_open.load(Ordering::SeqCst),
            half_open_probes: u32::from(self.half_open_ticket.load(Ordering::SeqCst) != 0),
        }
    }

    /// 重置熔断器（手动恢复）
    #[allow(dead_code)]
    pub async fn reset(&self) {
        log::info!("[{}] 熔断器手动重置 → Closed", log_cb::MANUAL_RESET);
        self.transition_to_closed().await;
    }

    fn allow_half_open_probe(&self) -> AllowResult {
        let ticket = self.next_probe_ticket.fetch_add(1, Ordering::SeqCst);
        if ticket != 0
            && self
                .half_open_ticket
                .compare_exchange(0, ticket, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        {
            AllowResult {
                allowed: true,
                used_half_open_permit: true,
                probe: Some(HalfOpenPermit {
                    slot: self.half_open_ticket.clone(),
                    ticket,
                }),
            }
        } else {
            AllowResult::without_probe(false)
        }
    }

    /// 转换到打开状态
    async fn transition_to_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Open;
        self.half_open_ticket.store(0, Ordering::SeqCst);
        *self.last_opened_at.write().await = Some(Instant::now());
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.consecutive_successes.store(0, Ordering::SeqCst);
    }

    /// 转换到半开状态
    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        if *state != CircuitState::Open {
            return;
        }

        *state = CircuitState::HalfOpen;
        self.consecutive_successes.store(0, Ordering::SeqCst);
        // 重置半开状态的请求限流计数
        self.half_open_ticket.store(0, Ordering::SeqCst);
    }

    /// 转换到关闭状态
    async fn transition_to_closed(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;
        self.half_open_ticket.store(0, Ordering::SeqCst);
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.consecutive_successes.store(0, Ordering::SeqCst);
        // 重置计数器
        self.total_requests.store(0, Ordering::SeqCst);
        self.failed_requests.store(0, Ordering::SeqCst);
    }
}

/// 熔断器统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub total_requests: u32,
    pub failed_requests: u32,
    /// 距离自动恢复探测还有多少秒（仅 Open 状态有意义，其余为 0）
    pub recovery_remaining_secs: u64,
    /// 本次 Open 是否由额度耗尽（致命失败）触发
    pub fatal: bool,
    /// 半开状态下已放行的探测请求数（便于判断探测是否卡住）
    pub half_open_probes: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // 初始状态应该是关闭
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert!(breaker.allow_request().await.allowed);

        // 记录 3 次失败
        for _ in 0..3 {
            breaker.record_failure(None).await;
        }

        // 应该转换到打开状态
        assert_eq!(breaker.get_state().await, CircuitState::Open);
        assert!(!breaker.allow_request().await.allowed);
    }

    #[tokio::test]
    async fn fatal_failure_marks_breaker_and_normal_failure_does_not() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            timeout_seconds: 60,
            ..Default::default()
        });

        // 普通失败未达阈值：既非 Open，也不应被标记为致命
        breaker.record_failure(None).await;
        assert!(!breaker.is_fatally_open().await);

        // 达到阈值打开：普通失败不应产生致命标记
        breaker.record_failure(None).await;
        breaker.record_failure(None).await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);
        assert!(!breaker.is_fatally_open().await);

        // 致命失败（额度耗尽）：打开且标记为致命
        let breaker2 = CircuitBreaker::new(CircuitBreakerConfig::default());
        breaker2.record_fatal_failure(None).await;
        assert_eq!(breaker2.get_state().await, CircuitState::Open);
        assert!(breaker2.is_fatally_open().await);

        // 恢复窗口结束后不再快速失败（交由 HalfOpen 探测）
        let breaker3 = CircuitBreaker::new(CircuitBreakerConfig {
            timeout_seconds: 0,
            ..Default::default()
        });
        breaker3.record_fatal_failure(None).await;
        assert!(!breaker3.is_fatally_open().await);
    }

    #[tokio::test]
    async fn success_clears_fatal_marker() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
        breaker.record_fatal_failure(None).await;
        assert!(breaker.is_fatally_open().await);

        breaker.record_success(None).await;
        // 成功记录后状态仍为 Open（需要探测恢复），但致命标记已被清除
        assert!(!breaker.is_fatally_open().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_closed() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // 打开熔断器
        breaker.record_failure(None).await;
        breaker.record_failure(None).await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // 手动转换到半开状态
        breaker.transition_to_half_open().await;
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);

        // 记录 2 次成功
        breaker.record_success(None).await;
        breaker.record_success(None).await;

        // 应该转换到关闭状态
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_half_open_transition_does_not_reset_inflight_permit() {
        let config = CircuitBreakerConfig {
            timeout_seconds: 0,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // 进入 Open，然后由于 timeout_seconds=0，allow_request 会立即切换到 HalfOpen 并占用探测名额
        breaker.transition_to_open().await;
        let first = breaker.allow_request().await;
        assert!(first.allowed);
        assert!(first.used_half_open_permit);
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);

        // 模拟并发下的“重复 HalfOpen 转换调用”，不应重置 in-flight 计数
        breaker.transition_to_half_open().await;

        // 由于名额仍被占用，第二次请求应被拒绝
        let second = breaker.allow_request().await;
        assert!(!second.allowed);
        assert!(!second.used_half_open_permit);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // 打开熔断器
        breaker.record_failure(None).await;
        breaker.record_failure(None).await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // 重置
        breaker.reset().await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert!(breaker.allow_request().await.allowed);
    }

    #[tokio::test]
    async fn cancelled_probe_releases_slot_without_changing_health() {
        let breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_seconds: 0,
            ..Default::default()
        }));
        breaker.record_failure(None).await;
        let before = breaker.get_stats().await;
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let request_breaker = breaker.clone();
        let request = tokio::spawn(async move {
            let permit = request_breaker.allow_request().await;
            assert!(permit.used_half_open_permit);
            ready_tx.send(()).unwrap();
            std::future::pending::<()>().await;
            drop(permit);
        });
        ready_rx.await.unwrap();
        assert!(!breaker.allow_request().await.allowed);
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        let after = breaker.get_stats().await;
        assert_eq!(after.half_open_probes, 0);
        assert_eq!(after.total_requests, before.total_requests);
        assert_eq!(after.failed_requests, before.failed_requests);
        assert_eq!(after.consecutive_failures, before.consecutive_failures);
        assert!(breaker.allow_request().await.used_half_open_permit);
    }

    #[tokio::test]
    async fn old_probe_after_reset_cannot_release_a_new_probe() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_seconds: 0,
            ..Default::default()
        });
        breaker.record_failure(None).await;
        let old = breaker.allow_request().await;
        assert!(old.used_half_open_permit);
        breaker.reset().await;
        breaker.record_failure(None).await;
        let new = breaker.allow_request().await;
        assert!(new.used_half_open_permit);
        drop(old);
        assert_eq!(breaker.get_stats().await.half_open_probes, 1);
        assert!(!breaker.allow_request().await.allowed);
        drop(new);
        assert_eq!(breaker.get_stats().await.half_open_probes, 0);
        assert!(breaker.allow_request().await.allowed);
    }
}
