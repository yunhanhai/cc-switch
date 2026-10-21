//! Routing Groups 命令（统一网关抽象 + smart 路由分组）

use crate::database::{RoutingGroup, RoutingGroupWithMembers};
use crate::store::AppState;
use std::str::FromStr;
use tauri::State;

fn require_proxy_app(app_type: &str) -> Result<crate::app_config::AppType, String> {
    let app = crate::app_config::AppType::from_str(app_type)
        .map_err(|e| format!("无效的应用类型: {e}"))?;
    if !app.supports_local_proxy() {
        return Err(format!("{} 不支持路由分组", app.as_str()));
    }
    Ok(app)
}

#[tauri::command]
pub fn list_routing_groups(
    state: State<'_, AppState>,
    app_type: String,
) -> Result<Vec<RoutingGroupWithMembers>, String> {
    require_proxy_app(&app_type)?;
    state
        .db
        .list_routing_groups(&app_type)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_routing_group(
    state: State<'_, AppState>,
    app_type: String,
    name: String,
) -> Result<RoutingGroup, String> {
    require_proxy_app(&app_type)?;
    state
        .db
        .create_routing_group(&app_type, &name, None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_routing_group(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    enabled: Option<bool>,
) -> Result<RoutingGroup, String> {
    state
        .db
        .update_routing_group(&id, name.as_deref(), enabled, None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_routing_group(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.db.delete_routing_group(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_routing_group(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<RoutingGroupWithMembers>, String> {
    state.db.get_routing_group(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_routing_group_members(
    state: State<'_, AppState>,
    group_id: String,
    provider_ids: Vec<String>,
) -> Result<(), String> {
    state
        .db
        .set_routing_group_members(&group_id, &provider_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_routing_group_member(
    state: State<'_, AppState>,
    group_id: String,
    provider_id: String,
) -> Result<(), String> {
    state
        .db
        .add_routing_group_member(&group_id, &provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_routing_group_member(
    state: State<'_, AppState>,
    group_id: String,
    provider_id: String,
) -> Result<(), String> {
    state
        .db
        .remove_routing_group_member(&group_id, &provider_id)
        .map_err(|e| e.to_string())
}
