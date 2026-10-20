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

    /// 统计指定应用自 `since` 起的故障转移事件数。
    ///
    /// 走 SQL `COUNT(*)` 而不是「取最近 N 条再在内存里过滤」：后者有两个问题 ——
    /// 一是条数上限会把结果截断（今天转移 500 次却只显示 200），二是为了数一个
    /// 数字要把整批事件行读出来再丢弃。COUNT 在 `(app_type, created_at)` 索引上
    /// 直接出结果。
    pub fn count_failover_events_since(&self, app_type: &str, since: i64) -> Result<u64, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM failover_events
                 WHERE app_type = ?1 AND created_at >= ?2",
                rusqlite::params![app_type, since],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count.max(0) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 插入一条指定时间的事件（绕过 save_failover_event 的"当前时间"）。
    ///
    /// 直接拿锁而不是用 `lock_conn!`：该宏内部带 `?`，只能用于返回 Result 的函数，
    /// 而测试辅助函数用 panic 表达失败更直观。
    fn insert_event_at(db: &Database, app_type: &str, created_at: i64) {
        let conn = db.conn.lock().expect("lock db connection");
        conn.execute(
            "INSERT INTO failover_events
             (app_type, from_provider_id, from_provider_name,
              to_provider_id, to_provider_name, reason, created_at)
             VALUES (?1, 'from', 'From', 'to', 'To', 'provider_failed', ?2)",
            rusqlite::params![app_type, created_at],
        )
        .expect("insert failover event");
    }

    /// 清理只删过期记录，保留窗口内的事件。
    ///
    /// 这条路径此前从未被任何代码调用，`failover_events` 会随每次切换无限增长。
    #[test]
    fn cleanup_removes_only_events_older_than_retention_window() {
        let db = Database::memory().expect("memory db");
        let now = Utc::now().timestamp();
        let forty_days_ago = now - 40 * 86400;

        insert_event_at(&db, "claude", forty_days_ago);
        insert_event_at(&db, "claude", now - 3600);

        let deleted = db.cleanup_old_failover_events(30).expect("cleanup");

        assert_eq!(deleted, 1, "只应删除 40 天前的那条");
        let remaining = db.get_failover_events("claude", 10).expect("events");
        assert_eq!(remaining.len(), 1, "窗口内的事件必须保留");
    }

    /// 保留天数为 0 时不应误删"刚刚发生"的事件。
    #[test]
    fn cleanup_with_zero_retention_keeps_current_events() {
        let db = Database::memory().expect("memory db");
        insert_event_at(&db, "claude", Utc::now().timestamp());

        let deleted = db.cleanup_old_failover_events(0).expect("cleanup");

        assert_eq!(deleted, 0);
        assert_eq!(db.get_failover_events("claude", 10).unwrap().len(), 1);
    }

    /// 清理不跨 app 误伤，且事件按时间倒序返回。
    #[test]
    fn events_are_app_scoped_and_ordered_newest_first() {
        let db = Database::memory().expect("memory db");
        let now = Utc::now().timestamp();

        insert_event_at(&db, "claude", now - 300);
        insert_event_at(&db, "claude", now - 100);
        insert_event_at(&db, "codex", now - 200);

        let claude_events = db.get_failover_events("claude", 10).expect("claude events");
        assert_eq!(claude_events.len(), 2);
        assert!(
            claude_events[0].created_at > claude_events[1].created_at,
            "应按时间倒序返回"
        );
        assert!(claude_events.iter().all(|event| event.app_type == "claude"));
    }

    /// 今日计数不能被「取最近 N 条」的上限截断。
    ///
    /// 之前摘要用 `get_failover_events(app, 200)` 再在内存里过滤，今天转移
    /// 超过 200 次就会少报。COUNT 走 SQL 聚合，不受任何 limit 影响。
    #[test]
    fn count_since_is_not_capped_by_list_limit() {
        let db = Database::memory().expect("memory db");
        let now = Utc::now().timestamp();
        // 窗口起点要早于所有"今日"事件：250 条分布在 now-0 .. now-249
        let window_start = now - 1000;

        // 今日 250 条（超过旧的 200 上限）
        for offset in 0..250 {
            insert_event_at(&db, "claude", now - offset);
        }
        // 窗口外的一条不应计入
        insert_event_at(&db, "claude", window_start - 120);

        let counted = db
            .count_failover_events_since("claude", window_start)
            .expect("count");

        assert_eq!(counted, 250, "COUNT 不应被任何列表上限截断");
    }

    /// 计数按 app 隔离。
    #[test]
    fn count_since_is_app_scoped() {
        let db = Database::memory().expect("memory db");
        let now = Utc::now().timestamp();
        let since = now - 3600;

        insert_event_at(&db, "claude", now - 10);
        insert_event_at(&db, "claude", now - 20);
        insert_event_at(&db, "codex", now - 30);

        assert_eq!(db.count_failover_events_since("claude", since).unwrap(), 2);
        assert_eq!(db.count_failover_events_since("codex", since).unwrap(), 1);
        assert_eq!(
            db.count_failover_events_since("gemini", since).unwrap(),
            0,
            "没有任何事件的应用应为 0"
        );
    }
}
