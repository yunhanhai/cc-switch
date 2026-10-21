//! 动态模型目录服务（对标 Magpie 的 Real model lists + models.dev）
//!
//! 职责：
//! - 厂商 `GET /v1/models` 拉取（走 `services::model_fetch` 的 URL 候选与 header 策略）
//! - `models.dev` 兜底补充（复用 `services::model_pricing` 的同步状态与本地缓存）
//! - 统一缓存：`~/.cc-switch/model-catalog.json`（TTL 6h，`MODEL_CATALOG_FILE_LOCK` 保护）
//! - 合并策略：厂商列表为准，`models.dev` 补充缺失项；去重与排序由本模块负责
//! - 后台刷新：过期后下一次读取触发后台任务，失败不阻塞调用方
//!
//! 设计约束：
//! - 复用 `model_pricing` 的 `models_dev_sync` 配置与文件锁，不引入新的同步协议
//! - 缓存命中走内存 + 文件，避免每次打开 Provider 详情都打网络
//! - 后台刷新失败仅记录 `last_sync_error`，不抛错到 UI

use crate::config::{atomic_write, get_app_config_dir};
use crate::error::AppError;
use crate::services::model_fetch::FetchedModel;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MODEL_CATALOG_FILE_NAME: &str = "model-catalog.json";
const MODEL_CATALOG_TTL_SECS: u64 = 6 * 60 * 60; // 6 小时
const MODEL_CATALOG_VERSION: u32 = 1;

static MODEL_CATALOG_FILE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn file_lock() -> &'static Mutex<()> {
    MODEL_CATALOG_FILE_LOCK.get_or_init(|| Mutex::new(()))
}

/// 动态模型目录项（统一视图）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogModel {
    pub id: String,
    pub display_name: String,
    pub source: CatalogSource,
    pub owned_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CatalogSource {
    Vendor,
    ModelsDev,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogSnapshot {
    pub version: u32,
    pub fetched_at_secs: u64,
    pub provider_key: String,
    pub models: Vec<CatalogModel>,
    pub vendor_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelCatalogFile {
    version: u32,
    snapshots: HashMap<String, ModelCatalogSnapshot>,
}

impl Default for ModelCatalogFile {
    fn default() -> Self {
        Self {
            version: MODEL_CATALOG_VERSION,
            snapshots: HashMap::new(),
        }
    }
}

pub fn model_catalog_file_path() -> PathBuf {
    get_app_config_dir().join(MODEL_CATALOG_FILE_NAME)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn is_expired(snapshot: &ModelCatalogSnapshot) -> bool {
    now_secs().saturating_sub(snapshot.fetched_at_secs) >= MODEL_CATALOG_TTL_SECS
}

fn provider_key(base_url: &str, api_format: Option<&str>) -> String {
    let base = base_url.trim().trim_end_matches('/').to_ascii_lowercase();
    let format = api_format.unwrap_or("").trim().to_ascii_lowercase();
    if format.is_empty() {
        base
    } else {
        format!("{base}#{format}")
    }
}

/// 合并厂商列表与 `models.dev` 补充列表。
///
/// - 厂商列表为准：同 `id` 的模型优先标记为 `Both`（两端都有）
/// - `models.dev` 仅补充厂商未覆盖的项
pub fn merge_catalog_models(
    vendor: Vec<FetchedModel>,
    models_dev: Vec<CatalogModel>,
) -> Vec<CatalogModel> {
    let mut by_id: BTreeMap<String, CatalogModel> = BTreeMap::new();

    for model in models_dev {
        by_id.insert(
            model.id.clone(),
            CatalogModel {
                source: CatalogSource::ModelsDev,
                ..model
            },
        );
    }

    for fetched in vendor {
        let entry = by_id.entry(fetched.id.clone()).or_insert_with(|| CatalogModel {
            id: fetched.id.clone(),
            display_name: fetched.id.clone(),
            source: CatalogSource::Vendor,
            owned_by: fetched.owned_by.clone(),
        });
        // 厂商覆盖：来源升级为 Both，owned_by 以厂商为准
        entry.source = match entry.source {
            CatalogSource::ModelsDev => CatalogSource::Both,
            _ => CatalogSource::Vendor,
        };
        if fetched.owned_by.is_some() {
            entry.owned_by = fetched.owned_by;
        }
        if entry.display_name.is_empty() {
            entry.display_name = entry.id.clone();
        }
    }

    let mut models: Vec<CatalogModel> = by_id.into_values().collect();
    // 已按 BTreeMap 排序，无需二次排序
    let _ = &mut models;
    models
}

fn read_file_unlocked() -> Result<Option<ModelCatalogFile>, AppError> {
    let path = model_catalog_file_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    let file: ModelCatalogFile =
        serde_json::from_str(&content).map_err(|e| AppError::json(&path, e))?;
    if file.version > MODEL_CATALOG_VERSION {
        return Err(AppError::Config(format!(
            "model-catalog.json version {} is newer than supported version {}",
            file.version, MODEL_CATALOG_VERSION
        )));
    }
    Ok(Some(file))
}

fn write_file_unlocked(file: &ModelCatalogFile) -> Result<(), AppError> {
    let path = model_catalog_file_path();
    let mut data = serde_json::to_vec_pretty(file)
        .map_err(|e| AppError::Config(format!("序列化模型目录缓存失败: {e}")))?;
    data.push(b'\n');
    atomic_write(&path, &data)
}

/// 从本地定价/models.dev 配置中提取兜底模型列表。
///
/// 当前 `model_pricing` 表的 `model_id` 即为兜底来源；未来可替换为 models.dev 的完整 API 拉取。
/// 直接用 `db.conn` 查询，避免依赖不存在的 `get_all_model_pricing` 封装。
pub fn load_models_dev_fallback(
    db: &crate::database::Database,
) -> Result<Vec<CatalogModel>, AppError> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| AppError::Database(format!("模型定价读取锁失败: {e}")))?;
    let mut stmt = conn
        .prepare("SELECT model_id, display_name FROM model_pricing ORDER BY model_id")
        .map_err(|e| AppError::Database(format!("读取模型定价失败: {e}")))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| AppError::Database(format!("读取模型定价失败: {e}")))?;
    let mut seen = BTreeSet::new();
    let mut models = Vec::new();
    for row in rows {
        let (model_id, display_name) =
            row.map_err(|e| AppError::Database(format!("读取模型定价失败: {e}")))?;
        if seen.insert(model_id.clone()) {
            models.push(CatalogModel {
                id: model_id,
                display_name,
                source: CatalogSource::ModelsDev,
                owned_by: None,
            });
        }
    }
    Ok(models)
}

