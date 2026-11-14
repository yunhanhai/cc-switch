//! 供应商路由器模块
//!
//! 负责选择和管理代理目标供应商，实现智能故障转移

use crate::app_config::AppType;
use crate::database::Database;
use crate::error::AppError;
use crate::provider::Provider;
use crate::proxy::circuit_breaker::{AllowResult, CircuitBreaker, CircuitBreakerConfig};
use crate::proxy::rate_limit::{CooldownEpoch, CooldownSnapshot, RateLimitCooldown};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Codex Official requests carry the selected account's native Authorization
/// header. Reusing that request against another account card would cross the
/// account boundary, so these cards must never participate in provider retry.
pub(crate) fn provider_supports_failover(app_type: &str, provider: &Provider) -> bool {
    app_type != AppType::Codex.as_str()
        || !crate::proxy::providers::is_codex_official_provider(provider)
}

/// 供应商路由器
pub struct ProviderRouter {
    /// 数据库连接
    db: Arc<Database>,
    /// 熔断器管理器 - key 格式: "app_type:provider_id"
    circuit_breakers: Arc<RwLock<HashMap<String, Arc<CircuitBreaker>>>>,
    /// 共享限流冷却表 - key 格式同上
    ///
    /// 熔断器回答「这个 provider 坏了吗」，冷却表回答「它让我等多久」。
    /// 冷却中的 provider 不会被剔除，只会被排到队尾（见 `select_providers`）。
    rate_limit_cooldown: Arc<RateLimitCooldown>,
}

