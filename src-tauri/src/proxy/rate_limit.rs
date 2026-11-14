//! 共享限流冷却注册表
//!
//! ## 解决的问题
//!
//! 429 等待重试按请求独立计数。本表共享冷却信息以便优先选择其他供应商，
//! 并防止较早发出的成功请求抹去新的限流窗口；它不是请求排队或节流器。
//!
//! 本模块把供应商的冷却期限保存为进程内共享状态：
//!
//! - 收到 429 时登记冷却截止时间（`note_rate_limited`）
//! - 路由选择阶段把仍在冷却中的 provider 排到队尾（`cooldown_remaining`）
//! - 成功只能清除本次发送前已知的冷却，不能清除并发请求后来登记的窗口
//!
//! ## 与熔断器的分工
//!
//! 熔断器负责「这个 provider 坏了吗」（按失败率/连续失败，分钟级恢复窗口）；
//! 冷却表负责「这个 provider 现在让我等多久」（按 Retry-After，秒级窗口）。
//! 两者独立：冷却中的 provider 仍然会被尝试（排到队尾），只是不再是首选。

use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 冷却表 key：`"app_type:provider_id"`
pub(crate) type CooldownKey = String;

/// A request observes the current generation immediately before sending.
/// Even a shorter overlapping 429 advances it, so an older success cannot
/// mistake an in-flight rate-limit response for confirmed recovery.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CooldownEpoch(u64);

/// 构造冷却表 key。
///
/// 必须与 `ProviderRouter` 的熔断器 key 保持同一格式，方便日志里对照排查。
pub(crate) fn cooldown_key(app_type: &str, provider_id: &str) -> CooldownKey {
    format!("{app_type}:{provider_id}")
}

/// 单次限流冷却记录
#[derive(Debug, Clone)]
struct CooldownEntry {
    /// 冷却截止时刻
    until: Instant,
    /// 上游给出的 Retry-After（无该头时为 None，说明走的是本地退避估算）
    retry_after_secs: Option<u64>,
    generation: u64,
}

impl CooldownEntry {
    fn remaining_secs(&self, now: Instant) -> u64 {
        // Instant 不能直接相减得到 Duration 的负数，用 checked_duration_since
        match self.until.checked_duration_since(now) {
            Some(remaining) => remaining
                .as_secs()
                .saturating_add(u64::from(remaining.subsec_nanos() > 0)),
            None => 0,
        }
    }

    fn is_expired(&self, now: Instant) -> bool {
        self.until <= now
    }
}

/// 冷却快照（序列化给前端）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CooldownSnapshot {
    pub provider_id: String,
    pub app_type: String,
    /// 剩余冷却秒数（向上取整到秒，至少 1）
    pub remaining_secs: u64,
    /// 上游 Retry-After 原值（无则 None）
    pub retry_after_secs: Option<u64>,
}

/// 进程内共享的限流冷却注册表
///
/// 由 `ProviderRouter` 持有，生命周期与代理服务器一致。
#[derive(Default)]
pub struct RateLimitCooldown {
    entries: RwLock<HashMap<CooldownKey, CooldownEntry>>,
    generation: AtomicU64,
}

impl RateLimitCooldown {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn epoch(&self) -> CooldownEpoch {
        CooldownEpoch(self.generation.load(Ordering::SeqCst))
    }

    /// 登记一次限流冷却。
    ///
    /// - 多次登记取「最晚的截止时间」，避免并发的短冷却覆盖掉一个长冷却。
    /// - `wait_secs == 0` 时不登记（上游说可以立刻重试，没必要冷却）。
    pub async fn note_rate_limited(
        &self,
        app_type: &str,
        provider_id: &str,
        wait_secs: u64,
        retry_after_secs: Option<u64>,
    ) {
        if wait_secs == 0 {
            return;
        }

        let key = cooldown_key(app_type, provider_id);
        let candidate_until = Instant::now() + Duration::from_secs(wait_secs);

        let mut entries = self.entries.write().await;
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        match entries.get_mut(&key) {
            Some(existing) => {
                existing.generation = generation;
                if candidate_until > existing.until {
                    existing.until = candidate_until;
                    existing.retry_after_secs = retry_after_secs;
                }
            }
            None => {
                entries.insert(
                    key,
                    CooldownEntry {
                        until: candidate_until,
                        retry_after_secs,
                        generation,
                    },
                );
            }
        }
    }

