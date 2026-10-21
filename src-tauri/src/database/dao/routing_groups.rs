//! Routing Groups DAO（统一网关抽象 + smart 路由分组）
//!
//! 每 app 可拥有多组路由分组，分组成员为该 app 的 provider 子集（归属制）。
//! 兼容期提供 `sync_default_from_failover_queue` 把旧的 in_failover_queue
//! 镜像到 default 分组，保持故障转移链路不变。

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutingGroup {
    pub id: String,
    pub app_type: String,
    pub name: String,
    pub sort_order: i64,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingGroupWithMembers {
    pub group: RoutingGroup,
    pub members: Vec<RoutingGroupMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutingGroupMember {
    pub group_id: String,
    pub provider_id: String,
    pub app_type: String,
    pub sort_index: i64,
    pub added_at: i64,
}

impl Database {
    fn validate_routing_group_name(name: &str) -> Result<String, AppError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput("路由分组名称不能为空".to_string()));
        }
        if trimmed.len() > 64 {
            return Err(AppError::InvalidInput(
                "路由分组名称不能超过 64 个字符".to_string(),
            ));
        }
        Ok(trimmed.to_string())
    }

    fn is_valid_app_type(app_type: &str) -> bool {
        matches!(app_type, "claude" | "codex" | "gemini" | "grokbuild")
    }

    pub fn list_routing_groups(
        &self,
        app_type: &str,
    ) -> Result<Vec<RoutingGroupWithMembers>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT id, app_type, name, sort_order, enabled, created_at, updated_at
                 FROM routing_groups WHERE app_type = ?1
                 ORDER BY sort_order, created_at, id",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let groups: Vec<RoutingGroup> = stmt
            .query_map(params![app_type], |row| {
                Ok(RoutingGroup {
                    id: row.get(0)?,
                    app_type: row.get(1)?,
                    name: row.get(2)?,
                    sort_order: row.get(3)?,
                    enabled: row.get::<_, i32>(4)? != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut out = Vec::with_capacity(groups.len());
        for group in groups {
            let mut m_stmt = conn
                .prepare(
                    "SELECT group_id, provider_id, app_type, sort_index, added_at
                     FROM routing_group_members WHERE group_id = ?1
                     ORDER BY sort_index, added_at, provider_id",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let members = m_stmt
                .query_map(params![group.id], |row| {
                    Ok(RoutingGroupMember {
                        group_id: row.get(0)?,
                        provider_id: row.get(1)?,
                        app_type: row.get(2)?,
                        sort_index: row.get(3)?,
                        added_at: row.get(4)?,
                    })
                })
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?;
            out.push(RoutingGroupWithMembers { group, members });
        }
        Ok(out)
    }

    pub fn create_routing_group(
        &self,
        app_type: &str,
        name: &str,
        sort_order: Option<i64>,
    ) -> Result<RoutingGroup, AppError> {
        if !Self::is_valid_app_type(app_type) {
            return Err(AppError::InvalidInput(format!(
                "不支持的应用类型: {app_type}"
            )));
        }
        let name = Self::validate_routing_group_name(name)?;
        let conn = lock_conn!(self.conn);
        let next_order: i64 = if let Some(v) = sort_order {
            v
        } else {
            let max: Option<i64> = conn
                .query_row(
                    "SELECT MAX(sort_order) FROM routing_groups WHERE app_type = ?1",
                    params![app_type],
                    |row| row.get(0),
                )
                .unwrap_or(None);
            max.map(|m| m + 1).unwrap_or(0)
        };
        let now = chrono::Utc::now().timestamp();
        let id = format!("rg_{}", uuid::Uuid::new_v4());
        conn.execute(
            "INSERT INTO routing_groups
             (id, app_type, name, sort_order, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
            params![id, app_type, name, next_order, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(RoutingGroup {
            id,
            app_type: app_type.to_string(),
            name,
            sort_order: next_order,
            enabled: true,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn update_routing_group(
        &self,
        id: &str,
        name: Option<&str>,
        enabled: Option<bool>,
        sort_order: Option<i64>,
    ) -> Result<RoutingGroup, AppError> {
        let conn = lock_conn!(self.conn);
        let existing = conn
            .query_row(
                "SELECT id, app_type, name, sort_order, enabled, created_at, updated_at
                 FROM routing_groups WHERE id = ?1",
                params![id],
                |row| {
                    Ok(RoutingGroup {
                        id: row.get(0)?,
                        app_type: row.get(1)?,
                        name: row.get(2)?,
                        sort_order: row.get(3)?,
                        enabled: row.get::<_, i32>(4)? != 0,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::InvalidInput(format!("路由分组不存在: {id}"))
                }
                other => AppError::Database(other.to_string()),
            })?;
        let new_name = if let Some(n) = name {
            Self::validate_routing_group_name(n)?
        } else {
            existing.name.clone()
        };
        let new_enabled = enabled.unwrap_or(existing.enabled);
        let new_order = sort_order.unwrap_or(existing.sort_order);
        let now = chrono::Utc::now().timestamp();
        let enabled_changed = new_enabled != existing.enabled;
        conn.execute(
            "UPDATE routing_groups SET name = ?1, enabled = ?2, sort_order = ?3, updated_at = ?4 WHERE id = ?5",
            params![new_name, if new_enabled { 1 } else { 0 }, new_order, now, id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        if enabled_changed {
            // 兼容期：启用态变更反向同步 in_failover_queue，保证旧队列视图与分组聚合一致
            let _ =
                Self::sync_in_failover_queue_from_enabled_locked(&conn, &existing.app_type);
        }
        Ok(RoutingGroup {
            id: existing.id,
            app_type: existing.app_type,
            name: new_name,
            sort_order: new_order,
            enabled: new_enabled,
            created_at: existing.created_at,
            updated_at: now,
        })
    }

    pub fn delete_routing_group(&self, id: &str) -> Result<(), AppError> {
        // 删除前取 app_type 用于反向同步
        let app_type_hint: Option<String> = {
            let conn = lock_conn!(self.conn);
            conn.query_row(
                "SELECT app_type FROM routing_groups WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok()
        };
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute("DELETE FROM routing_groups WHERE id = ?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        if affected == 0 {
            return Err(AppError::InvalidInput(format!("路由分组不存在: {id}")));
        }
        if let Some(app_type) = app_type_hint {
            let _ = Self::sync_in_failover_queue_from_enabled_locked(&conn, &app_type);
        }
        Ok(())
    }

    pub fn set_routing_group_members(
        &self,
        group_id: &str,
        provider_ids: &[String],
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let app_type: String = conn
            .query_row(
                "SELECT app_type FROM routing_groups WHERE id = ?1",
                params![group_id],
                |row| row.get(0),
            )
            .map_err(|_| AppError::InvalidInput(format!("路由分组不存在: {group_id}")))?;

        // 校验所有 provider 均属同一 app_type 且归属制：已被其他分组占用的拒绝
        for pid in provider_ids {
            let exists: Option<String> = conn
                .query_row(
                    "SELECT id FROM providers WHERE id = ?1 AND app_type = ?2",
                    params![pid, app_type],
                    |row| row.get(0),
                )
                .ok();
            if exists.is_none() {
                return Err(AppError::InvalidInput(format!(
                    "供应商不存在: {pid} ({app_type})"
                )));
            }
            // 归属制：同 app 下已被其他分组占用的 provider 拒绝重复加入
            let conflict: Option<String> = conn
                .query_row(
                    "SELECT group_id FROM routing_group_members
                     WHERE provider_id = ?1 AND app_type = ?2 AND group_id != ?3 LIMIT 1",
                    params![pid, app_type, group_id],
                    |row| row.get(0),
                )
                .ok();
            if let Some(other) = conflict {
                return Err(AppError::InvalidInput(format!(
                    "供应商 {pid} 已归属分组 {other}，同 app 下不可重复归属"
                )));
            }
        }

        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "DELETE FROM routing_group_members WHERE group_id = ?1",
            params![group_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        for (idx, pid) in provider_ids.iter().enumerate() {
            conn.execute(
                "INSERT INTO routing_group_members
                 (group_id, provider_id, app_type, sort_index, added_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![group_id, pid, app_type, idx as i64, now],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        conn.execute(
            "UPDATE routing_groups SET updated_at = ?1 WHERE id = ?2",
            params![now, group_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        // 兼容期反向同步：覆盖式写入时 enabled 聚合的归属已改变，旧队列标记需一致
        let _ = Self::sync_in_failover_queue_from_enabled_locked(&conn, &app_type);
        Ok(())
    }

    pub fn add_routing_group_member(
        &self,
        group_id: &str,
        provider_id: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let app_type: String = conn
            .query_row(
                "SELECT app_type FROM routing_groups WHERE id = ?1",
                params![group_id],
                |row| row.get(0),
            )
            .map_err(|_| AppError::InvalidInput(format!("路由分组不存在: {group_id}")))?;
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM providers WHERE id = ?1 AND app_type = ?2",
                params![provider_id, app_type],
                |row| row.get(0),
            )
            .ok();
        if exists.is_none() {
            return Err(AppError::InvalidInput(format!(
                "供应商不存在: {provider_id} ({app_type})"
            )));
        }
        // 归属制：检查是否已被其他分组占用
        let conflict: Option<String> = conn
            .query_row(
                "SELECT group_id FROM routing_group_members
                 WHERE provider_id = ?1 AND app_type = ?2 AND group_id != ?3 LIMIT 1",
                params![provider_id, app_type, group_id],
                |row| row.get(0),
            )
            .ok();
        if let Some(other) = conflict {
            return Err(AppError::InvalidInput(format!(
                "供应商 {provider_id} 已归属分组 {other}，同 app 下不可重复归属"
            )));
        }
        let max_idx: Option<i64> = conn
            .query_row(
                "SELECT MAX(sort_index) FROM routing_group_members WHERE group_id = ?1",
                params![group_id],
                |row| row.get(0),
            )
            .unwrap_or(None);
        let next = max_idx.map(|m| m + 1).unwrap_or(0);
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT OR IGNORE INTO routing_group_members
             (group_id, provider_id, app_type, sort_index, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![group_id, provider_id, app_type, next, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        // 单条追加：若所属分组已启用，聚合到旧队列的可见集需更新
        let _ = Self::sync_in_failover_queue_from_enabled_locked(&conn, &app_type);
        Ok(())
    }

    pub fn remove_routing_group_member(
        &self,
        group_id: &str,
        provider_id: &str,
    ) -> Result<(), AppError> {
        let app_type: Option<String> = {
            let conn = lock_conn!(self.conn);
            conn.query_row(
                "SELECT app_type FROM routing_groups WHERE id = ?1",
                params![group_id],
                |row| row.get(0),
            )
            .ok()
        };
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM routing_group_members WHERE group_id = ?1 AND provider_id = ?2",
            params![group_id, provider_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        if let Some(app_type) = app_type {
            let _ = Self::sync_in_failover_queue_from_enabled_locked(&conn, &app_type);
        }
        Ok(())
    }

    pub fn get_routing_group(&self, id: &str) -> Result<Option<RoutingGroupWithMembers>, AppError> {
        let conn = lock_conn!(self.conn);
        let group = match conn.query_row(
            "SELECT id, app_type, name, sort_order, enabled, created_at, updated_at
             FROM routing_groups WHERE id = ?1",
            params![id],
            |row| {
                Ok(RoutingGroup {
                    id: row.get(0)?,
                    app_type: row.get(1)?,
                    name: row.get(2)?,
                    sort_order: row.get(3)?,
                    enabled: row.get::<_, i32>(4)? != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        ) {
            Ok(g) => g,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(AppError::Database(e.to_string())),
        };
        let mut stmt = conn
            .prepare(
                "SELECT group_id, provider_id, app_type, sort_index, added_at
                 FROM routing_group_members WHERE group_id = ?1
                 ORDER BY sort_index, added_at, provider_id",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let members = stmt
            .query_map(params![id], |row| {
                Ok(RoutingGroupMember {
                    group_id: row.get(0)?,
                    provider_id: row.get(1)?,
                    app_type: row.get(2)?,
                    sort_index: row.get(3)?,
                    added_at: row.get(4)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(Some(RoutingGroupWithMembers { group, members }))
    }

    /// 批量重排同一 app 下的分组顺序（拖拽排序）。
    ///
    /// `ordered_ids` 为该 app 下分组 id 的全量新顺序；依次写入 `sort_order = index`。
    /// 仅允许对同一 `app_type` 的分组重排，跨 app 的 id 将报错。
    pub fn reorder_routing_groups(
        &self,
        app_type: &str,
        ordered_ids: &[String],
    ) -> Result<(), AppError> {
        if !Self::is_valid_app_type(app_type) {
            return Err(AppError::InvalidInput(format!(
                "不支持的应用类型: {app_type}"
            )));
        }
        if ordered_ids.is_empty() {
            return Ok(());
        }
        let conn = lock_conn!(self.conn);
        // 校验：所有 id 均属同一 app_type
        for id in ordered_ids {
            let owner: Option<String> = conn
                .query_row(
                    "SELECT app_type FROM routing_groups WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .ok();
            match owner {
                Some(t) if t == app_type => {}
                Some(other) => {
                    return Err(AppError::InvalidInput(format!(
                        "分组 {id} 不属于 {app_type}（实际 {other}）"
                    )))
                }
                _ => {
                    return Err(AppError::InvalidInput(format!("路由分组不存在: {id}")))
                }
            }
        }
        let now = chrono::Utc::now().timestamp();
        for (idx, id) in ordered_ids.iter().enumerate() {
            conn.execute(
                "UPDATE routing_groups SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
                params![idx as i64, now, id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// 批量重排单个分组内的成员顺序（拖拽排序）。
    ///
    /// `ordered_provider_ids` 为该分组当前成员的全量新顺序；
    /// 非成员或重复 id 将报错。
    pub fn reorder_routing_group_members(
        &self,
        group_id: &str,
        ordered_provider_ids: &[String],
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let app_type: String = conn
            .query_row(
                "SELECT app_type FROM routing_groups WHERE id = ?1",
                params![group_id],
                |row| row.get(0),
            )
            .map_err(|_| AppError::InvalidInput(format!("路由分组不存在: {group_id}")))?;

        // 读取当前成员集合校验
        let mut stmt = conn
            .prepare(
                "SELECT provider_id FROM routing_group_members WHERE group_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let current: Vec<String> = stmt
            .query_map(params![group_id], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let current_set: std::collections::HashSet<String> =
            current.into_iter().collect();
        if ordered_provider_ids.len() != current_set.len() {
            return Err(AppError::InvalidInput(
                "成员重排需提供全量成员且不重复".to_string(),
            ));
        }
        for pid in ordered_provider_ids {
            if !current_set.contains(pid) {
                return Err(AppError::InvalidInput(format!(
                    "供应商 {pid} 不在分组 {group_id} 中"
                )));
            }
        }
        for (idx, pid) in ordered_provider_ids.iter().enumerate() {
            conn.execute(
                "UPDATE routing_group_members SET sort_index = ?1 WHERE group_id = ?2 AND provider_id = ?3",
                params![idx as i64, group_id, pid],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE routing_groups SET updated_at = ?1 WHERE id = ?2",
            params![now, group_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        // 成员顺序变更不影响 failover 归属，仅顺序，同步 in_failover_queue 的顺序不由该字段决定
        // 故无需反向同步；但为保持 P1 语义与旧队列拖拽联动，仍可触发轻量同步（仅 enabled 分组聚合时顺序无关）
        let _ = Self::sync_in_failover_queue_from_enabled_locked(&conn, &app_type);
        Ok(())
    }

    /// 将已启用分组的聚合成员反向同步到 `providers.in_failover_queue`（兼容期）。
    ///
    /// 启用分组为空时清空旧队列标记；否则旧队列标记与启用分组聚合一致。
    /// 失败不抛，仅日志——调用方已在同一锁内，不应因兼容同步失败而阻断主流程。
    fn sync_in_failover_queue_from_enabled_locked(
        conn: &rusqlite::Connection,
        app_type: &str,
    ) -> Result<(), AppError> {
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
        let enabled_ids = {
            let mut stmt = conn
                .prepare(
                    "SELECT rgm.provider_id FROM routing_groups rg
                     JOIN routing_group_members rgm ON rgm.group_id = rg.id
                     WHERE rg.app_type = ?1 AND rg.enabled = 1",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let ids = stmt
                .query_map(params![app_type], |row| row.get::<_, String>(0))
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?;
            ids
        };
        if enabled_ids.is_empty() {
            conn.execute(
                "UPDATE providers SET in_failover_queue = 0 WHERE app_type = ?1",
                params![app_type],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            return Ok(());
        }
        // 先清后设，避免残留
        conn.execute(
            "UPDATE providers SET in_failover_queue = 0 WHERE app_type = ?1",
            params![app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        let placeholders = enabled_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "UPDATE providers SET in_failover_queue = 1 WHERE app_type = ? AND id IN ({placeholders})"
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::with_capacity(enabled_ids.len() + 1);
        params_vec.push(Box::new(app_type.to_string()));
        for id in &enabled_ids {
            params_vec.push(Box::new(id.clone()));
        }
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref() as &dyn rusqlite::ToSql).collect();
        conn.execute(&sql, params_refs.as_slice())
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 确保该 app 的 default 分组存在（幂等），供旧故障转移队列双写复用。
    pub fn ensure_default_routing_group(&self, app_type: &str) -> Result<RoutingGroup, AppError> {
        if !Self::is_valid_app_type(app_type) {
            return Err(AppError::InvalidInput(format!(
                "不支持的应用类型: {app_type}"
            )));
        }
        let conn = lock_conn!(self.conn);
        let group_id = format!("rg_default_{app_type}");
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT OR IGNORE INTO routing_groups
             (id, app_type, name, sort_order, enabled, created_at, updated_at)
             VALUES (?1, ?2, 'default', 0, 1, ?3, ?3)",
            rusqlite::params![group_id, app_type, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        let group = conn
            .query_row(
                "SELECT id, app_type, name, sort_order, enabled, created_at, updated_at
                 FROM routing_groups WHERE id = ?1",
                rusqlite::params![group_id],
                |row| {
                    Ok(RoutingGroup {
                        id: row.get(0)?,
                        app_type: row.get(1)?,
                        name: row.get(2)?,
                        sort_order: row.get(3)?,
                        enabled: row.get::<_, i32>(4)? != 0,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(group)
    }

    /// 已启用分组拼接的有序 provider id 列表（数据面使用）。
    ///
    /// 启用分组按 sort_order/created_at/id 排序，组内成员按 sort_index/added_at/provider_id 排序。
    /// 无启用分组或成员为空时返回空，调用方回退到旧的 in_failover_queue 队列。
    pub fn get_enabled_routing_group_provider_ids(
        &self,
        app_type: &str,
    ) -> Result<Vec<String>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT rgm.provider_id
                 FROM routing_groups rg
                 JOIN routing_group_members rgm ON rgm.group_id = rg.id
                 WHERE rg.app_type = ?1 AND rg.enabled = 1
                 ORDER BY rg.sort_order, rg.created_at, rg.id,
                          rgm.sort_index, rgm.added_at, rgm.provider_id",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let ids = stmt
            .query_map(params![app_type], |row| row.get::<_, String>(0))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::provider::Provider;
    use serde_json::json;

    fn provider(id: &str, name: &str) -> Provider {
        Provider::with_id(
            id.to_string(),
            name.to_string(),
            json!({"env": {"ANTHROPIC_BASE_URL": "https://api.example.com"}}),
            None,
        )
    }

    #[test]
    fn test_routing_group_crud_and_members() -> Result<(), AppError> {
        let db = Database::memory()?;
        db.save_provider("claude", &provider("p1", "P1"))?;
        db.save_provider("claude", &provider("p2", "P2"))?;
        db.save_provider("claude", &provider("p3", "P3"))?;

        let g = db.create_routing_group("claude", "default", None)?;
        assert_eq!(g.name, "default");
        assert!(g.enabled);

        db.set_routing_group_members(&g.id, &["p1".to_string(), "p2".to_string()])?;
        let listed = db.list_routing_groups("claude")?;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].members.len(), 2);

        // 归属制：同 app 下重复归属拒绝
        let g2 = db.create_routing_group("claude", "second", None)?;
        let err = db
            .set_routing_group_members(&g2.id, &["p1".to_string()])
            .unwrap_err();
        assert!(err.to_string().contains("已归属"), "{err:?}");

        // 增量添加与移除
        db.add_routing_group_member(&g.id, "p3")?;
        let got = db.get_routing_group(&g.id)?.unwrap();
        assert_eq!(got.members.len(), 3);
        db.remove_routing_group_member(&g.id, "p2")?;
        let got = db.get_routing_group(&g.id)?.unwrap();
        assert_eq!(got.members.len(), 2);
        assert!(got.members.iter().any(|m| m.provider_id == "p1"));
        assert!(got.members.iter().any(|m| m.provider_id == "p3"));

        // 更新与删除
        let updated = db.update_routing_group(&g.id, Some("primary"), Some(false), None)?;
        assert_eq!(updated.name, "primary");
        assert!(!updated.enabled);
        db.delete_routing_group(&g2.id)?;
        assert_eq!(db.list_routing_groups("claude")?.len(), 1);
        Ok(())
    }

    #[test]
    fn test_reorder_groups_and_members_syncs_failover_queue() -> Result<(), AppError> {
        let db = Database::memory()?;
        for id in ["p1", "p2", "p3", "p4"] {
            db.save_provider("claude", &provider(id, id))?;
        }
        let fast = db.create_routing_group("claude", "fast", None)?;
        let slow = db.create_routing_group("claude", "slow", None)?;
        db.set_routing_group_members(&fast.id, &["p1".to_string(), "p2".to_string()])?;
        db.set_routing_group_members(&slow.id, &["p3".to_string()])?;

        // 启用分组聚合写入 in_failover_queue：此时 enabled 聚合为 p1,p2,p3
        let queue = db.get_failover_queue("claude")?;
        let ids: Vec<_> = queue.into_iter().map(|item| item.provider_id).collect();
        assert!(ids.contains(&"p1".to_string()));
        assert!(ids.contains(&"p2".to_string()));
        assert!(ids.contains(&"p3".to_string()));
        assert!(!ids.contains(&"p4".to_string()));

        // 组间重排：slow 提到最前，生效路由仍为 3 个成员，队列语义不变
        db.reorder_routing_groups("claude", &[slow.id.clone(), fast.id.clone()])?;
        let enabled_ids = db.get_enabled_routing_group_provider_ids("claude")?;
        assert_eq!(enabled_ids, vec!["p3", "p1", "p2"]);
        // 组内重排：fast 内 p2 提到 p1 前
        db.reorder_routing_group_members(&fast.id, &["p2".to_string(), "p1".to_string()])?;
        let enabled_ids = db.get_enabled_routing_group_provider_ids("claude")?;
        assert_eq!(enabled_ids, vec!["p3", "p2", "p1"]);

        // 成员重排会触发 enabled 聚合反向同步，in_failover_queue 仍与聚合一致
        let queue = db.get_failover_queue("claude")?;
        let ids: std::collections::HashSet<_> =
            queue.into_iter().map(|item| item.provider_id).collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains("p1"));
        assert!(ids.contains("p2"));
        assert!(ids.contains("p3"));

        // 禁用 fast 分组后，仅 slow 成员保留在队列
        db.update_routing_group(&fast.id, None, Some(false), None)?;
        let queue = db.get_failover_queue("claude")?;
        let ids: Vec<_> = queue.into_iter().map(|item| item.provider_id).collect();
        assert_eq!(ids, vec!["p3"]);
        Ok(())
    }

    #[test]
    fn test_reorder_validation() -> Result<(), AppError> {
        let db = Database::memory()?;
        db.save_provider("claude", &provider("p1", "P1"))?;
        db.save_provider("claude", &provider("p2", "P2"))?;
        let g = db.create_routing_group("claude", "g1", None)?;
        db.set_routing_group_members(&g.id, &["p1".to_string(), "p2".to_string()])?;
        // 非成员重排
        let err = db
            .reorder_routing_group_members(&g.id, &["p1".to_string(), "p9".to_string()])
            .unwrap_err();
        assert!(err.to_string().contains("不在分组"), "{err:?}");
        // 长度不一致（去重缺漏）
        let err = db
            .reorder_routing_group_members(&g.id, &["p1".to_string()])
            .unwrap_err();
        assert!(err.to_string().contains("全量成员"), "{err:?}");
        // 跨 app 分组 id
        db.save_provider("codex", &provider("px", "PX"))?;
        let other = db.create_routing_group("codex", "other", None)?;
        let err = db
            .reorder_routing_groups("claude", &[other.id.clone()])
            .unwrap_err();
        assert!(err.to_string().contains("不属于"), "{err:?}");
        Ok(())
    }

    #[test]
    fn test_empty_and_disable_sync_clears_failover_queue() -> Result<(), AppError> {
        let db = Database::memory()?;
        db.save_provider("claude", &provider("p1", "P1"))?;
        db.save_provider("claude", &provider("p2", "P2"))?;
        let g = db.create_routing_group("claude", "solo", None)?;
        db.set_routing_group_members(&g.id, &["p1".to_string()])?;
        assert_eq!(db.get_enabled_routing_group_provider_ids("claude")?.len(), 1);
        assert_eq!(db.get_failover_queue("claude")?.len(), 1);
        // 清空成员后，启用分组为空，队列应清空（而非残留 p1）
        db.set_routing_group_members(&g.id, &[])?;
        assert!(db.get_enabled_routing_group_provider_ids("claude")?.is_empty());
        assert!(db.get_failover_queue("claude")?.is_empty());
        // 再加入 p2 后禁用分组，队列同样清空
        db.set_routing_group_members(&g.id, &["p2".to_string()])?;
        assert_eq!(db.get_failover_queue("claude")?.len(), 1);
        db.update_routing_group(&g.id, None, Some(false), None)?;
        assert!(db.get_enabled_routing_group_provider_ids("claude")?.is_empty());
        assert!(db.get_failover_queue("claude")?.is_empty());
        // 删除分组后亦保持空
        db.delete_routing_group(&g.id)?;
        assert!(db.get_failover_queue("claude")?.is_empty());
        Ok(())
    }

    #[test]
    fn test_failover_queue_double_write_and_detach() -> Result<(), AppError> {
        let db = Database::memory()?;
        for id in ["p1", "p2"] {
            db.save_provider("claude", &provider(id, id))?;
        }
        let fast = db.create_routing_group("claude", "fast", None)?;
        let slow = db.create_routing_group("claude", "slow", None)?;
        db.set_routing_group_members(&fast.id, &["p1".to_string()])?;
        // 旧队列双写 add 应搬运归属到 default 并追加 p2
        db.add_to_failover_queue("claude", "p2")?;
        let groups = db.list_routing_groups("claude")?;
        let default = groups
            .iter()
            .find(|entry| entry.group.id == "rg_default_claude")
            .expect("default group mirrored from failover queue");
        assert!(default.members.iter().any(|member| member.provider_id == "p2"));
        // p1 仍在 fast，p2 在 default，互不重叠
        let fast_members = groups
            .iter()
            .find(|entry| entry.group.id == fast.id)
            .unwrap();
        assert!(fast_members.members.iter().any(|member| member.provider_id == "p1"));
        // 再从旧队列移除 p2，default 应同步移除但 fast 不受影响
        db.remove_from_failover_queue("claude", "p2")?;
        let groups = db.list_routing_groups("claude")?;
        let _ = slow; // 占位，避免未使用警告
        let default = groups.iter().find(|entry| entry.group.id == "rg_default_claude").unwrap();
        assert!(!default.members.iter().any(|member| member.provider_id == "p2"));
        Ok(())
    }
}
