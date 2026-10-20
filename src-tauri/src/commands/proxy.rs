//! 代理服务相关的 Tauri 命令
//!
//! 提供前端调用的 API 接口

use crate::error::AppError;
use crate::proxy::types::*;
use crate::proxy::{CircuitBreakerConfig, CircuitBreakerStats};
use crate::store::AppState;
use std::str::FromStr;

fn require_proxy_app(app_type: &str) -> Result<crate::app_config::AppType, String> {
    let app = crate::app_config::AppType::from_str(app_type)
        .map_err(|error| format!("无效的应用类型: {error}"))?;
    if !app.supports_local_proxy() {
        return Err(format!("{} 不支持本地路由", app.as_str()));
    }
    Ok(app)
}

/// 启动代理服务器（仅启动服务，不接管 Live 配置）
#[tauri::command]
pub async fn start_proxy_server(
    state: tauri::State<'_, AppState>,
) -> Result<ProxyServerInfo, String> {
    state.proxy_service.start().await
}

/// 停止代理服务器（仅停止服务，不恢复/清理 Live 接管状态）
#[tauri::command]
pub async fn stop_proxy_server(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let takeover = state.proxy_service.get_takeover_status().await?;
    if takeover.claude
        || takeover.codex
        || takeover.gemini
        || takeover.grokbuild
        || takeover.opencode
        || takeover.openclaw
    {
        return Err(
            "仍有应用处于代理接管状态，请先在设置中关闭对应应用接管后再停止本地路由。".to_string(),
        );
    }

    state.proxy_service.stop().await
}

/// 停止代理服务器（恢复 Live 配置）
#[tauri::command]
pub async fn stop_proxy_with_restore(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.proxy_service.stop_with_restore().await
}

/// 获取各应用接管状态
#[tauri::command]
pub async fn get_proxy_takeover_status(
    state: tauri::State<'_, AppState>,
) -> Result<ProxyTakeoverStatus, String> {
    state.proxy_service.get_takeover_status().await
}

/// 为指定应用开启/关闭接管
#[tauri::command]
pub async fn set_proxy_takeover_for_app(
    state: tauri::State<'_, AppState>,
    app_type: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .proxy_service
        .set_takeover_for_app(&app_type, enabled)
        .await
}

/// 获取代理服务器状态
#[tauri::command]
pub async fn get_proxy_status(state: tauri::State<'_, AppState>) -> Result<ProxyStatus, String> {
    state.proxy_service.get_status().await
}

/// 获取代理配置
#[tauri::command]
pub async fn get_proxy_config(state: tauri::State<'_, AppState>) -> Result<ProxyConfig, String> {
    state.proxy_service.get_config().await
}

/// 更新代理配置
#[tauri::command]
pub async fn update_proxy_config(
    state: tauri::State<'_, AppState>,
    config: ProxyConfig,
) -> Result<(), String> {
    state.proxy_service.update_config(&config).await
}

// ==================== Global & Per-App Config ====================

/// 获取全局代理配置
///
/// 返回统一的全局配置字段（代理开关、监听地址、端口、日志开关）
#[tauri::command]
pub async fn get_global_proxy_config(
    state: tauri::State<'_, AppState>,
) -> Result<GlobalProxyConfig, String> {
    let db = &state.db;
    db.get_global_proxy_config()
        .await
        .map_err(|e| e.to_string())
}

/// 更新全局代理配置
///
/// 更新统一的全局配置字段，会同时更新三行（claude/codex/gemini）
#[tauri::command]
pub async fn update_global_proxy_config(
    state: tauri::State<'_, AppState>,
    config: GlobalProxyConfig,
) -> Result<(), String> {
    let db = &state.db;
    db.update_global_proxy_config(config)
        .await
        .map_err(|e| e.to_string())
}

/// 获取指定应用的代理配置
///
/// 返回应用级配置（enabled、auto_failover、超时、熔断器等）
#[tauri::command]
pub async fn get_proxy_config_for_app(
    state: tauri::State<'_, AppState>,
    app_type: String,
) -> Result<AppProxyConfig, String> {
    require_proxy_app(&app_type)?;
    let db = &state.db;
    db.get_proxy_config_for_app(&app_type)
        .await
        .map_err(|e| e.to_string())
}

