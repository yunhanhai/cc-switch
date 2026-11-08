//! Model Alias 解析（同模型跨渠道无缝切换）
//!
//! 职责边界：
//! - 本模块只做**纯函数**解析，不碰数据库，便于单测与在热路径反复调用。
//! - 数据读取在 `Database::model_alias_index`（一次读全表，内存查）。
//!
//! 两个关键语义（容易写错，单独强调）：
//!
//! 1. **安全阀**：某 app 完全没有 alias 数据时，选路**不得**做任何过滤。
//!    否则用户升级后所有请求都会因为「没声明 alias」被判为不支持。
//! 2. **回退而非拒绝**：某渠道没有为该 alias 声明物理名时，
//!    解析结果应为 `None`（交给旧的 `model_mapping` 链路），
//!    而不是判定该渠道不可用 —— 存量配置都是这种形态。

use std::collections::HashMap;

/// alias → provider → 物理名的索引（由 `Database::model_alias_index` 构建）。
pub type ModelAliasIndex = HashMap<(String, String), String>;

/// 解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasResolution {
    /// 该渠道为此 alias 声明了物理名，应替换请求中的 model。
    Mapped(String),
    /// 该渠道未声明此 alias，调用方应回退到既有映射链路。
    NotDeclared,
}

/// 在索引中查找某渠道对该逻辑模型的物理名。
pub fn resolve_for_provider(
    index: &ModelAliasIndex,
    alias: &str,
    provider_id: &str,
) -> AliasResolution {
    match index.get(&(alias.to_string(), provider_id.to_string())) {
        Some(target) if !target.trim().is_empty() => AliasResolution::Mapped(target.clone()),
        _ => AliasResolution::NotDeclared,
    }
}

/// 选路过滤：判断某渠道是否应当作为该模型的候选。
///
/// 返回 `true` 的全部情形（按顺序短路）：
/// 1. `index` 为空 —— 该 app 没有任何 alias，安全阀生效，全量放开；
/// 2. 该渠道为该 alias 声明了物理名；
/// 3. 该渠道存在其它形式的模型映射（`has_own_mapping`），交给旧链路解析。
///
/// 只有「该 app 有 alias 数据、但此渠道既没声明此 alias、也没有自有映射」
/// 时才返回 `false`。
pub fn provider_supports_alias(
    index: &ModelAliasIndex,
    alias: &str,
    provider_id: &str,
    has_own_mapping: bool,
) -> bool {
    if index.is_empty() {
        return true;
    }
    if matches!(
        resolve_for_provider(index, alias, provider_id),
        AliasResolution::Mapped(_)
    ) {
        return true;
    }
    // 没有 alias 声明但有自有映射：可能是通配规则，交给旧链路判定
    has_own_mapping
}

/// 归并建议：一个逻辑模型及其建议的渠道映射。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasSuggestion {
    pub alias: String,
    pub entries: Vec<(String, String)>,
}

/// 从各渠道已有的 `model_mapping` 归并出 alias 建议。
///
/// 规则：同一个 `source`（客户端实际发送的模型名）出现在 **两个及以上** 渠道时，
/// 它就是天然的「逻辑模型」——用户已经在用同一个名字打不同渠道，只是还没把
/// 它们显式声明成一组。只有单渠道出现的 source 不生成建议：那只是该渠道私有
/// 的改名，归并成 alias 反而会限制它。
///
/// `pairs_by_provider` 为 `(provider_id, [(source, target)])`。
///
/// 通配符 source（含 `*`）一律跳过：它表达的是"兜底"而不是某个具体模型，
/// 归并成 alias 会把兜底规则变成精确匹配，语义被改变。
pub fn suggest_aliases_from_mappings(
    pairs_by_provider: &[(String, Vec<(String, String)>)],
) -> Vec<AliasSuggestion> {
    // source -> [(provider_id, target)]
    let mut by_source: std::collections::BTreeMap<String, Vec<(String, String)>> =
        std::collections::BTreeMap::new();

    for (provider_id, pairs) in pairs_by_provider {
        for (source, target) in pairs {
            if source.contains('*') {
                continue;
            }
            let entry = by_source.entry(source.clone()).or_default();
            // 同一渠道内重复的 source 只保留第一条（与 apply_model_mapping 的
            // "先配置的精确规则优先" 语义一致）
            if !entry.iter().any(|(existing, _)| existing == provider_id) {
                entry.push((provider_id.clone(), target.clone()));
            }
        }
    }

    by_source
        .into_iter()
        .filter(|(_, entries)| entries.len() >= 2)
        .map(|(alias, entries)| AliasSuggestion { alias, entries })
        .collect()
}

