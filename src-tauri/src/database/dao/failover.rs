//! 故障转移队列 DAO
//!
//! 管理代理模式下的故障转移队列（基于 providers 表的 in_failover_queue 字段）

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::provider::Provider;
use serde::{Deserialize, Serialize};

/// 故障转移队列条目（简化版，用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailoverQueueItem {
    pub provider_id: String,
    pub provider_name: String,
    pub sort_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_notes: Option<String>,
}

impl Database {
    /// 获取故障转移队列（按 sort_index 排序）
    pub fn get_failover_queue(&self, app_type: &str) -> Result<Vec<FailoverQueueItem>, AppError> {
        let conn = lock_conn!(self.conn);

        let mut stmt = conn
            .prepare(
                "SELECT id, name, sort_index, notes
                 FROM providers
                 WHERE app_type = ?1 AND in_failover_queue = 1
                 ORDER BY COALESCE(sort_index, 999999), id ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let items = stmt
            .query_map([app_type], |row| {
                Ok(FailoverQueueItem {
                    provider_id: row.get(0)?,
                    provider_name: row.get(1)?,
                    sort_index: row.get(2)?,
                    provider_notes: row.get(3)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(items)
    }

    /// 获取故障转移队列中的供应商（完整 Provider 信息，按顺序）
    pub fn get_failover_providers(&self, app_type: &str) -> Result<Vec<Provider>, AppError> {
        let all_providers = self.get_all_providers(app_type)?;

        let result: Vec<Provider> = all_providers
            .into_values()
            .filter(|p| p.in_failover_queue)
            .collect();

        Ok(result)
    }

    /// 添加供应商到故障转移队列（兼容期双写到 default 路由分组）。
    pub fn add_to_failover_queue(&self, app_type: &str, provider_id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "UPDATE providers SET in_failover_queue = 1 WHERE id = ?1 AND app_type = ?2",
            rusqlite::params![provider_id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 兼容期：同步到 default 分组（归属制搬运 + 追加）。失败不阻塞主流程，仅日志。
        if let Err(e) = Self::sync_failover_add_to_default_group(&conn, app_type, provider_id) {
            log::warn!("[{app_type}] 同步故障转移添加 {provider_id} 到 default 分组失败: {e}");
        }
        Ok(())
    }

    fn sync_failover_add_to_default_group(
        conn: &rusqlite::Connection,
        app_type: &str,
        provider_id: &str,
    ) -> Result<(), AppError> {
        let group_id = format!("rg_default_{app_type}");
        // 若 routing_groups 表尚不存在（旧库未迁移），忽略
        let has_table: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='routing_groups'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !has_table {
            return Ok(());
        }
        // 确保 default 分组存在
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT OR IGNORE INTO routing_groups
             (id, app_type, name, sort_order, enabled, created_at, updated_at)
             VALUES (?1, ?2, 'default', 0, 1, ?3, ?3)",
            rusqlite::params![group_id, app_type, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        // 归属制搬运：从其他分组移出
        Self::detach_from_other_groups(conn, provider_id, app_type, &group_id)?;
        // 已在 default 则不变，否则追加
        let already: bool = conn
            .query_row(
                "SELECT 1 FROM routing_group_members WHERE group_id = ?1 AND provider_id = ?2",
                rusqlite::params![group_id, provider_id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if already {
            return Ok(());
        }
        let max_idx: Option<i64> = conn
            .query_row(
                "SELECT MAX(sort_index) FROM routing_group_members WHERE group_id = ?1",
                rusqlite::params![group_id],
                |row| row.get(0),
            )
            .unwrap_or(None);
        let next = max_idx.map(|m| m + 1).unwrap_or(0);
        conn.execute(
            "INSERT OR IGNORE INTO routing_group_members
             (group_id, provider_id, app_type, sort_index, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![group_id, provider_id, app_type, next, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    fn detach_from_other_groups(
        conn: &rusqlite::Connection,
        provider_id: &str,
        app_type: &str,
        keep_group_id: &str,
    ) -> Result<(), AppError> {
        conn.execute(
            "DELETE FROM routing_group_members
             WHERE provider_id = ?1 AND app_type = ?2 AND group_id != ?3",
            rusqlite::params![provider_id, app_type, keep_group_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 从故障转移队列中移除供应商（兼容期同步移除 default 分组成员）。
    pub fn remove_from_failover_queue(
        &self,
        app_type: &str,
        provider_id: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        // 1. 从队列中移除
        conn.execute(
            "UPDATE providers SET in_failover_queue = 0 WHERE id = ?1 AND app_type = ?2",
            rusqlite::params![provider_id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 兼容期：同步移除 default 分组成员（忽略错误）
        let _ = conn.execute(
            "DELETE FROM routing_group_members WHERE group_id = ?1 AND provider_id = ?2",
            rusqlite::params![format!("rg_default_{app_type}"), provider_id],
        );

        // 2. 清除该供应商的健康状态（退出队列后不再需要健康监控）
        conn.execute(
            "DELETE FROM provider_health WHERE provider_id = ?1 AND app_type = ?2",
            rusqlite::params![provider_id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        log::info!("已从故障转移队列移除供应商 {provider_id} ({app_type}), 并清除其健康状态");

        Ok(())
    }

    /// 清空故障转移队列
    pub fn clear_failover_queue(&self, app_type: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "UPDATE providers SET in_failover_queue = 0 WHERE app_type = ?1",
            [app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 检查供应商是否在故障转移队列中
    pub fn is_in_failover_queue(
        &self,
        app_type: &str,
        provider_id: &str,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);

        let in_queue: bool = conn
            .query_row(
                "SELECT in_failover_queue FROM providers WHERE id = ?1 AND app_type = ?2",
                rusqlite::params![provider_id, app_type],
                |row| row.get(0),
            )
            .unwrap_or(false);

        Ok(in_queue)
    }

    /// 获取可添加到故障转移队列的供应商（不在队列中的）
    pub fn get_available_providers_for_failover(
        &self,
        app_type: &str,
    ) -> Result<Vec<Provider>, AppError> {
        let all_providers = self.get_all_providers(app_type)?;

        let available: Vec<Provider> = all_providers
            .into_values()
            .filter(|p| !p.in_failover_queue)
            .collect();

        Ok(available)
    }
}
