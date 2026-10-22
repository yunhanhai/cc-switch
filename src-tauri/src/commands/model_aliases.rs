 //! Model Alias 命令（同模型跨渠道无缝切换）
 //!
 //! 一个逻辑模型（如 `sonnet`）在多个渠道各有不同物理名。配置后：
 //! - 选路阶段跳过不认识该逻辑模型的渠道
 //! - 转发阶段按当前渠道把模型名换成该渠道的物理名
 //! 从而做到「渠道断了自动换，模型名跟着换」。
 
 use crate::database::ModelAlias;
 use crate::store::AppState;
 use std::str::FromStr;
 use tauri::State;
 
 fn require_proxy_app(app_type: &str) -> Result<crate::app_config::AppType, String> {
     let app = crate::app_config::AppType::from_str(app_type)
         .map_err(|e| format!("无效的应用类型: {e}"))?;
     if !app.supports_local_proxy() {
         return Err(format!("{} 不支持模型别名", app.as_str()));
     }
     Ok(app)
 }
 
 #[tauri::command]
 pub fn list_model_aliases(
     state: State<'_, AppState>,
     app_type: String,
 ) -> Result<Vec<ModelAlias>, String> {
     require_proxy_app(&app_type)?;
     state
         .db
         .list_model_aliases(&app_type)
         .map_err(|e| e.to_string())
 }
 
 /// 整体替换某个逻辑模型的渠道映射（UI 保存）。
 ///
 /// `entries` 为 `(provider_id, target_model)`；传空数组等于删除该逻辑模型。
 #[tauri::command]
 pub fn set_model_alias_entries(
     state: State<'_, AppState>,
     app_type: String,
     alias: String,
     entries: Vec<(String, String)>,
 ) -> Result<(), String> {
     require_proxy_app(&app_type)?;
     state
         .db
         .set_model_alias_entries(&app_type, &alias, &entries)
         .map_err(|e| e.to_string())
 }
 
 #[tauri::command]
 pub fn upsert_model_alias(
     state: State<'_, AppState>,
     app_type: String,
     alias: String,
     provider_id: String,
     target_model: String,
 ) -> Result<ModelAlias, String> {
     require_proxy_app(&app_type)?;
     state
         .db
         .upsert_model_alias(&app_type, &alias, &provider_id, &target_model, None, None)
         .map_err(|e| e.to_string())
 }
 
 #[tauri::command]
 pub fn delete_model_alias(
     state: State<'_, AppState>,
     app_type: String,
     id: String,
 ) -> Result<(), String> {
     require_proxy_app(&app_type)?;
     state
         .db
         .delete_model_alias(&app_type, &id)
         .map_err(|e| e.to_string())
 }
 
/// 该 app 是否已配置任何模型别名。
///
/// 前端用它决定是否展示「已启用模型别名路由」的提示。
#[tauri::command]
pub fn has_model_aliases(state: State<'_, AppState>, app_type: String) -> Result<bool, String> {
    require_proxy_app(&app_type)?;
    state
        .db
        .has_any_model_alias(&app_type)
        .map_err(|e| e.to_string())
}

/// 一条归并建议（供前端展示「一键采纳」）。
#[derive(serde::Serialize)]
pub struct ModelAliasSuggestion {
    pub alias: String,
    /// (provider_id, target_model)
    pub entries: Vec<(String, String)>,
}

/// 从各渠道已有的 `model_mapping` 归并出 alias 建议。
///
/// 只建议「同一个 source 出现在两个及以上渠道」的情况 —— 那是用户实际已在
/// 用同一名字打不同渠道、但还没显式声明成一组。已配置过的 alias 会被过滤。
#[tauri::command]
pub fn suggest_model_aliases(
    state: State<'_, AppState>,
    app_type: String,
) -> Result<Vec<ModelAliasSuggestion>, String> {
    require_proxy_app(&app_type)?;

    let providers = state
        .db
        .get_all_providers(&app_type)
        .map_err(|e| e.to_string())?;
    let pairs_by_provider: Vec<(String, Vec<(String, String)>)> = providers
        .values()
        .map(|provider| {
            (
                provider.id.clone(),
                crate::proxy::model_mapper::provider_mapping_pairs(provider),
            )
        })
        .filter(|(_, pairs)| !pairs.is_empty())
        .collect();

    let existing_aliases: Vec<String> = state
        .db
        .list_model_aliases(&app_type)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|item| item.alias)
        .collect();

    let suggestions = crate::proxy::model_alias::suggest_aliases_from_mappings(&pairs_by_provider);
    let filtered =
        crate::proxy::model_alias::filter_existing_aliases(suggestions, &existing_aliases);

    Ok(filtered
        .into_iter()
        .map(|suggestion| ModelAliasSuggestion {
            alias: suggestion.alias,
            entries: suggestion.entries,
        })
        .collect())
}