/// 过滤掉用户已经配置过的 alias，避免重复建议。
pub fn filter_existing_aliases(
    suggestions: Vec<AliasSuggestion>,
    existing_aliases: &[String],
) -> Vec<AliasSuggestion> {
    let existing: std::collections::HashSet<&str> =
        existing_aliases.iter().map(String::as_str).collect();
    suggestions
        .into_iter()
        .filter(|suggestion| !existing.contains(suggestion.alias.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_of(entries: &[(&str, &str, &str)]) -> ModelAliasIndex {
        entries
            .iter()
            .map(|(alias, provider, target)| {
                (
                    (alias.to_string(), provider.to_string()),
                    target.to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn resolve_returns_mapped_for_declared_provider() {
        let index = index_of(&[("sonnet", "ch_a", "claude-sonnet-4-5")]);
        assert_eq!(
            resolve_for_provider(&index, "sonnet", "ch_a"),
            AliasResolution::Mapped("claude-sonnet-4-5".to_string())
        );
    }

    #[test]
    fn resolve_returns_not_declared_for_unknown_provider() {
        let index = index_of(&[("sonnet", "ch_a", "claude-sonnet-4-5")]);
        assert_eq!(
            resolve_for_provider(&index, "sonnet", "ch_b"),
            AliasResolution::NotDeclared
        );
        assert_eq!(
            resolve_for_provider(&index, "opus", "ch_a"),
            AliasResolution::NotDeclared
        );
    }

    #[test]
    fn empty_target_is_treated_as_not_declared() {
        // 防御性：库里若存在空白 target（不应发生），不能把空串发给上游
        let index = index_of(&[("sonnet", "ch_a", "   ")]);
        assert_eq!(
            resolve_for_provider(&index, "sonnet", "ch_a"),
            AliasResolution::NotDeclared
        );
    }

    #[test]
    fn empty_index_opens_all_providers() {
        // 安全阀：没有任何 alias 数据时不得过滤，否则升级即全部不可用
        let empty = ModelAliasIndex::new();
        assert!(provider_supports_alias(&empty, "sonnet", "anything", false));
    }

    #[test]
    fn declared_provider_passes_even_without_own_mapping() {
        let index = index_of(&[("sonnet", "ch_a", "s-a")]);
        assert!(provider_supports_alias(&index, "sonnet", "ch_a", false));
    }

    #[test]
    fn undeclared_provider_without_mapping_is_filtered_out() {
        let index = index_of(&[("sonnet", "ch_a", "s-a")]);
        assert!(!provider_supports_alias(&index, "sonnet", "ch_b", false));
    }

    #[test]
    fn undeclared_provider_with_own_mapping_passes_through() {
        // 渠道 B 没配 alias，但有自己的 model_mapping（可能是通配）→ 交给旧链路
        let index = index_of(&[("sonnet", "ch_a", "s-a")]);
        assert!(provider_supports_alias(&index, "sonnet", "ch_b", true));
    }

    fn pairs(entries: &[(&str, &[(&str, &str)])]) -> Vec<(String, Vec<(String, String)>)> {
        entries
            .iter()
            .map(|(provider, rules)| {
                (
                    provider.to_string(),
                    rules
                        .iter()
                        .map(|(source, target)| (source.to_string(), target.to_string()))
                        .collect(),
                )
            })
            .collect()
    }

    #[test]
    fn suggestion_groups_source_shared_by_multiple_channels() {
        let input = pairs(&[
            (
                "ch_a",
                &[
                    ("sonnet", "claude-sonnet-4-5-20250929"),
                    ("only-a", "private-a"),
                ],
            ),
            (
                "ch_b",
                &[
                    ("sonnet", "anthropic/claude-sonnet-4.5"),
                    ("only-b", "private-b"),
                ],
            ),
        ]);

        let suggestions = suggest_aliases_from_mappings(&input);
        assert_eq!(suggestions.len(), 1, "只有 sonnet 出现在两个渠道");
        assert_eq!(suggestions[0].alias, "sonnet");
        assert_eq!(suggestions[0].entries.len(), 2);
        assert_eq!(
            suggestions[0].entries[0],
            ("ch_a".to_string(), "claude-sonnet-4-5-20250929".to_string())
        );
        assert_eq!(
            suggestions[0].entries[1],
            (
                "ch_b".to_string(),
                "anthropic/claude-sonnet-4.5".to_string()
            )
        );
    }

    #[test]
    fn suggestion_skips_single_channel_and_wildcards() {
        let input = pairs(&[
            ("ch_a", &[("solo", "x"), ("*", "fallback")]),
            ("ch_b", &[("*", "other-fallback")]),
        ]);
        // solo 只在一个渠道；* 是通配兜底 —— 都不应生成建议
        assert!(suggest_aliases_from_mappings(&input).is_empty());
    }

    #[test]
    fn suggestion_dedups_repeated_source_within_one_channel() {
        let input = pairs(&[
            ("ch_a", &[("sonnet", "first"), ("sonnet", "second")]),
            ("ch_b", &[("sonnet", "b-sonnet")]),
        ]);
        let suggestions = suggest_aliases_from_mappings(&input);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].entries.len(), 2, "同渠道重复只保留一条");
        assert_eq!(
            suggestions[0].entries[0],
            ("ch_a".to_string(), "first".to_string()),
            "保留先配置的那条"
        );
    }

    #[test]
    fn suggestion_filters_aliases_already_configured() {
        let input = pairs(&[
            ("ch_a", &[("sonnet", "a"), ("opus", "a-opus")]),
            ("ch_b", &[("sonnet", "b"), ("opus", "b-opus")]),
        ]);
        let suggestions = suggest_aliases_from_mappings(&input);
        assert_eq!(suggestions.len(), 2);

        let remaining = filter_existing_aliases(suggestions, &["sonnet".to_string()]);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].alias, "opus");
    }
}
