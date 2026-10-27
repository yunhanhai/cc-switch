// Claude → Codex 供应商配置转换
//
// 一个渠道商的 API Key 在 Claude Code 与 Codex 之间可复用。注意协议语义：
// Claude 供应商的 base_url 是 Anthropic Messages 端点（/v1/messages），而
// Codex 原生说 Responses——直连必然 404。转换因此固定 wire_api = "anthropic"，
// 由 CC Switch 本地路由做 Responses ⇄ Anthropic 桥接（判定链
// codex_provider_uses_anthropic 只认显式 wire_api / meta.apiFormat，绝不猜测
// base_url）。桥接路径上 forwarder 的 apply_codex_upstream_model 会把请求模型
// 覆写为 config 顶层 model（不在 modelCatalog 时），因此这里把源供应商的
// ANTHROPIC_MODEL 搬为顶层 model——网关是 Anthropic 协议，自家模型名才是它
// 认识的名字；缺省回退 Codex 通用默认。
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
wire_api = "anthropic"
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
  /** Codex 侧使用的模型名；缺省优先用源 ANTHROPIC_MODEL，否则用仓库预设默认 */
  modelName?: string;
}

/** 从 Claude env 里读主模型名（ANTHROPIC_MODEL），供桥接路径作为上游模型 */
const extractClaudeModel = (provider: Provider): string | undefined => {
  const config = provider.settingsConfig as Record<string, any>;
  const model = config?.env?.ANTHROPIC_MODEL;
  if (typeof model !== "string") return undefined;
  const trimmed = model.trim();
  return trimmed.length > 0 ? trimmed : undefined;
};

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
    options.modelName ?? extractClaudeModel(provider) ?? CODEX_DEFAULT_MODEL,
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
 * 由 Codex 列表按其自身排序规则落位。meta.apiFormat = "anthropic" 与
 * config 里的 wire_api 双保险：providerNeedsRouting / 后端桥接判定都认显式
 * 声明，切换该供应商时会提示先启动本地路由。转换不可行（无端点/无 Key）返回
 * undefined。
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
  meta: {
    apiFormat: "anthropic",
    ...(provider.meta ? provider.meta : {}),
  },
});