/// 更新指定应用的代理配置
///
/// 更新应用级配置（enabled、auto_failover、超时、熔断器等）
#[tauri::command]
pub async fn update_proxy_config_for_app(
    state: tauri::State<'_, AppState>,
    config: AppProxyConfig,
) -> Result<(), String> {
    let db = &state.db;
    let app_type = config.app_type.clone();
    require_proxy_app(&app_type)?;
    let circuit_config = CircuitBreakerConfig::from(&config);

    db.update_proxy_config_for_app(config)
        .await
        .map_err(|e| e.to_string())?;

    state
        .proxy_service
        .update_circuit_breaker_config_for_app(&app_type, circuit_config)
        .await
}

async fn get_default_cost_multiplier_internal(
    state: &AppState,
    app_type: &str,
) -> Result<String, AppError> {
    let db = &state.db;
    db.get_default_cost_multiplier(app_type).await
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub async fn get_default_cost_multiplier_test_hook(
    state: &AppState,
    app_type: &str,
) -> Result<String, AppError> {
    get_default_cost_multiplier_internal(state, app_type).await
}

/// 获取默认成本倍率
#[tauri::command]
pub async fn get_default_cost_multiplier(
    state: tauri::State<'_, AppState>,
    app_type: String,
) -> Result<String, String> {
    get_default_cost_multiplier_internal(&state, &app_type)
        .await
        .map_err(|e| e.to_string())
}

async fn set_default_cost_multiplier_internal(
    state: &AppState,
    app_type: &str,
    value: &str,
) -> Result<(), AppError> {
    let db = &state.db;
    db.set_default_cost_multiplier(app_type, value).await
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub async fn set_default_cost_multiplier_test_hook(
    state: &AppState,
    app_type: &str,
    value: &str,
) -> Result<(), AppError> {
    set_default_cost_multiplier_internal(state, app_type, value).await
}

/// 设置默认成本倍率
#[tauri::command]
pub async fn set_default_cost_multiplier(
    state: tauri::State<'_, AppState>,
    app_type: String,
    value: String,
) -> Result<(), String> {
    set_default_cost_multiplier_internal(&state, &app_type, &value)
        .await
        .map_err(|e| e.to_string())
}

async fn get_pricing_model_source_internal(
    state: &AppState,
    app_type: &str,
) -> Result<String, AppError> {
    let db = &state.db;
    db.get_pricing_model_source(app_type).await
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub async fn get_pricing_model_source_test_hook(
    state: &AppState,
    app_type: &str,
) -> Result<String, AppError> {
    get_pricing_model_source_internal(state, app_type).await
}

/// 获取计费模式来源
#[tauri::command]
pub async fn get_pricing_model_source(
    state: tauri::State<'_, AppState>,
    app_type: String,
) -> Result<String, String> {
    get_pricing_model_source_internal(&state, &app_type)
        .await
        .map_err(|e| e.to_string())
}

async fn set_pricing_model_source_internal(
    state: &AppState,
    app_type: &str,
    value: &str,
) -> Result<(), AppError> {
    let db = &state.db;
    db.set_pricing_model_source(app_type, value).await
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub async fn set_pricing_model_source_test_hook(
    state: &AppState,
    app_type: &str,
    value: &str,
) -> Result<(), AppError> {
    set_pricing_model_source_internal(state, app_type, value).await
}

/// 设置计费模式来源
#[tauri::command]
pub async fn set_pricing_model_source(
    state: tauri::State<'_, AppState>,
    app_type: String,
    value: String,
) -> Result<(), String> {
    set_pricing_model_source_internal(&state, &app_type, &value)
        .await
        .map_err(|e| e.to_string())
}

/// 检查代理服务器是否正在运行
#[tauri::command]
pub async fn is_proxy_running(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    Ok(state.proxy_service.is_running().await)
}

/// 检查是否处于 Live 接管模式
#[tauri::command]
pub async fn is_live_takeover_active(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    state.proxy_service.is_takeover_active().await
}

/// 代理模式下切换供应商（热切换）
#[tauri::command]
pub async fn switch_proxy_provider(
    state: tauri::State<'_, AppState>,
    app_type: String,
    provider_id: String,
) -> Result<(), String> {
    let app = require_proxy_app(&app_type)?;
    // Codex official account cards can use the client's native OpenAI login
    // through takeover. Other apps' official providers remain blocked.
    let provider = state
        .db
        .get_provider_by_id(&provider_id, &app_type)
        .map_err(|e| format!("读取供应商失败: {e}"))?
        .ok_or_else(|| format!("供应商不存在: {provider_id}"))?;
    if provider.category.as_deref() == Some("official")
        && !crate::services::provider::official_provider_supports_proxy_takeover(&app, &provider)
    {
        return Err(
            "代理接管模式下不能切换到官方供应商 (Cannot switch to official provider during proxy takeover)"
                .to_string(),
        );
    }

    state
        .proxy_service
        .switch_proxy_target(&app_type, &provider_id)
        .await
}

// ==================== 故障转移相关命令 ====================

/// 获取供应商健康状态
///
/// 返回的是「持久化健康计数」+「进程内存态」的合成视图：
/// - `is_healthy` / `consecutive_failures` 来自数据库（跨重启保留）
/// - `circuit_state` / `recovery_remaining_secs` / `rate_limit_cooldown_secs`
///   来自运行中的熔断器与冷却表（重启即重置）
///
/// 这样前端的一次 5 秒轮询就能同时拿到「为什么熔断」和「还要等多久」。
#[tauri::command]
pub async fn get_provider_health(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    app_type: String,
) -> Result<ProviderHealth, String> {
    require_proxy_app(&app_type)?;
    let db = &state.db;
    let mut health = db
        .get_provider_health(&provider_id, &app_type)
        .await
        .map_err(|e| e.to_string())?;

    // 内存态可能不可用（代理未运行 / 该 provider 尚无流量），此时保持默认值
    if let Ok(Some(stats)) = state
        .proxy_service
        .get_provider_circuit_breaker_stats(&provider_id, &app_type)
        .await
    {
        health.circuit_state = Some(stats.state.to_string());
        health.recovery_remaining_secs = stats.recovery_remaining_secs;
        health.fatal = stats.fatal;
    }

    // 点查而非拉全表：这个命令被每个 provider 卡片独立轮询（5s 一次），
    // 若每次都 clone 整张冷却表再线性过滤，provider 越多浪费越大。
    if let Ok(cooldown_secs) = state
        .proxy_service
        .get_rate_limit_cooldown_secs(&provider_id, &app_type)
        .await
    {
        health.rate_limit_cooldown_secs = cooldown_secs;
    }

    Ok(health)
}

/// 韧性健康摘要：今日转移次数 + 429 等待统计 + 当前冷却状态 + 最近检测延迟概况
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResilienceSummary {
    pub app_type: String,
    /// 今日故障转移次数
    pub today_failover_count: u64,
    /// 本次运行期 429 等待重试次数
    pub rate_limit_wait_count: u64,
    /// 本次运行期 429 累计等待秒数
    pub rate_limit_wait_total_secs: u64,
    /// 最近一轮检测的平均延迟（毫秒，无数据为 None）
    pub avg_latency_ms: Option<u64>,
    /// 最近一轮检测的最慢站点（名称 + 毫秒）
    pub slowest_provider: Option<(String, u64)>,
    /// 最近一轮检测不可达站点数
    pub unreachable_count: u64,
    /// 当前处于 429 限流冷却中的站点数
    pub cooling_down_count: u64,
    /// 当前最长剩余冷却秒数（无冷却为 0）
    pub longest_cooldown_secs: u64,
}

/// 连通性检测结果的聚合概况
#[derive(Debug, Default, PartialEq, Eq)]
pub struct StreamCheckSummary {
    /// 不可达站点数
    pub unreachable_count: u64,
    /// 可达站点的平均延迟（无可用样本为 None）
    pub avg_latency_ms: Option<u64>,
    /// 最慢站点（provider_id + 毫秒）
    pub slowest: Option<(String, u64)>,
}

/// 把「每个 provider 最近一次检测结果」聚合成摘要指标。
///
/// 抽成纯函数是为了能直接测边界情况 —— 尤其是「该应用一个供应商都没有」
/// 这种只能靠构造状态复现的场景，在命令层里没法单独验证。
pub fn summarize_stream_checks(
    checks: &[crate::database::LatestStreamCheck],
) -> StreamCheckSummary {
    let mut summary = StreamCheckSummary::default();
    let mut latency_total_ms = 0u64;
    let mut latency_samples = 0u64;

    for check in checks {
        if !check.success {
            summary.unreachable_count += 1;
            continue;
        }
        let Some(latency_ms) = check.response_time_ms else {
            continue;
        };
        latency_total_ms = latency_total_ms.saturating_add(latency_ms);
        latency_samples += 1;
        if summary
            .slowest
            .as_ref()
            .is_none_or(|(_, previous_ms)| latency_ms > *previous_ms)
        {
            summary.slowest = Some((check.provider_id.clone(), latency_ms));
        }
    }

    summary.avg_latency_ms = (latency_samples > 0).then(|| latency_total_ms / latency_samples);
    summary
}

#[tauri::command]
pub async fn get_resilience_summary(
    state: tauri::State<'_, crate::store::AppState>,
    app_type: String,
) -> Result<ResilienceSummary, String> {
    use chrono::Utc;

    // 今日转移次数：直接 COUNT，避免被「取最近 N 条」的上限截断
    let today_start = Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0);
    let today_failover_count = state
        .db
        .count_failover_events_since(&app_type, today_start)
        .map_err(|e| e.to_string())?;

    // 429 统计来自运行中的代理状态。
    //
    // 必须读 per-app 分桶而不是全局累计：ProxyStatus 是代理服务的单例，
    // 全局计数把所有应用的 429 等待混在一起，直接展示会让 claude 的限流
    // 等待出现在 codex 的摘要里。
    let app_rate_limit_waits = match state.proxy_service.get_status().await {
        Ok(st) => st.rate_limit_waits_for(&app_type),
        Err(_) => Default::default(),
    };

    // 当前仍处于冷却窗口的站点（429 Retry-After 驱动的进程内共享状态）
    let cooldowns = state
        .proxy_service
        .get_rate_limit_cooldowns(&app_type)
        .await
        .unwrap_or_default();

    // 最近一轮检测延迟概况
    let checks = state
        .db
        .get_latest_stream_checks(&app_type)
        .map_err(|e| e.to_string())?;

    // 只统计仍然存在的供应商：需要 provider 名称来标注「最慢站点」。
    //
    // 删除供应商时已经会级联清理检测记录，但历史数据库里可能残留旧记录
    // （清理逻辑上线前删掉的供应商）。这些行会让「不可达」「平均延迟」把
    // 已经不存在的站点也算进去，而用户没有任何入口能发现或修正它。
    //
    // 这里只查一次 provider 表并复用：`get_all_providers` 内部会为每个供应商
    // 单独查一次 endpoints（N+1），而本函数被前端每 15 秒轮询一次，调用两遍
    // 等于把 N+1 翻倍。
    let all_providers = state.db.get_all_providers(&app_type);
    let provider_names: Option<std::collections::HashMap<String, String>> =
        all_providers.as_ref().ok().map(|providers| {
            providers
                .iter()
                .map(|(id, provider)| (id.clone(), provider.name.clone()))
                .collect()
        });

    // 查询失败时才回退到「不过滤」（宁可显示旧数据，也不要凭空清零）；
    // 查询成功则严格按现有供应商过滤 —— 包括「该应用一个供应商都没有」的情况，
    // 此时应当全部剔除，而不是因为集合为空就放过所有孤儿记录。
    let checks: Vec<_> = match &provider_names {
        Some(names) => checks
            .into_iter()
            .filter(|check| names.contains_key(&check.provider_id))
            .collect(),
        None => {
            log::warn!("[{app_type}] 读取供应商列表失败，韧性摘要将包含无法校验归属的检测记录");
            checks
        }
    };

    let check_summary = summarize_stream_checks(&checks);

    let slowest_provider = check_summary.slowest.map(|(provider_id, latency_ms)| {
        let name = provider_names
            .as_ref()
            .and_then(|names| names.get(&provider_id).cloned())
            .unwrap_or(provider_id);
        (name, latency_ms)
    });

    Ok(ResilienceSummary {
        app_type,
        today_failover_count,
        rate_limit_wait_count: app_rate_limit_waits.count,
        rate_limit_wait_total_secs: app_rate_limit_waits.total_secs,
        avg_latency_ms: check_summary.avg_latency_ms,
        slowest_provider,
        unreachable_count: check_summary.unreachable_count,
        cooling_down_count: cooldowns.len() as u64,
        longest_cooldown_secs: cooldowns
            .iter()
            .map(|cooldown| cooldown.remaining_secs)
            .max()
            .unwrap_or(0),
    })
}

/// 查询最近的故障转移事件（前端时间线）
#[tauri::command]
pub fn get_failover_events(
    state: tauri::State<'_, crate::store::AppState>,
    app_type: String,
    limit: Option<u32>,
) -> Result<Vec<crate::database::FailoverEvent>, String> {
    state
        .db
        .get_failover_events(&app_type, limit.unwrap_or(20).min(100))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reset_circuit_breaker(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    provider_id: String,
    app_type: String,
) -> Result<(), String> {
    require_proxy_app(&app_type)?;
    // 1. 重置数据库健康状态
    let db = &state.db;
    db.update_provider_health(&provider_id, &app_type, true, None)
        .await
        .map_err(|e| e.to_string())?;

    // 2. 如果代理正在运行，重置内存中的熔断器状态
    state
        .proxy_service
        .reset_provider_circuit_breaker(&provider_id, &app_type)
        .await?;

    // 3. 检查是否应该切回优先级更高的供应商（从 proxy_config 表读取）
    // 只有当该应用已被代理接管（enabled=true）且开启了自动故障转移时才执行
    let (app_enabled, auto_failover_enabled) = match db.get_proxy_config_for_app(&app_type).await {
        Ok(config) => (config.enabled, config.auto_failover_enabled),
        Err(e) => {
            log::error!("[{app_type}] Failed to read proxy_config: {e}, defaulting to disabled");
            (false, false)
        }
    };

    if app_enabled && auto_failover_enabled && state.proxy_service.is_running().await {
        // 获取当前供应商 ID
        let current_id = db
            .get_current_provider(&app_type)
            .map_err(|e| e.to_string())?;

        if let Some(current_id) = current_id {
            // 获取故障转移队列
            let queue = db
                .get_failover_queue(&app_type)
                .map_err(|e| e.to_string())?;

            // 找到恢复的供应商和当前供应商在队列中的位置（使用 sort_index）
            let restored_order = queue
                .iter()
                .find(|item| item.provider_id == provider_id)
                .and_then(|item| item.sort_index);

            let current_order = queue
                .iter()
                .find(|item| item.provider_id == current_id)
                .and_then(|item| item.sort_index);

            // 如果恢复的供应商优先级更高（sort_index 更小），则切换
            if let (Some(restored), Some(current)) = (restored_order, current_order) {
                if restored < current {
                    log::info!(
                        "[Recovery] 供应商 {provider_id} 已恢复且优先级更高 (P{restored} vs P{current})，自动切换"
                    );

                    // 获取供应商名称用于日志和事件
                    let provider_name = db
                        .get_all_providers(&app_type)
                        .ok()
                        .and_then(|providers| providers.get(&provider_id).map(|p| p.name.clone()))
                        .unwrap_or_else(|| provider_id.clone());

                    // 创建故障转移切换管理器并执行切换
                    // reason=None：这是用户开启自动转移后的即时切换，不是故障驱动，
                    // 前端不会为它弹"请求失败"类通知
                    let switch_manager =
                        crate::proxy::failover_switch::FailoverSwitchManager::new(db.clone());
                    if let Err(e) = switch_manager
                        .try_switch(
                            Some(&app_handle),
                            &app_type,
                            &provider_id,
                            &provider_name,
                            None,
                        )
                        .await
                    {
                        log::error!("[Recovery] 自动切换失败: {e}");
                    }
                }
            }
        }
    }

    Ok(())
}

/// 获取熔断器配置
#[tauri::command]
pub async fn get_circuit_breaker_config(
    state: tauri::State<'_, AppState>,
) -> Result<CircuitBreakerConfig, String> {
    let db = &state.db;
    db.get_circuit_breaker_config()
        .await
        .map_err(|e| e.to_string())
}

/// 更新熔断器配置
#[tauri::command]
pub async fn update_circuit_breaker_config(
    state: tauri::State<'_, AppState>,
    config: CircuitBreakerConfig,
) -> Result<(), String> {
    let db = &state.db;

    // 1. 更新数据库配置
    db.update_circuit_breaker_config(&config)
        .await
        .map_err(|e| e.to_string())?;

    // 2. 如果代理正在运行，热更新内存中的熔断器配置
    state
        .proxy_service
        .update_circuit_breaker_configs(config)
        .await?;

    Ok(())
}

/// 获取熔断器统计信息（仅当代理服务器运行时）
#[tauri::command]
pub async fn get_circuit_breaker_stats(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    app_type: String,
) -> Result<Option<CircuitBreakerStats>, String> {
    require_proxy_app(&app_type)?;
    // 熔断器是纯内存态：代理未运行 / 该 provider 尚未产生流量时返回 None。
    state
        .proxy_service
        .get_provider_circuit_breaker_stats(&provider_id, &app_type)
        .await
}

/// 获取该应用当前处于限流冷却中的供应商列表
///
/// 冷却来自 429 Retry-After（或本地退避）：这些供应商仍可被尝试，
/// 但在路由选择阶段会被排到队列末尾。前端据此在卡片上显示倒计时。
#[tauri::command]
pub async fn get_rate_limit_cooldowns(
    state: tauri::State<'_, AppState>,
    app_type: String,
) -> Result<Vec<crate::proxy::rate_limit::CooldownSnapshot>, String> {
    require_proxy_app(&app_type)?;
    state
        .proxy_service
        .get_rate_limit_cooldowns(&app_type)
        .await
}

#[cfg(test)]
mod resilience_summary_tests {
    use super::{summarize_stream_checks, StreamCheckSummary};
    use crate::database::LatestStreamCheck;

    fn check(provider_id: &str, success: bool, latency_ms: Option<u64>) -> LatestStreamCheck {
        LatestStreamCheck {
            provider_id: provider_id.to_string(),
            success,
            response_time_ms: latency_ms,
        }
    }

    /// 没有任何检测记录时，所有指标都应是「无数据」而不是 0。
    ///
    /// 用 0 表示「无数据」会让 UI 显示出「平均延迟 0ms」这种误导性的数字。
    #[test]
    fn empty_input_reports_no_data() {
        assert_eq!(summarize_stream_checks(&[]), StreamCheckSummary::default());
    }

    /// 全部不可达时不应有延迟数据。
    #[test]
    fn all_unreachable_has_no_latency() {
        let summary =
            summarize_stream_checks(&[check("a", false, None), check("b", false, Some(50))]);

        assert_eq!(summary.unreachable_count, 2);
        assert_eq!(summary.avg_latency_ms, None, "不可达站点的延迟不应计入平均");
        assert_eq!(summary.slowest, None);
    }

    /// 延迟恰好为 0ms 是合法样本，必须参与计算而不是被当成缺失。
    #[test]
    fn zero_latency_counts_as_a_real_sample() {
        let summary = summarize_stream_checks(&[
            check("fast", true, Some(0)),
            check("slow", true, Some(100)),
        ]);

        assert_eq!(
            summary.avg_latency_ms,
            Some(50),
            "0ms 必须作为样本参与平均，否则会把 0 当缺失丢掉"
        );
        assert_eq!(
            summary.slowest,
            Some(("slow".to_string(), 100)),
            "最慢站点不应被 0ms 抢走"
        );
    }

    /// 可达但缺少延迟读数时，计入「可达」但不进延迟样本。
    #[test]
    fn reachable_without_latency_is_not_a_latency_sample() {
        let summary = summarize_stream_checks(&[
            check("no-reading", true, None),
            check("measured", true, Some(80)),
        ]);

        assert_eq!(summary.unreachable_count, 0);
        assert_eq!(summary.avg_latency_ms, Some(80));
        assert_eq!(summary.slowest, Some(("measured".to_string(), 80)));
    }

    /// 平均延迟向下取整，且不会因溢出 panic。
    #[test]
    fn average_is_integer_division_of_measured_samples() {
        let summary = summarize_stream_checks(&[
            check("a", true, Some(10)),
            check("b", true, Some(11)),
            check("c", true, Some(12)),
        ]);

        assert_eq!(summary.avg_latency_ms, Some(11));
    }

    /// 最慢站点取严格最大值；并列时保留先遇到的那个（稳定输出）。
    #[test]
    fn slowest_keeps_first_provider_on_ties() {
        let summary = summarize_stream_checks(&[
            check("first", true, Some(200)),
            check("second", true, Some(200)),
        ]);

        assert_eq!(summary.slowest, Some(("first".to_string(), 200)));
    }
}
