//! 动态模型目录命令层

use crate::services::model_catalog::{FetchCatalogParams, ModelCatalogSnapshot};
use crate::store::AppState;
use std::collections::BTreeMap;
use tauri::State;

#[tauri::command(rename_all = "camelCase")]
pub async fn get_model_catalog(
    state: State<'_, AppState>,
    base_url: String,
    api_key: String,
    is_full_url: Option<bool>,
    models_url: Option<String>,
    custom_user_agent: Option<String>,
    api_format: Option<String>,
    request_headers: Option<BTreeMap<String, String>>,
) -> Result<ModelCatalogSnapshot, String> {
    let params = FetchCatalogParams {
        base_url,
        api_key,
        is_full_url: is_full_url.unwrap_or(false),
        models_url_override: models_url,
        custom_user_agent,
        api_format,
        request_headers,
    };
    crate::services::model_catalog::fetch_catalog_for_provider(&state.db, params, false)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn refresh_model_catalog(
    state: State<'_, AppState>,
    base_url: String,
    api_key: String,
    is_full_url: Option<bool>,
    models_url: Option<String>,
    custom_user_agent: Option<String>,
    api_format: Option<String>,
    request_headers: Option<BTreeMap<String, String>>,
) -> Result<ModelCatalogSnapshot, String> {
    let params = FetchCatalogParams {
        base_url,
        api_key,
        is_full_url: is_full_url.unwrap_or(false),
        models_url_override: models_url,
        custom_user_agent,
        api_format,
        request_headers,
    };
    crate::services::model_catalog::fetch_catalog_for_provider(&state.db, params, true)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_model_catalog_cache_status(
    base_url: String,
    api_format: Option<String>,
) -> Result<Option<ModelCatalogSnapshot>, String> {
    let key = {
        let base = base_url.trim().trim_end_matches('/').to_ascii_lowercase();
        let format = api_format.unwrap_or_default().trim().to_ascii_lowercase();
        if format.is_empty() {
            base
        } else {
            format!("{base}#{format}")
        }
    };
    crate::services::model_catalog::get_cached_catalog(&key).map_err(|e| e.to_string())
}