    /// 查询剩余冷却秒数；未冷却或已过期返回 0。
    pub async fn remaining_secs(&self, app_type: &str, provider_id: &str) -> u64 {
        let key = cooldown_key(app_type, provider_id);
        let now = Instant::now();

        // 读锁即可完成查询；过期条目的清理放到写入路径，避免读路径频繁抢写锁
        let entries = self.entries.read().await;
        match entries.get(&key) {
            Some(entry) if entry.is_expired(now) => 0,
            Some(entry) => entry.remaining_secs(now),
            None => 0,
        }
    }

    /// Explicit reset; ordinary request success uses clear_after_success.
    pub async fn clear(&self, app_type: &str, provider_id: &str) {
        let key = cooldown_key(app_type, provider_id);
        let mut entries = self.entries.write().await;
        entries.remove(&key);
    }

    pub(crate) async fn clear_after_success(
        &self,
        app_type: &str,
        provider_id: &str,
        observed: CooldownEpoch,
    ) {
        let key = cooldown_key(app_type, provider_id);
        let mut entries = self.entries.write().await;
        if entries
            .get(&key)
            .is_some_and(|entry| entry.generation <= observed.0)
        {
            entries.remove(&key);
        }
    }

    /// 顺带清理所有已过期条目，返回清理数量。
    ///
    /// 供 `get_circuit_breaker_stats` 之类的低频读路径调用，避免表无限增长。
    pub async fn prune_expired(&self) -> usize {
        let now = Instant::now();
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|_, entry| !entry.is_expired(now));
        before - entries.len()
    }

    /// 导出当前所有活跃冷却（供健康摘要展示）。
    pub async fn snapshots(&self) -> Vec<CooldownSnapshot> {
        let now = Instant::now();
        let entries = self.entries.read().await;

        entries
            .iter()
            .filter_map(|(key, entry)| {
                if entry.is_expired(now) {
                    return None;
                }
                // key 格式固定为 "app_type:provider_id"，按第一个 ':' 切分。
                // provider_id 本身可能含 ':'，所以只切一刀。
                let (app_type, provider_id) = key.split_once(':')?;
                Some(CooldownSnapshot {
                    provider_id: provider_id.to_string(),
                    app_type: app_type.to_string(),
                    remaining_secs: entry.remaining_secs(now).max(1),
                    retry_after_secs: entry.retry_after_secs,
                })
            })
            .collect()
    }
}