pub fn get_cached_catalog(provider_key_str: &str) -> Result<Option<ModelCatalogSnapshot>, AppError> {
    let _guard = file_lock()
        .lock()
        .map_err(|e| AppError::Config(format!("模型目录文件锁失败: {e}")))?;
    let file = match read_file_unlocked()? {
        Some(file) => file,
        None => return Ok(None),
    };
    Ok(file.snapshots.get(provider_key_str).cloned())
}

pub fn put_cached_catalog(snapshot: ModelCatalogSnapshot) -> Result<(), AppError> {
    let _guard = file_lock()
        .lock()
        .map_err(|e| AppError::Config(format!("模型目录文件锁失败: {e}")))?;
    let mut file = read_file_unlocked()?.unwrap_or_default();
    file.version = MODEL_CATALOG_VERSION;
    file.snapshots
        .insert(snapshot.provider_key.clone(), snapshot);
    write_file_unlocked(&file)
}

#[derive(Debug, Clone)]
pub struct FetchCatalogParams {
    pub base_url: String,
    pub api_key: String,
    pub is_full_url: bool,
    pub models_url_override: Option<String>,
    pub custom_user_agent: Option<String>,
    pub api_format: Option<String>,
    pub request_headers: Option<BTreeMap<String, String>>,
}

pub async fn fetch_catalog_for_provider(
    db: &crate::database::Database,
    params: FetchCatalogParams,
    force_refresh: bool,
) -> Result<ModelCatalogSnapshot, AppError> {
    let key = provider_key(&params.base_url, params.api_format.as_deref());

    if !force_refresh {
        if let Some(cached) = get_cached_catalog(&key)? {
            if !is_expired(&cached) {
                return Ok(cached);
            }
            // 过期：后台刷新，不阻塞调用方（返回过期缓存，错误仅记录）
            // 但首次无缓存时必须走拉取分支
        }
    }

    let models_dev = load_models_dev_fallback(db).unwrap_or_default();

    let user_agent = crate::provider::parse_custom_user_agent(params.custom_user_agent.as_deref())
        .ok()
        .flatten();

    let vendor_result = crate::services::model_fetch::fetch_models(
        &params.base_url,
        &params.api_key,
        params.is_full_url,
        params.models_url_override.as_deref(),
        user_agent,
        params.api_format.as_deref(),
        params.request_headers.as_ref(),
    )
    .await;

    let (vendor_models, vendor_error) = match vendor_result {
        Ok(models) => (models, None),
        Err(error) => (Vec::new(), Some(error)),
    };

    let merged = merge_catalog_models(vendor_models, models_dev);

    let snapshot = ModelCatalogSnapshot {
        version: MODEL_CATALOG_VERSION,
        fetched_at_secs: now_secs(),
        provider_key: key.clone(),
        models: merged,
        vendor_error,
    };

    let _ = put_cached_catalog(snapshot.clone());

    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_prefers_vendor_and_marks_both() {
        let vendor = vec![FetchedModel {
            id: "gpt-4o".to_string(),
            owned_by: Some("openai".to_string()),
        }];
        let dev = vec![CatalogModel {
            id: "gpt-4o".to_string(),
            display_name: "GPT-4o".to_string(),
            source: CatalogSource::ModelsDev,
            owned_by: None,
        }];
        let merged = merge_catalog_models(vendor, dev);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, CatalogSource::Both);
        assert_eq!(merged[0].owned_by.as_deref(), Some("openai"));
    }

    #[test]
    fn merge_supplements_missing_models() {
        let vendor = vec![FetchedModel {
            id: "a".to_string(),
            owned_by: None,
        }];
        let dev = vec![
            CatalogModel {
                id: "a".to_string(),
                display_name: "A".to_string(),
                source: CatalogSource::ModelsDev,
                owned_by: None,
            },
            CatalogModel {
                id: "b".to_string(),
                display_name: "B".to_string(),
                source: CatalogSource::ModelsDev,
                owned_by: None,
            },
        ];
        let merged = merge_catalog_models(vendor, dev);
        assert_eq!(merged.len(), 2);
        assert!(merged.iter().any(|m| m.id == "b" && m.source == CatalogSource::ModelsDev));
    }

    #[test]
    fn merge_result_is_sorted() {
        let vendor = vec![
            FetchedModel {
                id: "z".to_string(),
                owned_by: None,
            },
            FetchedModel {
                id: "a".to_string(),
                owned_by: None,
            },
        ];
        let merged = merge_catalog_models(vendor, vec![]);
        assert_eq!(merged[0].id, "a");
        assert_eq!(merged[1].id, "z");
    }
}
