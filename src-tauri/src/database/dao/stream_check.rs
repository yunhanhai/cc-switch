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
    pub fn get_latest_stream_checks(
        &self,
        app_type: &str,
    ) -> Result<Vec<LatestStreamCheck>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn.prepare(
            "SELECT l.provider_id, l.success, l.response_time_ms
             FROM stream_check_logs l
             INNER JOIN (
                 SELECT provider_id, MAX(tested_at) AS max_tested
                 FROM stream_check_logs
                 WHERE app_type = ?1
                 GROUP BY provider_id
             ) latest
               ON l.provider_id = latest.provider_id
              AND l.tested_at = latest.max_tested
             WHERE l.app_type = ?1",
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
