//! 故障转移事件日志 DAO
//!
//! 记录每次自动故障转移的来源/目标/原因，供前端时间线展示。

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use chrono::Utc;
use serde::Serialize;

/// 故障转移事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailoverEvent {
    pub id: i64,
    pub app_type: String,
    pub from_provider_id: Option<String>,
    pub from_provider_name: Option<String>,
    pub to_provider_id: String,
    pub to_provider_name: String,
    /// billing_exhausted | rate_limited | provider_failed | manual
    pub reason: Option<String>,
    pub created_at: i64,
}

impl Database {
    /// 保存一次故障转移事件
    pub fn save_failover_event(
        &self,
        app_type: &str,
        from_provider_id: Option<&str>,
        from_provider_name: Option<&str>,
        to_provider_id: &str,
        to_provider_name: &str,
        reason: Option<&str>,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO failover_events
             (app_type, from_provider_id, from_provider_name, to_provider_id, to_provider_name, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                app_type,
                from_provider_id,
                from_provider_name,
                to_provider_id,
                to_provider_name,
                reason,
                Utc::now().timestamp(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 查询指定应用最近的故障转移事件（时间倒序）
    pub fn get_failover_events(
        &self,
        app_type: &str,
        limit: u32,
    ) -> Result<Vec<FailoverEvent>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn.prepare(
            "SELECT id, app_type, from_provider_id, from_provider_name,
                    to_provider_id, to_provider_name, reason, created_at
             FROM failover_events
             WHERE app_type = ?1
             ORDER BY created_at DESC, id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![app_type, limit], |row| {
            Ok(FailoverEvent {
                id: row.get(0)?,
                app_type: row.get(1)?,
                from_provider_id: row.get(2)?,
                from_provider_name: row.get(3)?,
                to_provider_id: row.get(4)?,
                to_provider_name: row.get(5)?,
                reason: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// 清理超过保留天数的事件（随备份清理流程调用）
    pub fn cleanup_old_failover_events(&self, retain_days: i64) -> Result<u64, AppError> {
        let cutoff = Utc::now().timestamp() - retain_days * 86400;
        let conn = lock_conn!(self.conn);
        let deleted = conn
            .execute(
                "DELETE FROM failover_events WHERE created_at < ?1",
                [cutoff],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(deleted as u64)
    }
}