impl ProviderRouter {
    /// 创建新的供应商路由器
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            rate_limit_cooldown: Arc::new(RateLimitCooldown::new()),
        }
    }

    /// 选择可用的供应商（支持故障转移）
    ///
    /// 返回按优先级排序的可用供应商列表：
    /// - 故障转移关闭时：仅返回当前供应商
    /// - 故障转移开启时：仅使用故障转移队列，按队列顺序依次尝试（P1 → P2 → ...）
    pub async fn select_providers(&self, app_type: &str) -> Result<Vec<Provider>, AppError> {
        self.select_providers_for_model(app_type, None).await
    }

    /// 选择可用的供应商，并按请求模型过滤候选渠道（model alias 路由）。
    ///
    /// `request_model` 为 `None` 时行为与 [`select_providers`] 完全一致。
    ///
    /// 过滤只发生在**故障转移链**上，且遵循两条保守原则：
    /// 1. 该 app 完全没有 alias 数据时不过滤（安全阀，避免升级后全部不可用）；
    /// 2. 有自有 `model_mapping` 的渠道即使没声明 alias 也保留（可能是通配规则）。
    ///
    /// 过滤后若一个渠道都不剩，**回退到未过滤的完整链**——宁可让上游返回
    /// model_not_found（用户能看懂），也不要在本地静默返回 NoAvailableProvider。
    pub async fn select_providers_for_model(
        &self,
        app_type: &str,
        request_model: Option<&str>,
    ) -> Result<Vec<Provider>, AppError> {
        let mut result = Vec::new();
        let mut total_providers = 0usize;
        let mut circuit_open_count = 0usize;
        let current_id = AppType::from_str(app_type)
            .ok()
            .and_then(|app_enum| {
                crate::settings::get_effective_current_provider(&self.db, &app_enum)
                    .ok()
                    .flatten()
            })
            .or_else(|| self.db.get_current_provider(app_type).ok().flatten());
        let current_provider = current_id
            .as_deref()
            .map(|id| self.db.get_provider_by_id(id, app_type))
            .transpose()?
            .flatten();

        // 检查该应用的自动故障转移开关是否开启（从 proxy_config 表读取）
        let auto_failover_enabled = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(config) => config.auto_failover_enabled,
            Err(e) => {
                log::error!("[{app_type}] 读取 proxy_config 失败: {e}，默认禁用故障转移");
                false
            }
        };

        if auto_failover_enabled
            && current_provider
                .as_ref()
                .is_some_and(|provider| !provider_supports_failover(app_type, provider))
        {
            // A selected Codex Official account is an explicit account choice.
            // Keep it as a single route even if an old failover setting remains
            // enabled; retrying would reuse its inbound token for another card.
            total_providers = 1;
            result.push(current_provider.expect("checked above"));
        } else if auto_failover_enabled {
            // 故障转移开启：仅按队列顺序依次尝试（P1 → P2 → ...）
            let all_providers = self.db.get_all_providers(app_type)?;

            // 统一网关：优先使用已启用路由分组的拼接顺序（smart 分组），
            // 无启用分组或成员为空时回退到旧的 in_failover_queue 队列，保证存量库与空分组的兼容性。
            let ordered_ids: Vec<String> =
                match self.db.get_enabled_routing_group_provider_ids(app_type) {
                    Ok(ids) if !ids.is_empty() => {
                        log::debug!(
                            "[{app_type}] 使用路由分组选路（{} 个成员，跨 {} 个已启用分组）",
                            ids.len(),
                            self.db
                                .list_routing_groups(app_type)
                                .map(|gs| gs.iter().filter(|g| g.group.enabled).count())
                                .unwrap_or(0)
                        );
                        ids
                    }
                    Ok(_) => self
                        .db
                        .get_failover_queue(app_type)?
                        .into_iter()
                        .map(|item| item.provider_id)
                        .collect(),
                    Err(e) => {
                        log::warn!("[{app_type}] 读取路由分组失败，回退到旧队列: {e}");
                        self.db
                            .get_failover_queue(app_type)?
                            .into_iter()
                            .map(|item| item.provider_id)
                            .collect()
                    }
                };

            // 先按队列顺序收集可用 provider，同时记下每个的剩余限流冷却。
            // 冷却中的 provider 不剔除 —— 它只是不该当首选。
            let mut available: Vec<(Provider, u64)> = Vec::with_capacity(ordered_ids.len());

            // model alias 过滤上下文：只在调用方给了模型名、且该 app 配了 alias 时
            // 才加载（`model_alias_index` 为空即安全阀生效，后续不过滤）。
            // 读取失败按"没有 alias"处理，绝不能因新功能让选路整体失败。
            let alias_index = match request_model {
                Some(model) if !model.trim().is_empty() => {
                    match self.db.model_alias_index(app_type) {
                        Ok(index) => {
                            if index.is_empty() {
                                None
                            } else {
                                log::debug!(
                                    "[{app_type}] model alias 生效：{} 条映射，请求模型 {model}",
                                    index.len()
                                );
                                Some(index)
                            }
                        }
                        Err(e) => {
                            log::warn!("[{app_type}] 读取 model alias 失败，跳过模型过滤: {e}");
                            None
                        }
                    }
                }
                _ => None,
            };

            for provider_id in ordered_ids {
                let Some(provider) = all_providers.get(&provider_id).cloned() else {
                    continue;
                };
                if !provider_supports_failover(app_type, &provider) {
                    continue;
                }
                total_providers += 1;

                // 模型级过滤：该渠道既不认识这个 alias，也没有自有映射时跳过。
                // 被跳过的渠道不参与熔断统计（`total_providers` 已累加，但
                // `circuit_open_count` 不会增加），因此不会误报"全部熔断"。
                if let (Some(model), Some(index)) = (request_model, alias_index.as_ref()) {
                    let has_own_mapping =
                        crate::proxy::model_mapper::provider_has_own_model_mapping(&provider);
                    if !crate::proxy::model_alias::provider_supports_alias(
                        index,
                        model,
                        &provider.id,
                        has_own_mapping,
                    ) {
                        log::debug!(
                            "[{app_type}] 跳过渠道 {}：未声明模型 {model} 且无自有映射",
                            provider.name
                        );
                        continue;
                    }
                }

                let circuit_key = format!("{app_type}:{}", provider.id);
                let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;

                if breaker.is_available().await {
                    let cooldown_secs = self
                        .rate_limit_cooldown
                        .remaining_secs(app_type, &provider.id)
                        .await;
                    available.push((provider, cooldown_secs));
                } else {
                    circuit_open_count += 1;
                }
            }

            // 安全兜底：模型过滤把渠道全滤掉了，说明 alias 配置与当前渠道不匹配。
            // 此时回退到"不按模型过滤"的完整链，让上游返回真实的 model_not_found，
            // 而不是在本地静默报 NoAvailableProvider（用户无法据此排查）。
            if available.is_empty() && alias_index.is_some() {
                log::warn!(
                    "[{app_type}] model alias 过滤后无可用渠道，回退到完整故障转移链（请检查 alias 配置）"
                );
                for provider_id in self
                    .db
                    .get_enabled_routing_group_provider_ids(app_type)?
                    .into_iter()
                    .chain(
                        self.db
                            .get_failover_queue(app_type)?
                            .into_iter()
                            .map(|item| item.provider_id),
                    )
                {
                    let Some(provider) = all_providers.get(&provider_id).cloned() else {
                        continue;
                    };
                    if !provider_supports_failover(app_type, &provider) {
                        continue;
                    }
                    if available
                        .iter()
                        .any(|(existing, _)| existing.id == provider.id)
                    {
                        continue;
                    }
                    let circuit_key = format!("{app_type}:{}", provider.id);
                    let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;
                    if breaker.is_available().await {
                        let cooldown_secs = self
                            .rate_limit_cooldown
                            .remaining_secs(app_type, &provider.id)
                            .await;
                        available.push((provider, cooldown_secs));
                    }
                }
            }

            // 稳定重排：冷却中的排到队尾，冷却时间长的更靠后。
            // 用「是否冷却」作为主键而不是直接按秒数排序，是为了保证
            // 未冷却的 provider 之间仍严格保持用户配置的 P1→P2 顺序。
            let any_cooling = available.iter().any(|(_, secs)| *secs > 0);
            if any_cooling {
                available.sort_by_key(|(_, cooldown_secs)| {
                    if *cooldown_secs == 0 {
                        (0u8, 0u64)
                    } else {
                        (1u8, *cooldown_secs)
                    }
                });

                let cooling_count = available
                    .iter()
                    .filter(|(_, cooldown_secs)| *cooldown_secs > 0)
                    .count();
                log::info!(
                    "[{app_type}] {cooling_count} 个 provider 处于限流冷却，已排到队列末尾（最长 {}s）",
                    available.last().map(|(_, secs)| *secs).unwrap_or(0)
                );
            }

            result.extend(available.into_iter().map(|(provider, _)| provider));
        } else {
            // 故障转移关闭：仅使用当前供应商，跳过熔断器检查
            if let Some(current) = current_provider {
                total_providers = 1;
                result.push(current);
            }
        }

        if result.is_empty() {
            if total_providers > 0 && circuit_open_count == total_providers {
                log::warn!("[{app_type}] [FO-004] 所有供应商均已熔断");
                return Err(AppError::AllProvidersCircuitOpen);
            } else {
                log::warn!("[{app_type}] [FO-005] 未配置供应商");
                return Err(AppError::NoProvidersConfigured);
            }
        }

        Ok(result)
    }

    /// 请求执行前获取熔断器“放行许可”
    ///
    /// - Closed：直接放行
    /// - Open：超时到达后切到 HalfOpen 并放行一次探测
    /// - HalfOpen：按限流规则放行探测
    ///
    /// The returned owner releases a HalfOpen slot even if the request is cancelled.
    pub async fn allow_provider_request(&self, provider_id: &str, app_type: &str) -> AllowResult {
        let circuit_key = format!("{app_type}:{provider_id}");
        let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;
        breaker.allow_request().await
    }

    /// 记录供应商请求结果
    pub async fn record_result(
        &self,
        provider_id: &str,
        app_type: &str,
        permit: Option<&mut AllowResult>,
        success: bool,
        error_msg: Option<String>,
    ) -> Result<(), AppError> {
        // 1. 按应用独立获取熔断器配置
        let failure_threshold = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(app_config) => app_config.circuit_failure_threshold,
            Err(_) => 5, // 默认值
        };

        // 2. 更新熔断器状态
        let circuit_key = format!("{app_type}:{provider_id}");
        let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;

        if success {
            breaker.record_success(permit).await;
        } else {
            breaker.record_failure(permit).await;
        }

        // 3. 更新数据库健康状态（使用配置的阈值）
        self.db
            .update_provider_health_with_threshold(
                provider_id,
                app_type,
                success,
                error_msg.clone(),
                failure_threshold,
            )
            .await?;

        Ok(())
    }

    /// Record a billing/quota failure and immediately open the provider circuit.
    /// 指定 provider 是否正处于「致命熔断」窗口（单 Provider bypass 路径专用）。
    pub async fn is_fatally_open(&self, provider_id: &str, app_type: &str) -> bool {
        let circuit_key = format!("{app_type}:{provider_id}");
        let breakers = self.circuit_breakers.read().await;
        match breakers.get(&circuit_key) {
            Some(breaker) => breaker.is_fatally_open().await,
            None => false,
        }
    }

    pub async fn record_fatal_result(
        &self,
        provider_id: &str,
        app_type: &str,
        permit: Option<&mut AllowResult>,
        error_msg: String,
    ) -> Result<(), AppError> {
        let circuit_key = format!("{app_type}:{provider_id}");
        let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;
        breaker.record_fatal_failure(permit).await;
        self.db
            .update_provider_health_with_threshold(
                provider_id,
                app_type,
                false,
                Some(error_msg),
                // Billing exhaustion is terminal for this provider until the
                // circuit recovery probe succeeds; reflect that immediately
                // in the persisted health state as well.
                1,
            )
            .await
    }

    /// 重置熔断器（手动恢复）
    pub async fn reset_circuit_breaker(&self, circuit_key: &str) {
        let breakers = self.circuit_breakers.read().await;
        if let Some(breaker) = breakers.get(circuit_key) {
            breaker.reset().await;
        }
    }

    /// 重置指定供应商的全部「路由惩罚」状态：熔断器 + 限流冷却。
    ///
    /// 只重置熔断器是不够的：用户点「重置熔断」的意图是「这家现在可以用了」，
    /// 但冷却表是独立的另一份状态，残留的冷却仍会在 `select_providers` 里
    /// 把它排到队尾 —— 表现为「点了重置却还是不走这家」，而界面上熔断徽章
    /// 已经消失，用户无从判断原因。
    pub async fn reset_provider_breaker(&self, provider_id: &str, app_type: &str) {
        let circuit_key = format!("{app_type}:{provider_id}");
        self.reset_circuit_breaker(&circuit_key).await;
        self.rate_limit_cooldown.clear(app_type, provider_id).await;
    }

    // ==================== 共享限流冷却 ====================

    /// 登记一次限流冷却。
    ///
    /// Records shared routing preference after rate limits. Cooling providers
    /// remain candidates; this does not serialize or throttle concurrent calls.
    pub async fn note_rate_limited(
        &self,
        provider_id: &str,
        app_type: &str,
        wait_secs: u64,
        retry_after_secs: Option<u64>,
    ) {
        self.rate_limit_cooldown
            .note_rate_limited(app_type, provider_id, wait_secs, retry_after_secs)
            .await;
    }

    /// 查询单个 provider 的剩余限流冷却秒数（0 表示未冷却）。
    ///
    /// 与 `rate_limit_cooldown_snapshots` 的区别：这是一次 O(1) 的点查，
    /// 适合被前端按 provider 高频轮询；快照接口会 clone 整张表，只该用于
    /// 汇总展示。健康查询走这条路径，避免 N 个卡片各拉一次全表。
    pub async fn rate_limit_cooldown_secs(&self, provider_id: &str, app_type: &str) -> u64 {
        self.rate_limit_cooldown
            .remaining_secs(app_type, provider_id)
            .await
    }

    /// Explicitly reset this provider's cooldown.
    #[cfg(test)]
    pub async fn clear_rate_limit_cooldown(&self, provider_id: &str, app_type: &str) {
        self.rate_limit_cooldown.clear(app_type, provider_id).await;
    }

    pub(crate) fn cooldown_epoch(&self) -> CooldownEpoch {
        self.rate_limit_cooldown.epoch()
    }

    pub(crate) async fn clear_cooldown_after_success(
        &self,
        provider_id: &str,
        app_type: &str,
        observed: CooldownEpoch,
    ) {
        self.rate_limit_cooldown
            .clear_after_success(app_type, provider_id, observed)
            .await;
    }

    /// 导出所有活跃冷却快照，并顺带清理过期条目。
    pub async fn rate_limit_cooldown_snapshots(&self) -> Vec<CooldownSnapshot> {
        // 低频读路径：顺手回收过期条目，避免长期运行后 map 无限增长
        let pruned = self.rate_limit_cooldown.prune_expired().await;
        if pruned > 0 {
            log::debug!("[RateLimit] 清理 {pruned} 条过期限流冷却记录");
        }
        self.rate_limit_cooldown.snapshots().await
    }

    /// 更新所有熔断器的配置（热更新）
    pub async fn update_all_configs(&self, config: CircuitBreakerConfig) {
        let breakers = self.circuit_breakers.read().await;
        for breaker in breakers.values() {
            breaker.update_config(config.clone()).await;
        }
    }

    /// 更新指定应用已创建熔断器的配置（热更新）
    pub async fn update_app_configs(&self, app_type: &str, config: CircuitBreakerConfig) {
        let prefix = format!("{app_type}:");
        let breakers = self.circuit_breakers.read().await;
        for (key, breaker) in breakers.iter() {
            if key.starts_with(&prefix) {
                breaker.update_config(config.clone()).await;
            }
        }
    }

    /// 获取熔断器状态
    #[allow(dead_code)]
    pub async fn get_circuit_breaker_stats(
        &self,
        provider_id: &str,
        app_type: &str,
    ) -> Option<crate::proxy::circuit_breaker::CircuitBreakerStats> {
        let circuit_key = format!("{app_type}:{provider_id}");
        let breakers = self.circuit_breakers.read().await;

        if let Some(breaker) = breakers.get(&circuit_key) {
            Some(breaker.get_stats().await)
        } else {
            None
        }
    }

    /// 获取或创建熔断器
    async fn get_or_create_circuit_breaker(&self, key: &str) -> Arc<CircuitBreaker> {
        // 先尝试读锁获取
        {
            let breakers = self.circuit_breakers.read().await;
            if let Some(breaker) = breakers.get(key) {
                return breaker.clone();
            }
        }

        // 如果不存在，获取写锁创建
        let mut breakers = self.circuit_breakers.write().await;

        // 双重检查，防止竞争条件
        if let Some(breaker) = breakers.get(key) {
            return breaker.clone();
        }

        // 从 key 中提取 app_type (格式: "app_type:provider_id")
        let app_type = key.split(':').next().unwrap_or("claude");

        // 按应用独立读取熔断器配置
        let config = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(app_config) => crate::proxy::circuit_breaker::CircuitBreakerConfig {
                failure_threshold: app_config.circuit_failure_threshold,
                success_threshold: app_config.circuit_success_threshold,
                timeout_seconds: app_config.circuit_timeout_seconds as u64,
                error_rate_threshold: app_config.circuit_error_rate_threshold,
                min_requests: app_config.circuit_min_requests,
            },
            Err(_) => crate::proxy::circuit_breaker::CircuitBreakerConfig::default(),
        };

        let breaker = Arc::new(CircuitBreaker::new(config));
        breakers.insert(key.to_string(), breaker.clone());

        breaker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::provider::{AuthBinding, AuthBindingSource, ProviderMeta};
    use serde_json::json;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    fn managed_codex_official(id: &str, account_id: &str) -> Provider {
        let mut provider = Provider::with_id(
            id.to_string(),
            "OpenAI Official".to_string(),
            json!({ "auth": {}, "config": "" }),
            None,
        );
        provider.category = Some("official".to_string());
        provider.meta = Some(ProviderMeta {
            provider_type: Some("codex_oauth".to_string()),
            auth_binding: Some(AuthBinding {
                source: AuthBindingSource::ManagedAccount,
                auth_provider: Some("codex_oauth".to_string()),
                account_id: Some(account_id.to_string()),
            }),
            ..Default::default()
        });
        provider
    }

    struct TempHome {
        #[allow(dead_code)]
        dir: TempDir,
        original_home: Option<String>,
        original_userprofile: Option<String>,
        original_test_home: Option<String>,
    }

    impl TempHome {
        fn new() -> Self {
            let dir = TempDir::new().expect("failed to create temp home");
            let original_home = env::var("HOME").ok();
            let original_userprofile = env::var("USERPROFILE").ok();
            let original_test_home = env::var("CC_SWITCH_TEST_HOME").ok();

            env::set_var("HOME", dir.path());
            env::set_var("USERPROFILE", dir.path());
            env::set_var("CC_SWITCH_TEST_HOME", dir.path());
            crate::settings::reload_settings().expect("reload settings");

            Self {
                dir,
                original_home,
                original_userprofile,
                original_test_home,
            }
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            match &self.original_home {
                Some(value) => env::set_var("HOME", value),
                None => env::remove_var("HOME"),
            }

            match &self.original_userprofile {
                Some(value) => env::set_var("USERPROFILE", value),
                None => env::remove_var("USERPROFILE"),
            }

            match &self.original_test_home {
                Some(value) => env::set_var("CC_SWITCH_TEST_HOME", value),
                None => env::remove_var("CC_SWITCH_TEST_HOME"),
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_provider_router_creation() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let router = ProviderRouter::new(db);

        let breaker = router.get_or_create_circuit_breaker("claude:test").await;
        assert!(breaker.allow_request().await.allowed);
    }

    #[tokio::test]
    #[serial]
    async fn test_failover_disabled_uses_current_provider() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        let provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();
        db.add_to_failover_queue("claude", "b").unwrap();

        let router = ProviderRouter::new(db.clone());
        let providers = router.select_providers("claude").await.unwrap();

        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "a");
    }

    #[tokio::test]
    #[serial]
    async fn test_failover_enabled_uses_queue_order_ignoring_current() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        // 设置 sort_index 来控制顺序：b=1, a=2
        let mut provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        provider_a.sort_index = Some(2);
        let mut provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);
        provider_b.sort_index = Some(1);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();

        db.add_to_failover_queue("claude", "b").unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();

        // 启用自动故障转移（使用新的 proxy_config API）
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());
        let providers = router.select_providers("claude").await.unwrap();

        assert_eq!(providers.len(), 2);
        // 故障转移开启时：仅按队列顺序选择（忽略当前供应商）
        assert_eq!(providers[0].id, "b");
        assert_eq!(providers[1].id, "a");
    }

    #[tokio::test]
    #[serial]
    async fn test_failover_enabled_uses_queue_only_even_if_current_not_in_queue() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        let mut provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);
        provider_b.sort_index = Some(1);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();

        // 只把 b 加入故障转移队列（模拟“当前供应商不在队列里”的常见配置）
        db.add_to_failover_queue("claude", "b").unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());
        let providers = router.select_providers("claude").await.unwrap();

        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "b");
    }

    /// 冷却中的 provider 不剔除，只排到队尾。
    ///
    /// 这是本特性的核心语义：429 冷却不等于「这个站点坏了」，
    /// 如果把它从候选链里删掉，单 provider 场景会直接变成「无可用供应商」。
    #[tokio::test]
    #[serial]
    async fn rate_limited_provider_moves_to_queue_tail_without_being_dropped() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        // P1 = a（sort_index 1），P2 = b（sort_index 2）
        let mut provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        provider_a.sort_index = Some(1);
        let mut provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);
        provider_b.sort_index = Some(2);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();
        db.add_to_failover_queue("claude", "b").unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        // P1 被限流 30 秒
        router.note_rate_limited("a", "claude", 30, Some(30)).await;

        let providers = router.select_providers("claude").await.unwrap();
        assert_eq!(
            providers
                .iter()
                .map(|provider| provider.id.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "a"],
            "冷却中的 P1 应排到队尾，但不能被剔除"
        );
    }

    /// 未冷却的 provider 之间必须严格保持用户配置的 P1→P2 顺序。
    ///
    /// 排序主键是「是否冷却」而不是「剩余秒数」，否则一旦有站点冷却，
    /// 其余健康站点的相对顺序会被剩余秒数意外打乱。
    #[tokio::test]
    #[serial]
    async fn non_cooling_providers_keep_configured_order() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        for (id, sort_index) in [("a", 1), ("b", 2), ("c", 3)] {
            let mut provider =
                Provider::with_id(id.to_string(), format!("Provider {id}"), json!({}), None);
            provider.sort_index = Some(sort_index);
            db.save_provider("claude", &provider).unwrap();
            db.add_to_failover_queue("claude", id).unwrap();
        }

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());
        // 队尾的 c 冷却，a/b 都健康
        router.note_rate_limited("c", "claude", 60, Some(60)).await;

        let providers = router.select_providers("claude").await.unwrap();
        assert_eq!(
            providers
                .iter()
                .map(|provider| provider.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    /// 请求成功后清除冷却，下一次选路立刻恢复原顺序。
    #[tokio::test]
    #[serial]
    async fn clearing_cooldown_restores_configured_order() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        let mut provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        provider_a.sort_index = Some(1);
        let mut provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);
        provider_b.sort_index = Some(2);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();
        db.add_to_failover_queue("claude", "b").unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());
        router.note_rate_limited("a", "claude", 30, Some(30)).await;
        let cooldown_of = |snapshots: &[CooldownSnapshot], provider_id: &str, app_type: &str| {
            snapshots
                .iter()
                .find(|snapshot| {
                    snapshot.provider_id == provider_id && snapshot.app_type == app_type
                })
                .map(|snapshot| snapshot.remaining_secs)
                .unwrap_or(0)
        };
        assert!(
            cooldown_of(&router.rate_limit_cooldown_snapshots().await, "a", "claude") > 0,
            "登记冷却后应能读到剩余秒数"
        );

        router.clear_rate_limit_cooldown("a", "claude").await;
        assert_eq!(
            cooldown_of(&router.rate_limit_cooldown_snapshots().await, "a", "claude"),
            0
        );

        let providers = router.select_providers("claude").await.unwrap();
        assert_eq!(
            providers
                .iter()
                .map(|provider| provider.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    /// 冷却快照必须按 app_type 隔离，避免同名 provider id 跨应用串味。
    #[tokio::test]
    #[serial]
    async fn cooldown_snapshots_are_scoped_per_app() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let router = ProviderRouter::new(db);

        router
            .note_rate_limited("shared-id", "claude", 30, Some(30))
            .await;

        let snapshots = router.rate_limit_cooldown_snapshots().await;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].app_type, "claude");
        assert_eq!(snapshots[0].provider_id, "shared-id");
        // codex 下同名 provider 不应出现在快照里
        assert!(snapshots
            .iter()
            .all(|snapshot| snapshot.app_type != "codex"));
    }

    /// 重置熔断必须同时清掉限流冷却。
    ///
    /// 两者是独立的「路由惩罚」状态：用户点「重置熔断」的意图是「这家现在
    /// 可以用了」，只清熔断却留着冷却，会让该供应商继续被排到队尾，而界面
    /// 上的熔断徽章已经消失 —— 用户看不到任何提示，只觉得"重置没生效"。
    #[tokio::test]
    #[serial]
    async fn resetting_circuit_breaker_also_clears_rate_limit_cooldown() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let router = ProviderRouter::new(db);

        router
            .note_rate_limited("provider-a", "claude", 120, Some(120))
            .await;
        assert!(
            router
                .rate_limit_cooldown_secs("provider-a", "claude")
                .await
                > 0,
            "前置条件：应已登记冷却"
        );

        router.reset_provider_breaker("provider-a", "claude").await;

        assert_eq!(
            router
                .rate_limit_cooldown_secs("provider-a", "claude")
                .await,
            0,
            "重置熔断后冷却必须一起清掉"
        );
        assert!(
            router.rate_limit_cooldown_snapshots().await.is_empty(),
            "快照里也不应再出现该供应商"
        );
    }

    /// 重置一个供应商不应波及其它供应商或其它应用的冷却。
    #[tokio::test]
    #[serial]
    async fn resetting_one_provider_leaves_other_cooldowns_intact() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let router = ProviderRouter::new(db);

        router
            .note_rate_limited("target", "claude", 120, Some(120))
            .await;
        router
            .note_rate_limited("bystander", "claude", 120, Some(120))
            .await;
        router
            .note_rate_limited("target", "codex", 120, Some(120))
            .await;

        router.reset_provider_breaker("target", "claude").await;

        assert_eq!(router.rate_limit_cooldown_secs("target", "claude").await, 0);
        assert!(
            router.rate_limit_cooldown_secs("bystander", "claude").await > 0,
            "同应用其它供应商的冷却必须保留"
        );
        assert!(
            router.rate_limit_cooldown_secs("target", "codex").await > 0,
            "另一个应用下同名供应商的冷却必须保留"
        );
    }

    #[tokio::test]
    #[serial]
    async fn codex_official_current_stays_single_route_when_failover_is_stale() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let official = managed_codex_official("official-a", "account-a");
        let fallback = Provider::with_id(
            "fallback".to_string(),
            "Fallback".to_string(),
            json!({}),
            None,
        );
        db.save_provider("codex", &official).unwrap();
        db.save_provider("codex", &fallback).unwrap();
        db.set_current_provider("codex", &official.id).unwrap();
        db.add_to_failover_queue("codex", &fallback.id).unwrap();

        let mut config = db.get_proxy_config_for_app("codex").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let providers = ProviderRouter::new(db)
            .select_providers("codex")
            .await
            .unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, official.id);
    }

    #[tokio::test]
    #[serial]
    async fn stale_codex_official_queue_entries_are_not_retry_targets() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let current = Provider::with_id(
            "third-party".to_string(),
            "Third Party".to_string(),
            json!({}),
            None,
        );
        let official = managed_codex_official("official-a", "account-a");
        let fallback = Provider::with_id(
            "fallback".to_string(),
            "Fallback".to_string(),
            json!({}),
            None,
        );
        db.save_provider("codex", &current).unwrap();
        db.save_provider("codex", &official).unwrap();
        db.save_provider("codex", &fallback).unwrap();
        db.set_current_provider("codex", &current.id).unwrap();
        db.add_to_failover_queue("codex", &official.id).unwrap();
        db.add_to_failover_queue("codex", &fallback.id).unwrap();

        let mut config = db.get_proxy_config_for_app("codex").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let providers = ProviderRouter::new(db)
            .select_providers("codex")
            .await
            .unwrap();
        assert_eq!(
            providers
                .iter()
                .map(|provider| provider.id.as_str())
                .collect::<Vec<_>>(),
            vec!["fallback"]
        );
    }

    /// 无 alias 配置时，传入模型名不得改变选路结果（安全阀）。
    #[tokio::test]
    #[serial]
    async fn select_providers_for_model_without_aliases_is_unchanged() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        for id in ["a", "b"] {
            db.save_provider(
                "claude",
                &Provider::with_id(id.to_string(), id.to_string(), json!({}), None),
            )
            .unwrap();
            db.add_to_failover_queue("claude", id).unwrap();
        }
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db);
        let without_model = router.select_providers("claude").await.unwrap();
        let with_model = router
            .select_providers_for_model("claude", Some("claude-sonnet-4-5"))
            .await
            .unwrap();
        assert_eq!(
            with_model.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            without_model
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>(),
            "无 alias 时模型过滤必须不生效"
        );
    }

    /// 配了 alias 后，未声明该模型的渠道应被跳过。
    #[tokio::test]
    #[serial]
    async fn select_providers_for_model_skips_undeclared_channels() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        for id in ["a", "b", "c"] {
            db.save_provider(
                "claude",
                &Provider::with_id(id.to_string(), id.to_string(), json!({}), None),
            )
            .unwrap();
            db.add_to_failover_queue("claude", id).unwrap();
        }
        // a 与 c 声明了 sonnet，b 没有（也没有自有映射）
        db.upsert_model_alias("claude", "sonnet", "a", "a-sonnet", None, None)
            .unwrap();
        db.upsert_model_alias("claude", "sonnet", "c", "c-sonnet", None, None)
            .unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let providers = ProviderRouter::new(db)
            .select_providers_for_model("claude", Some("sonnet"))
            .await
            .unwrap();
        assert_eq!(
            providers.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            vec!["a", "c"],
            "未声明 sonnet 的渠道 b 应被跳过"
        );
    }

    /// 有自有 model_mapping 的渠道即使没声明 alias 也应保留。
    #[tokio::test]
    #[serial]
    async fn select_providers_for_model_keeps_channels_with_own_mapping() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        db.save_provider(
            "claude",
            &Provider::with_id("a".to_string(), "a".to_string(), json!({}), None),
        )
        .unwrap();
        // b 没有 alias，但有通配 model_mapping
        db.save_provider(
            "claude",
            &Provider::with_id(
                "b".to_string(),
                "b".to_string(),
                json!({"model_mapping": [{"source": "*", "target": "fallback-model"}]}),
                None,
            ),
        )
        .unwrap();
        for id in ["a", "b"] {
            db.add_to_failover_queue("claude", id).unwrap();
        }
        db.upsert_model_alias("claude", "sonnet", "a", "a-sonnet", None, None)
            .unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let providers = ProviderRouter::new(db)
            .select_providers_for_model("claude", Some("sonnet"))
            .await
            .unwrap();
        assert_eq!(
            providers.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"],
            "有自有映射的渠道不应被过滤"
        );
    }

    /// 过滤后无渠道时回退到完整链，而不是报 NoProvidersConfigured。
    #[tokio::test]
    #[serial]
    async fn select_providers_for_model_falls_back_when_everything_filtered() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        db.save_provider(
            "claude",
            &Provider::with_id("a".to_string(), "a".to_string(), json!({}), None),
        )
        .unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();
        // 只为 opus 配 alias；请求 sonnet 时所有渠道都被滤掉
        db.upsert_model_alias("claude", "opus", "a", "a-opus", None, None)
            .unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let providers = ProviderRouter::new(db)
            .select_providers_for_model("claude", Some("sonnet"))
            .await
            .expect("应回退到完整链而不是报错");
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "a");
    }

    #[tokio::test]
    #[serial]
    async fn test_select_providers_does_not_consume_half_open_permit() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        db.update_circuit_breaker_config(&CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_seconds: 0,
            ..Default::default()
        })
        .await
        .unwrap();

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        let provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();

        db.add_to_failover_queue("claude", "a").unwrap();
        db.add_to_failover_queue("claude", "b").unwrap();

        // 启用自动故障转移（使用新的 proxy_config API）
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        router
            .record_result("b", "claude", None, false, Some("fail".to_string()))
            .await
            .unwrap();

        let providers = router.select_providers("claude").await.unwrap();
        assert_eq!(providers.len(), 2);

        assert!(router.allow_provider_request("b", "claude").await.allowed);
    }

    #[tokio::test]
    #[serial]
    async fn test_release_permit_neutral_frees_half_open_slot() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        // 配置熔断器：1 次失败即熔断，0 秒超时立即进入 HalfOpen
        db.update_circuit_breaker_config(&CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_seconds: 0,
            ..Default::default()
        })
        .await
        .unwrap();

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        db.save_provider("claude", &provider_a).unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();

        // 启用自动故障转移
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        // 触发熔断：1 次失败
        router
            .record_result("a", "claude", None, false, Some("fail".to_string()))
            .await
            .unwrap();

        // 第一次请求：获取 HalfOpen 探测名额
        let mut first = router.allow_provider_request("a", "claude").await;
        assert!(first.allowed);
        assert!(first.used_half_open_permit);

        // 第二次请求应被拒绝（名额已被占用）
        let second = router.allow_provider_request("a", "claude").await;
        assert!(!second.allowed);

        // Neutral release leaves provider health unchanged.
        first.release();

        // 第三次请求应被允许（名额已释放）
        let third = router.allow_provider_request("a", "claude").await;
        assert!(third.allowed);
        assert!(third.used_half_open_permit);
    }
}
