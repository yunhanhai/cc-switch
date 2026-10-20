//! 流式健康检查日志 DAO

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::stream_check::{StreamCheckConfig, StreamCheckResult};

impl Database {
    /// 保存流式检查日志
    pub fn save_stream_check_log(
        &self,
        provider_id: &str,
        provider_name: &str,
        app_type: &str,
        result: &StreamCheckResult,
    ) -> Result<i64, AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "INSERT INTO stream_check_logs 
             (provider_id, provider_name, app_type, status, success, message, 
              response_time_ms, http_status, model_used, retry_count, tested_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                provider_id,
                provider_name,
                app_type,
                format!("{:?}", result.status).to_lowercase(),
                result.success,
                result.message,
                result.response_time_ms.map(|t| t as i64),
                result.http_status.map(|s| s as i64),
                result.model_used,
                result.retry_count as i64,
                result.tested_at,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(conn.last_insert_rowid())
    }

    /// 获取流式检查配置
    pub fn get_stream_check_config(&self) -> Result<StreamCheckConfig, AppError> {
        match self.get_setting("stream_check_config")? {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| AppError::Message(format!("解析配置失败: {e}"))),
            None => Ok(StreamCheckConfig::default()),
        }
    }

    /// Delete stream check logs older than `retain_days` days.
    /// Returns the number of deleted rows.
    pub fn cleanup_old_stream_check_logs(&self, retain_days: i64) -> Result<u64, AppError> {
        let cutoff = chrono::Utc::now().timestamp() - retain_days * 86400;
        let conn = lock_conn!(self.conn);
        let deleted = conn
            .execute(
                "DELETE FROM stream_check_logs WHERE tested_at < ?1",
                [cutoff],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        if deleted > 0 {
            log::info!("Cleaned up {deleted} stream_check_logs older than {retain_days} days");
        }
        Ok(deleted as u64)
    }

    /// 保存流式检查配置
    pub fn save_stream_check_config(&self, config: &StreamCheckConfig) -> Result<(), AppError> {
        let json = serde_json::to_string(config)
            .map_err(|e| AppError::Message(format!("序列化配置失败: {e}")))?;
        self.set_setting("stream_check_config", &json)
    }

    /// 查询指定应用下每个 Provider 最近一次连通性检测结果
    ///
    /// 用于前端按健康/延迟排序与"选中不可达"快捷操作。
    /// 返回 (provider_id, success, response_time_ms)；从未检测过的 Provider 不在结果中。
    ///
    /// 用 `MAX(id)` 而非 `MAX(tested_at)` 定位"最近一次"：`tested_at` 是秒级时间戳，
    /// 同一 Provider 在同一秒内被检测两次（连点检测按钮、批量检测与单点检测重叠）时，
    /// 按时间关联会把两行都匹配上，返回重复的 provider 记录 —— 下游按行累加的
    /// "不可达数""平均延迟"会被同一站点重复计数。`id` 是 AUTOINCREMENT，严格单调，
    /// 天然给出唯一的最新行。
    pub fn get_latest_stream_checks(
        &self,
        app_type: &str,
    ) -> Result<Vec<LatestStreamCheck>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn.prepare(
            "SELECT provider_id, success, response_time_ms
             FROM stream_check_logs
             WHERE id IN (
                 SELECT MAX(id)
                 FROM stream_check_logs
                 WHERE app_type = ?1
                 GROUP BY provider_id
             )",
        )?;
        let rows = stmt.query_map([app_type], |row| {
            Ok(LatestStreamCheck {
                provider_id: row.get(0)?,
                success: row.get::<_, i64>(1)? != 0,
                response_time_ms: row.get::<_, Option<i64>>(2)?.map(|v| v as u64),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }
}

/// Provider 最近一次连通性检测结果摘要
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestStreamCheck {
    pub provider_id: String,
    pub success: bool,
    pub response_time_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::stream_check::HealthStatus;

    fn result_at(success: bool, latency_ms: Option<u64>, tested_at: i64) -> StreamCheckResult {
        StreamCheckResult {
            status: if success {
                HealthStatus::Operational
            } else {
                HealthStatus::Failed
            },
            success,
            message: if success { "ok" } else { "unreachable" }.to_string(),
            response_time_ms: latency_ms,
            http_status: success.then_some(200),
            model_used: String::new(),
            tested_at,
            retry_count: 0,
            error_category: None,
        }
    }

    /// 同一 Provider 在同一秒内被检测两次时，只能返回一行（最新那次）。
    ///
    /// 这是 `MAX(tested_at)` 关联的经典陷阱：`tested_at` 只有秒级精度，
    /// 两次检测落在同一秒会让 JOIN 命中两行，下游按行累加的统计就会把
    /// 同一站点重复计数（不可达数虚高、平均延迟被重复样本带偏）。
    #[test]
    fn latest_check_returns_one_row_per_provider_within_same_second() {
        let db = Database::memory().expect("memory db");
        let same_second = 1_700_000_000i64;

        // 同一秒内的两次检测：先失败、后成功
        db.save_stream_check_log(
            "provider-a",
            "Provider A",
            "claude",
            &result_at(false, None, same_second),
        )
        .expect("save first check");
        db.save_stream_check_log(
            "provider-a",
            "Provider A",
            "claude",
            &result_at(true, Some(120), same_second),
        )
        .expect("save second check");

        let latest = db
            .get_latest_stream_checks("claude")
            .expect("latest checks");

        assert_eq!(latest.len(), 1, "同一 Provider 同一秒只应返回一行");
        assert_eq!(latest[0].provider_id, "provider-a");
        assert!(latest[0].success, "应取后写入的那次（成功）");
        assert_eq!(latest[0].response_time_ms, Some(120));
    }

    /// 多个 Provider 各自只返回最新一次，且互不干扰。
    #[test]
    fn latest_check_is_per_provider_and_app_scoped() {
        let db = Database::memory().expect("memory db");

        db.save_stream_check_log("a", "A", "claude", &result_at(true, Some(10), 100))
            .expect("save a old");
        db.save_stream_check_log("a", "A", "claude", &result_at(true, Some(20), 200))
            .expect("save a new");
        db.save_stream_check_log("b", "B", "claude", &result_at(false, None, 150))
            .expect("save b");
        // 同名 provider 在另一个 app 下的记录不应混入 claude 的结果
        db.save_stream_check_log("a", "A", "codex", &result_at(false, None, 300))
            .expect("save codex a");

        let mut latest = db
            .get_latest_stream_checks("claude")
            .expect("latest claude");
        latest.sort_by(|left, right| left.provider_id.cmp(&right.provider_id));

        assert_eq!(latest.len(), 2);
        assert_eq!(latest[0].provider_id, "a");
        assert_eq!(latest[0].response_time_ms, Some(20), "应取 a 的最新一次");
        assert_eq!(latest[1].provider_id, "b");
        assert!(!latest[1].success);
    }
}