/// 判断是否需要为一次限流登记共享冷却。
///
/// Respect the existing setting: enabled rate-limit handling records positive
/// waits, including a final limit that exceeds this request's retry budget.
pub(crate) fn should_register_cooldown(
    rate_limit_retry_enabled: bool,
    wait_secs: Option<u64>,
) -> bool {
    rate_limit_retry_enabled && matches!(wait_secs, Some(secs) if secs > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_fractional_second_remains_in_cooldown() {
        let now = Instant::now();
        let entry = CooldownEntry {
            until: now + Duration::from_millis(750),
            retry_after_secs: Some(1),
            generation: 1,
        };
        assert_eq!(entry.remaining_secs(now), 1);
        assert_eq!(entry.remaining_secs(entry.until), 0);
    }

    #[tokio::test]
    async fn success_only_clears_cooldown_known_at_attempt_start() {
        let cooldown = RateLimitCooldown::new();
        let earlier = cooldown.epoch();
        cooldown.note_rate_limited("codex", "a", 30, Some(30)).await;
        cooldown.clear_after_success("codex", "a", earlier).await;
        assert!(cooldown.remaining_secs("codex", "a").await > 0);
        let current = cooldown.epoch();
        cooldown
            .note_rate_limited("claude", "b", 60, Some(60))
            .await;
        cooldown.clear_after_success("codex", "a", current).await;
        assert_eq!(cooldown.remaining_secs("codex", "a").await, 0);
        assert!(cooldown.remaining_secs("claude", "b").await > 0);
    }

    #[tokio::test]
    async fn shorter_overlapping_limit_and_reset_do_not_allow_a_stale_clear() {
        let cooldown = RateLimitCooldown::new();
        cooldown.note_rate_limited("codex", "a", 30, Some(30)).await;
        let observed = cooldown.epoch();
        cooldown.note_rate_limited("codex", "a", 1, Some(1)).await;
        cooldown.clear_after_success("codex", "a", observed).await;
        assert!(cooldown.remaining_secs("codex", "a").await > 20);
        cooldown.clear("codex", "a").await;
        cooldown.note_rate_limited("codex", "a", 30, Some(30)).await;
        cooldown.clear_after_success("codex", "a", observed).await;
        assert!(cooldown.remaining_secs("codex", "a").await > 20);
    }

    #[tokio::test]
    async fn cooldown_is_visible_after_registration() {
        let cooldown = RateLimitCooldown::new();
        cooldown
            .note_rate_limited("claude", "provider-a", 30, Some(30))
            .await;

        let remaining = cooldown.remaining_secs("claude", "provider-a").await;
        assert!((29..=30).contains(&remaining), "remaining={remaining}");
    }

    #[tokio::test]
    async fn cooldown_is_scoped_per_app_type() {
        let cooldown = RateLimitCooldown::new();
        cooldown
            .note_rate_limited("claude", "shared-id", 30, None)
            .await;

        assert!(cooldown.remaining_secs("claude", "shared-id").await > 0);
        // 同名 provider id 在另一个 app 下不应被连带冷却
        assert_eq!(cooldown.remaining_secs("codex", "shared-id").await, 0);
    }

    #[tokio::test]
    async fn longer_cooldown_wins_over_shorter_one() {
        let cooldown = RateLimitCooldown::new();
        cooldown
            .note_rate_limited("claude", "p", 120, Some(120))
            .await;
        // 后到的短冷却不能把长冷却截断
        cooldown.note_rate_limited("claude", "p", 5, Some(5)).await;

        let remaining = cooldown.remaining_secs("claude", "p").await;
        assert!(remaining > 100, "remaining={remaining}");
    }

    #[tokio::test]
    async fn shorter_cooldown_does_not_extend_existing_one() {
        let cooldown = RateLimitCooldown::new();
        cooldown.note_rate_limited("claude", "p", 5, Some(5)).await;
        cooldown
            .note_rate_limited("claude", "p", 120, Some(120))
            .await;

        // 长冷却覆盖短冷却
        let remaining = cooldown.remaining_secs("claude", "p").await;
        assert!(remaining > 100, "remaining={remaining}");
    }

    #[tokio::test]
    async fn zero_wait_is_not_registered() {
        let cooldown = RateLimitCooldown::new();
        cooldown.note_rate_limited("claude", "p", 0, Some(0)).await;

        assert_eq!(cooldown.remaining_secs("claude", "p").await, 0);
        assert!(cooldown.snapshots().await.is_empty());
    }

    #[tokio::test]
    async fn clear_removes_cooldown() {
        let cooldown = RateLimitCooldown::new();
        cooldown.note_rate_limited("claude", "p", 60, None).await;
        assert!(cooldown.remaining_secs("claude", "p").await > 0);

        cooldown.clear("claude", "p").await;
        assert_eq!(cooldown.remaining_secs("claude", "p").await, 0);
    }

    #[tokio::test]
    async fn snapshots_round_trip_provider_id_with_colon() {
        let cooldown = RateLimitCooldown::new();
        // provider id 自身含 ':'，切分必须只切第一刀
        cooldown
            .note_rate_limited("codex", "vendor:model-x", 45, Some(45))
            .await;

        let snapshots = cooldown.snapshots().await;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].app_type, "codex");
        assert_eq!(snapshots[0].provider_id, "vendor:model-x");
        assert_eq!(snapshots[0].retry_after_secs, Some(45));
    }

    #[test]
    fn cooldown_registration_requires_enabled_retry_with_wait() {
        assert!(should_register_cooldown(true, Some(30)));
        assert!(!should_register_cooldown(true, None));
        assert!(!should_register_cooldown(true, Some(0)));
        assert!(!should_register_cooldown(false, Some(30)));
    }
}
