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
        conn.execute(
            "UPDATE routing_groups SET name = ?1, enabled = ?2, sort_order = ?3, updated_at = ?4 WHERE id = ?5",
            params![new_name, if new_enabled { 1 } else { 0 }, new_order, now, id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
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
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute("DELETE FROM routing_groups WHERE id = ?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        if affected == 0 {
            return Err(AppError::InvalidInput(format!("路由分组不存在: {id}")));
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
        Ok(())
    }

    pub fn remove_routing_group_member(
        &self,
        group_id: &str,
        provider_id: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM routing_group_members WHERE group_id = ?1 AND provider_id = ?2",
            params![group_id, provider_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
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
}
