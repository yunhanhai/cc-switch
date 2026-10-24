 //! Model Aliases DAO（同模型跨渠道无缝切换）
 //!
 //! 一个逻辑模型（`alias`，如 `sonnet`）在多个渠道各有不同物理名
 //! （`target_model`）。选路阶段按 alias 过滤候选渠道，转发阶段按当前
 //! 正在尝试的渠道解析物理名，实现「渠道断了自动换、模型名跟着换」。
 //!
 //! 与 `settings_config.model_mapping` 的分工：
 //! - 本表：跨渠道共享的逻辑名 → 各渠道物理名
 //! - `model_mapping`：单渠道私有的显示名 → 实际名（保留，作为回退）
 
 use crate::database::{lock_conn, Database};
 use crate::error::AppError;
 use rusqlite::params;
 use serde::{Deserialize, Serialize};
 use std::collections::HashMap;
 
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelAlias {
     pub id: String,
     pub app_type: String,
     pub alias: String,
     pub provider_id: String,
     pub target_model: String,
     pub priority: i64,
     pub enabled: bool,
     pub created_at: i64,
     pub updated_at: i64,
 }
 
 impl Database {
     fn validate_alias_name(value: &str) -> Result<String, AppError> {
         let trimmed = value.trim();
         if trimmed.is_empty() {
             return Err(AppError::InvalidInput("模型别名不能为空".to_string()));
         }
         if trimmed.len() > 128 {
             return Err(AppError::InvalidInput(
                 "模型别名不能超过 128 个字符".to_string(),
             ));
         }
         Ok(trimmed.to_string())
     }
 
     fn validate_target_model(value: &str) -> Result<String, AppError> {
         let trimmed = value.trim();
         if trimmed.is_empty() {
             return Err(AppError::InvalidInput(
                 "目标模型名不能为空".to_string(),
             ));
         }
         if trimmed.len() > 256 {
             return Err(AppError::InvalidInput(
                 "目标模型名不能超过 256 个字符".to_string(),
             ));
         }
         Ok(trimmed.to_string())
     }
 
     /// 列出某 app 下全部别名（按 alias、priority 排序）。
     pub fn list_model_aliases(&self, app_type: &str) -> Result<Vec<ModelAlias>, AppError> {
         let conn = lock_conn!(self.conn);
         let mut stmt = conn
             .prepare(
                 "SELECT id, app_type, alias, provider_id, target_model,
                         priority, enabled, created_at, updated_at
                  FROM model_aliases WHERE app_type = ?1
                  ORDER BY alias, priority, provider_id",
             )
             .map_err(|e| AppError::Database(e.to_string()))?;
         let rows = stmt
             .query_map(params![app_type], |row| {
                 Ok(ModelAlias {
                     id: row.get(0)?,
                     app_type: row.get(1)?,
                     alias: row.get(2)?,
                     provider_id: row.get(3)?,
                     target_model: row.get(4)?,
                     priority: row.get(5)?,
                     enabled: row.get::<_, i32>(6)? != 0,
                     created_at: row.get(7)?,
                     updated_at: row.get(8)?,
                 })
             })
             .map_err(|e| AppError::Database(e.to_string()))?
             .collect::<Result<Vec<_>, _>>()
             .map_err(|e| AppError::Database(e.to_string()))?;
         Ok(rows)
     }
 
     /// 新增或更新一条别名（按 `(app_type, alias, provider_id)` 唯一键 upsert）。
     pub fn upsert_model_alias(
         &self,
         app_type: &str,
         alias: &str,
         provider_id: &str,
         target_model: &str,
         priority: Option<i64>,
         enabled: Option<bool>,
     ) -> Result<ModelAlias, AppError> {
         let alias = Self::validate_alias_name(alias)?;
         let target_model = Self::validate_target_model(target_model)?;
         let priority = priority.unwrap_or(0);
         let enabled = enabled.unwrap_or(true);
         let now = chrono::Utc::now().timestamp();
         let conn = lock_conn!(self.conn);
 
         // 归属校验：provider 必须存在于该 app，避免配了永远走不到的别名
         let provider_exists: bool = conn
             .query_row(
                 "SELECT 1 FROM providers WHERE id = ?1 AND app_type = ?2",
                 params![provider_id, app_type],
                 |_| Ok(true),
             )
             .unwrap_or(false);
         if !provider_exists {
             return Err(AppError::InvalidInput(format!(
                 "供应商不存在: {provider_id} ({app_type})"
             )));
         }
 
         let existing_id: Option<String> = conn
             .query_row(
                 "SELECT id FROM model_aliases
                  WHERE app_type = ?1 AND alias = ?2 AND provider_id = ?3",
                 params![app_type, alias, provider_id],
                 |row| row.get(0),
             )
             .ok();
 
         let id = match existing_id {
             Some(id) => {
                 conn.execute(
                     "UPDATE model_aliases
                      SET target_model = ?1, priority = ?2, enabled = ?3, updated_at = ?4
                      WHERE id = ?5 AND app_type = ?6",
                     params![
                         target_model,
                         priority,
                         if enabled { 1 } else { 0 },
                         now,
                         id,
                         app_type
                     ],
                 )
                 .map_err(|e| AppError::Database(e.to_string()))?;
                 id
             }
             None => {
                 let id = format!("ma_{}", uuid::Uuid::new_v4());
                 conn.execute(
                     "INSERT INTO model_aliases
                      (id, app_type, alias, provider_id, target_model,
                       priority, enabled, created_at, updated_at)
                      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                     params![
                         id,
                         app_type,
                         alias,
                         provider_id,
                         target_model,
                         priority,
                         if enabled { 1 } else { 0 },
                         now
                     ],
                 )
                 .map_err(|e| AppError::Database(e.to_string()))?;
                 id
             }
         };
 
         Ok(ModelAlias {
             id,
             app_type: app_type.to_string(),
             alias,
             provider_id: provider_id.to_string(),
             target_model,
             priority,
             enabled,
             created_at: now,
             updated_at: now,
         })
     }
 
     pub fn delete_model_alias(&self, app_type: &str, id: &str) -> Result<(), AppError> {
         let conn = lock_conn!(self.conn);
         let affected = conn
             .execute(
                 "DELETE FROM model_aliases WHERE id = ?1 AND app_type = ?2",
                 params![id, app_type],
             )
             .map_err(|e| AppError::Database(e.to_string()))?;
         if affected == 0 {
             return Err(AppError::InvalidInput(format!("模型别名不存在: {id}")));
         }
         Ok(())
     }
 
     /// 用给定集合整体替换某 app 下某个 alias 的所有渠道映射（UI 保存用）。
     ///
     /// `entries` 为 `(provider_id, target_model)`；空集合表示删除该 alias。
     pub fn set_model_alias_entries(
         &self,
         app_type: &str,
         alias: &str,
         entries: &[(String, String)],
     ) -> Result<(), AppError> {
         let alias = Self::validate_alias_name(alias)?;
         // 先全量校验，避免删了一半才发现某条非法
         let mut normalized = Vec::with_capacity(entries.len());
         let mut seen = std::collections::HashSet::new();
         for (provider_id, target_model) in entries {
             let target = Self::validate_target_model(target_model)?;
             if !seen.insert(provider_id.clone()) {
                 return Err(AppError::InvalidInput(format!(
                     "同一渠道重复配置: {provider_id}"
                 )));
             }
             normalized.push((provider_id.clone(), target));
         }
 
         let conn = lock_conn!(self.conn);
         for (provider_id, _) in &normalized {
             let exists: bool = conn
                 .query_row(
                     "SELECT 1 FROM providers WHERE id = ?1 AND app_type = ?2",
                     params![provider_id, app_type],
                     |_| Ok(true),
                 )
                 .unwrap_or(false);
             if !exists {
                 return Err(AppError::InvalidInput(format!(
                     "供应商不存在: {provider_id} ({app_type})"
                 )));
             }
         }
 
         conn.execute(
             "DELETE FROM model_aliases WHERE app_type = ?1 AND alias = ?2",
             params![app_type, alias],
         )
         .map_err(|e| AppError::Database(e.to_string()))?;
 
         let now = chrono::Utc::now().timestamp();
         for (index, (provider_id, target_model)) in normalized.iter().enumerate() {
             let id = format!("ma_{}", uuid::Uuid::new_v4());
             conn.execute(
                 "INSERT INTO model_aliases
                  (id, app_type, alias, provider_id, target_model,
                   priority, enabled, created_at, updated_at)
                  VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7)",
                 params![id, app_type, alias, provider_id, target_model, index as i64, now],
             )
             .map_err(|e| AppError::Database(e.to_string()))?;
         }
         Ok(())
     }
 
     /// 解析某渠道上该逻辑模型对应的物理名。
     ///
     /// 返回 `None` 表示该渠道没有为这个 alias 声明物理名 —— 调用方应回退到
     /// 既有的 `model_mapping` / env 映射链路，而不是直接判定为不支持。
     pub fn resolve_alias_target(
         &self,
         app_type: &str,
         alias: &str,
         provider_id: &str,
     ) -> Result<Option<String>, AppError> {
         let conn = lock_conn!(self.conn);
         let target: Option<String> = conn
             .query_row(
                 "SELECT target_model FROM model_aliases
                  WHERE app_type = ?1 AND alias = ?2 AND provider_id = ?3 AND enabled = 1
                  ORDER BY priority, updated_at DESC LIMIT 1",
                 params![app_type, alias, provider_id],
                 |row| row.get(0),
             )
             .ok();
         Ok(target)
     }
 
     /// 该 app 下某 alias 声明了哪些渠道（用于选路过滤）。
     ///
     /// 返回空集合有两种含义，调用方必须区分：
     /// - 该 app 完全没有 alias 数据 → 不做过滤（保持旧行为）
     /// - 该 app 有 alias 数据但此 alias 无渠道 → 该模型不可路由
     pub fn providers_for_alias(
         &self,
         app_type: &str,
         alias: &str,
     ) -> Result<Vec<String>, AppError> {
         let conn = lock_conn!(self.conn);
         let mut stmt = conn
             .prepare(
                 "SELECT provider_id FROM model_aliases
                  WHERE app_type = ?1 AND alias = ?2 AND enabled = 1
                  ORDER BY priority, provider_id",
             )
             .map_err(|e| AppError::Database(e.to_string()))?;
         let rows = stmt
             .query_map(params![app_type, alias], |row| row.get::<_, String>(0))
             .map_err(|e| AppError::Database(e.to_string()))?
             .collect::<Result<Vec<_>, _>>()
             .map_err(|e| AppError::Database(e.to_string()))?;
         Ok(rows)
     }
 
     /// 该 app 是否存在任何 alias 定义（选路过滤的安全阀）。
     pub fn has_any_model_alias(&self, app_type: &str) -> Result<bool, AppError> {
         let conn = lock_conn!(self.conn);
         let exists: bool = conn
             .query_row(
                 "SELECT 1 FROM model_aliases WHERE app_type = ?1 AND enabled = 1 LIMIT 1",
                 params![app_type],
                 |_| Ok(true),
             )
             .unwrap_or(false);
         Ok(exists)
     }
 
     /// 批量解析：一次取回该 app 下所有 alias → provider → target 的映射。
     ///
     /// 选路与转发是热路径，逐条 `resolve_alias_target` 会反复拿锁；
     /// 这里一次读全表，调用方在内存里查。
     pub fn model_alias_index(
         &self,
         app_type: &str,
     ) -> Result<HashMap<(String, String), String>, AppError> {
         let conn = lock_conn!(self.conn);
         let mut stmt = conn
             .prepare(
                 "SELECT alias, provider_id, target_model FROM model_aliases
                  WHERE app_type = ?1 AND enabled = 1
                  ORDER BY priority",
             )
             .map_err(|e| AppError::Database(e.to_string()))?;
         let rows = stmt
             .query_map(params![app_type], |row| {
                 Ok((
                     row.get::<_, String>(0)?,
                     row.get::<_, String>(1)?,
                     row.get::<_, String>(2)?,
                 ))
             })
             .map_err(|e| AppError::Database(e.to_string()))?
             .collect::<Result<Vec<_>, _>>()
             .map_err(|e| AppError::Database(e.to_string()))?;
 
         let mut index = HashMap::with_capacity(rows.len());
         for (alias, provider_id, target) in rows {
             // 先到先得：ORDER BY priority 保证高优先级不被覆盖
             index.entry((alias, provider_id)).or_insert(target);
         }
         Ok(index)
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
 
     fn seed(db: &Database) -> Result<(), AppError> {
         for id in ["ch_a", "ch_b", "ch_c"] {
             db.save_provider("claude", &provider(id, id))?;
         }
         Ok(())
     }
 
     #[test]
     fn alias_upsert_and_resolve_per_provider() -> Result<(), AppError> {
         let db = Database::memory()?;
         seed(&db)?;
 
         // 同一逻辑名在三个渠道各有不同物理名
         db.upsert_model_alias("claude", "sonnet", "ch_a", "claude-sonnet-4-5-20250929", None, None)?;
         db.upsert_model_alias("claude", "sonnet", "ch_b", "anthropic/claude-sonnet-4.5", None, None)?;
         db.upsert_model_alias("claude", "sonnet", "ch_c", "sonnet-latest", None, None)?;
 
         assert_eq!(
             db.resolve_alias_target("claude", "sonnet", "ch_a")?.as_deref(),
             Some("claude-sonnet-4-5-20250929")
         );
         assert_eq!(
             db.resolve_alias_target("claude", "sonnet", "ch_b")?.as_deref(),
             Some("anthropic/claude-sonnet-4.5")
         );
         assert_eq!(
             db.resolve_alias_target("claude", "sonnet", "ch_c")?.as_deref(),
             Some("sonnet-latest")
         );
         // 未声明的渠道返回 None（调用方回退到旧映射）
         db.save_provider("claude", &provider("ch_d", "ch_d"))?;
         assert_eq!(db.resolve_alias_target("claude", "sonnet", "ch_d")?, None);
         Ok(())
     }
 
     #[test]
    fn alias_upsert_is_idempotent_on_unique_key() -> Result<(), AppError> {
        let db = Database::memory()?;
        seed(&db)?;
        db.upsert_model_alias("claude", "sonnet", "ch_a", "old-name", None, None)?;
        db.upsert_model_alias("claude", "sonnet", "ch_a", "new-name", None, None)?;
        let all = db.list_model_aliases("claude")?;
        assert_eq!(all.len(), 1, "同一 (alias, provider) 应只有一条");
        assert_eq!(all[0].target_model, "new-name");
        Ok(())
    }

    /// 前端 `ModelAlias` 接口用 camelCase（`appType` / `providerId` / `targetModel`），
    /// 而这些结构体是直接作为 Tauri 命令返回值序列化给前端的。
    /// 此测试确保序列化后的字段名与前端读取的一致——否则 UI 会静默拿到 undefined。
    #[test]
    fn model_alias_serializes_with_camel_case_field_names() -> Result<(), AppError> {
        let db = Database::memory()?;
        seed(&db)?;
        let created = db.upsert_model_alias("claude", "sonnet", "ch_a", "a-model", None, None)?;

        let value = serde_json::to_value(&created).unwrap();
        let object = value.as_object().expect("应为 JSON 对象");

        for field in [
            "appType",
            "providerId",
            "targetModel",
            "createdAt",
            "updatedAt",
        ] {
            assert!(
                object.contains_key(field),
                "序列化后缺少 camelCase 字段 {field}（前端会读到 undefined），实际字段: {:?}",
                object.keys().collect::<Vec<_>>()
            );
        }
        Ok(())
    }
 
     #[test]
     fn alias_rejects_unknown_provider() -> Result<(), AppError> {
         let db = Database::memory()?;
         seed(&db)?;
         let err = db
             .upsert_model_alias("claude", "sonnet", "ghost", "x", None, None)
             .unwrap_err();
         assert!(err.to_string().contains("供应商不存在"), "{err:?}");
         // 跨 app 的 provider 也不允许
         db.save_provider("codex", &provider("cx", "cx"))?;
         let err = db
             .upsert_model_alias("claude", "sonnet", "cx", "x", None, None)
             .unwrap_err();
         assert!(err.to_string().contains("供应商不存在"), "{err:?}");
         Ok(())
     }
 
     #[test]
     fn alias_set_entries_replaces_whole_alias() -> Result<(), AppError> {
         let db = Database::memory()?;
         seed(&db)?;
         db.set_model_alias_entries(
             "claude",
             "fast",
             &[
                 ("ch_a".to_string(), "haiku-fast".to_string()),
                 ("ch_b".to_string(), "hf-1".to_string()),
             ],
         )?;
         assert_eq!(db.providers_for_alias("claude", "fast")?.len(), 2);
 
         // 覆盖为单条：旧的两条必须被清掉
         db.set_model_alias_entries(
             "claude",
             "fast",
             &[("ch_c".to_string(), "hc".to_string())],
         )?;
         assert_eq!(
             db.providers_for_alias("claude", "fast")?,
             vec!["ch_c".to_string()]
         );
         // 空集合 = 删除该 alias
         db.set_model_alias_entries("claude", "fast", &[])?;
         assert!(db.providers_for_alias("claude", "fast")?.is_empty());
         Ok(())
     }
 
     #[test]
     fn alias_rejects_duplicate_provider_in_same_batch() -> Result<(), AppError> {
         let db = Database::memory()?;
         seed(&db)?;
         let err = db
             .set_model_alias_entries(
                 "claude",
                 "sonnet",
                 &[
                     ("ch_a".to_string(), "a1".to_string()),
                     ("ch_a".to_string(), "a2".to_string()),
                 ],
             )
             .unwrap_err();
         assert!(err.to_string().contains("重复配置"), "{err:?}");
         // 校验失败不得留下半成品
         assert!(db.providers_for_alias("claude", "sonnet")?.is_empty());
         Ok(())
     }
 
     #[test]
     fn alias_disabled_is_ignored_by_resolve_and_filter() -> Result<(), AppError> {
         let db = Database::memory()?;
         seed(&db)?;
         db.upsert_model_alias("claude", "sonnet", "ch_a", "a", None, Some(false))?;
         assert_eq!(db.resolve_alias_target("claude", "sonnet", "ch_a")?, None);
         assert!(db.providers_for_alias("claude", "sonnet")?.is_empty());
         // 全部禁用等同于没有 alias 数据
         assert!(!db.has_any_model_alias("claude")?);
         Ok(())
     }

     #[test]
     fn alias_index_and_safety_valve_are_per_app() -> Result<(), AppError> {
         let db = Database::memory()?;
         seed(&db)?;
         db.save_provider("codex", &provider("cx", "cx"))?;
         db.upsert_model_alias("claude", "sonnet", "ch_a", "a-model", None, None)?;

         // 安全阀：codex 完全没配 alias → false，选路不应过滤
         assert!(db.has_any_model_alias("claude")?);
         assert!(!db.has_any_model_alias("codex")?, "无 alias 的 app 必须返回 false");

         // 索引只含本 app 数据，不串扰
         let index = db.model_alias_index("claude")?;
         assert_eq!(
             index.get(&("sonnet".to_string(), "ch_a".to_string())).map(String::as_str),
             Some("a-model")
         );
         assert!(db.model_alias_index("codex")?.is_empty());
         Ok(())
     }

     #[test]
     fn alias_delete_requires_matching_app() -> Result<(), AppError> {
         let db = Database::memory()?;
         seed(&db)?;
         let created = db.upsert_model_alias("claude", "sonnet", "ch_a", "a", None, None)?;
         // 用错 app 删除应报错，且不误删
         let err = db.delete_model_alias("codex", &created.id).unwrap_err();
         assert!(err.to_string().contains("不存在"), "{err:?}");
         assert_eq!(db.list_model_aliases("claude")?.len(), 1);
         db.delete_model_alias("claude", &created.id)?;
         assert!(db.list_model_aliases("claude")?.is_empty());
         Ok(())
     }
 }
