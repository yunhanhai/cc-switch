// Claude → Codex 供应商配置转换
//
// 一个渠道商（base_url + API key）在 Claude Code 与 Codex 之间是可复用的；
// 模型名语义不同（Anthropic 系 vs GPT 系），不随转换搬运，保留 Codex 侧
// 预设的默认模型（与 codexProviderPresets / deeplink 导入的行为一致）。
import type { Provider } from "@/types";
import { getApiKeyFromConfig } from "@/utils/providerConfigUtils";

const CODEX_DEFAULT_MODEL = "gpt-5.6-sol";

const escapeTomlBasicString = (value: string): string =>
  value.replace(/["\\\u0000-\u001f]/g, (ch) => {
    if (ch === '"') return '\\"';
    if (ch === "\\") return "\\\\";
    return `\\u${ch.charCodeAt(0).toString(16).padStart(4, "0")}`;
  });

const tomlBasicString = (value: string): string =>
  `"${escapeTomlBasicString(value)}"`;

export interface ClaudeToCodexConversion {
  auth: Record<string, any>;
  config: string;
}

/** 生成 Codex 的 config.toml（[model_providers.custom] 指向原渠道端点） */
export const generateCodexConfigFromClaudeEndpoint = (
  providerName: string,
  baseUrl: string,
  modelName: string = CODEX_DEFAULT_MODEL,
): string => {
  const configText = (value: string) => tomlBasicString(value);

  return `model_provider = "custom"
model = ${configText(modelName)}
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.custom]
name = ${configText(providerName)}
base_url = ${configText(baseUrl)}
wire_api = "responses"
requires_openai_auth = true`;
};

/** 从 Claude 供应商的 env 读取自定义端点；官方直连（无端点）返回 undefined */
export const extractClaudeBaseUrl = (
  provider: Provider,
): string | undefined => {
  const config = provider.settingsConfig as Record<string, any>;
  const baseUrl = config?.env?.ANTHROPIC_BASE_URL;
  if (typeof baseUrl !== "string") return undefined;
  const trimmed = baseUrl.trim();
  return trimmed.length > 0 ? trimmed : undefined;
};

export interface ConvertClaudeToCodexOptions {
  /** Codex 侧使用的模型名；缺省用仓库预设默认 */
  modelName?: string;
}

/**
 * 把 Claude 供应商的 API 设置转换为 Codex 的 settingsConfig。
 *
 * 端点 + API Key 是跨渠道可复用的部分；转换不改变原供应商。
 * 无自定义端点（官方直连）或无 API Key 时返回 undefined，调用方应提示用户。
 */
export const convertClaudeProviderToCodex = (
  provider: Provider,
  options: ConvertClaudeToCodexOptions = {},
): ClaudeToCodexConversion | undefined => {
  const baseUrl = extractClaudeBaseUrl(provider);
  if (!baseUrl) return undefined;

  const apiKey = getApiKeyFromConfig(
    JSON.stringify(provider.settingsConfig ?? {}),
  );
  if (!apiKey) return undefined;

  const config = generateCodexConfigFromClaudeEndpoint(
    provider.name,
    baseUrl,
    options.modelName,
  );

  return {
    auth: { OPENAI_API_KEY: apiKey },
    config,
  };
};

/** App.tsx::handleConvertToCodex 构造新 Codex Provider 时延续的字段来源 */
export interface ConvertedCodexProvider
  extends Omit<Provider, "id" | "createdAt"> {
  settingsConfig: { auth: Record<string, any>; config: string };
}

/**
 * 构造转换后的 Codex Provider 对象（不含 id/createdAt，由 add mutation 生成）。
 *
 * 展示性元数据（icon/category/官网/备注）延续原供应商——它们描述的是同一个
 * 渠道商；名字加 " (Codex)" 后缀以便在两个列表中区分；sortIndex 不延续，
 * 由 Codex 列表按其自身排序规则落位。转换不可行（无端点/无 Key）返回 undefined。
 */
export const buildConvertedCodexProvider = (
  provider: Provider,
  conversion: ClaudeToCodexConversion,
): ConvertedCodexProvider => ({
  name: `${provider.name} (Codex)`,
  settingsConfig: {
    auth: conversion.auth,
    config: conversion.config,
  },
  websiteUrl: provider.websiteUrl,
  category: provider.category,
  sortIndex: undefined,
  notes: provider.notes,
  icon: provider.icon,
  iconColor: provider.iconColor,
});
