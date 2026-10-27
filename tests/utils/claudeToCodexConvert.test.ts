import { describe, expect, it } from "vitest";
import {
  buildConvertedCodexProvider,
  convertClaudeProviderToCodex,
  extractClaudeBaseUrl,
  generateCodexConfigFromClaudeEndpoint,
} from "@/utils/claudeToCodexConvert";
import type { Provider } from "@/types";

const claudeProvider = (overrides: Partial<Provider> = {}): Provider => ({
  id: "claude-provider-1",
  name: "Kimi",
  settingsConfig: {
    env: {
      ANTHROPIC_BASE_URL: "https://api.kimi.com/anthropic",
      ANTHROPIC_AUTH_TOKEN: "sk-kimi-key",
      ANTHROPIC_MODEL: "kimi-k2-turbo-preview",
    },
  },
  ...overrides,
});

describe("convertClaudeProviderToCodex", () => {
  it("maps the endpoint and API key into Codex auth + config TOML", () => {
    const conversion = convertClaudeProviderToCodex(claudeProvider());

    expect(conversion).toBeDefined();
    expect(conversion?.auth).toEqual({ OPENAI_API_KEY: "sk-kimi-key" });
    expect(conversion?.config).toContain('model_provider = "custom"');
    expect(conversion?.config).toContain("[model_providers.custom]");
    expect(conversion?.config).toContain(
      'base_url = "https://api.kimi.com/anthropic"',
    );
    expect(conversion?.config).toContain('name = "Kimi"');
    // Claude 端点说的是 Anthropic Messages 协议，必须显式声明让本地路由桥接；
    // 用 "responses" 会对 Anthropic 网关发 /responses 而直接 404。
    expect(conversion?.config).toContain('wire_api = "anthropic"');
    expect(conversion?.config).toContain("requires_openai_auth = true");
  });

  it("prefers ANTHROPIC_AUTH_TOKEN over ANTHROPIC_API_KEY", () => {
    const conversion = convertClaudeProviderToCodex(
      claudeProvider({
        settingsConfig: {
          env: {
            ANTHROPIC_BASE_URL: "https://api.example.com",
            ANTHROPIC_AUTH_TOKEN: "sk-auth-token",
            ANTHROPIC_API_KEY: "sk-api-key",
          },
        },
      }),
    );

    expect(conversion?.auth.OPENAI_API_KEY).toBe("sk-auth-token");
  });

  it("falls back to ANTHROPIC_API_KEY when the token field is absent", () => {
    const conversion = convertClaudeProviderToCodex(
      claudeProvider({
        settingsConfig: {
          env: {
            ANTHROPIC_BASE_URL: "https://api.example.com",
            ANTHROPIC_API_KEY: "sk-api-key",
          },
        },
      }),
    );

    expect(conversion?.auth.OPENAI_API_KEY).toBe("sk-api-key");
  });

  it("carries the Anthropic model name as the upstream model for the bridged gateway", () => {
    const conversion = convertClaudeProviderToCodex(claudeProvider());

    // 桥接后请求模型会被覆写为 config 顶层 model；Anthropic 协议网关认识的
    // 是自家模型名，因此源 ANTHROPIC_MODEL 是正确的搬运对象。
    expect(conversion?.config).toMatch(/^model = "kimi-k2-turbo-preview"$/m);
  });

  it("falls back to the Codex default model when ANTHROPIC_MODEL is absent", () => {
    const conversion = convertClaudeProviderToCodex(
      claudeProvider({
        settingsConfig: {
          env: {
            ANTHROPIC_BASE_URL: "https://api.example.com",
            ANTHROPIC_AUTH_TOKEN: "sk-x",
          },
        },
      }),
    );

    expect(conversion?.config).toMatch(/^model = "gpt-5\.6-sol"$/m);
  });

  it("honors an explicit model name override over ANTHROPIC_MODEL", () => {
    const conversion = convertClaudeProviderToCodex(claudeProvider(), {
      modelName: "gpt-5-codex",
    });

    expect(conversion?.config).toMatch(/^model = "gpt-5-codex"$/m);
  });

  it("escapes quotes and backslashes in TOML basic strings", () => {
    const config = generateCodexConfigFromClaudeEndpoint(
      'Acme "Pro"\\CN',
      "https://api.acme.com/v1\\",
    );

    expect(config).toContain('name = "Acme \\"Pro\\"\\\\CN"');
    expect(config).toContain('base_url = "https://api.acme.com/v1\\\\"');
  });

  it("returns undefined for official providers without a custom endpoint", () => {
    const official = claudeProvider({
      settingsConfig: { env: { ANTHROPIC_AUTH_TOKEN: "sk-x" } },
    });

    expect(extractClaudeBaseUrl(official)).toBeUndefined();
    expect(convertClaudeProviderToCodex(official)).toBeUndefined();
  });

  it("returns undefined when the endpoint is an empty string", () => {
    const blank = claudeProvider({
      settingsConfig: {
        env: { ANTHROPIC_BASE_URL: "   ", ANTHROPIC_AUTH_TOKEN: "sk-x" },
      },
    });

    expect(convertClaudeProviderToCodex(blank)).toBeUndefined();
  });

  it("returns undefined when no API key is present", () => {
    const noKey = claudeProvider({
      settingsConfig: {
        env: { ANTHROPIC_BASE_URL: "https://api.example.com" },
      },
    });

    expect(convertClaudeProviderToCodex(noKey)).toBeUndefined();
  });

  it("returns undefined when settingsConfig is not an object", () => {
    const broken = claudeProvider();
    (broken.settingsConfig as unknown) = "not-an-object";

    expect(convertClaudeProviderToCodex(broken)).toBeUndefined();
  });

  it("does not mutate the original provider", () => {
    const provider = claudeProvider();
    const snapshot = JSON.stringify(provider);

    convertClaudeProviderToCodex(provider);

    expect(JSON.stringify(provider)).toBe(snapshot);
  });
});

describe("buildConvertedCodexProvider", () => {
  const fullProvider = claudeProvider({
    id: "claude-kimi",
    createdAt: 1700000000000,
    sortIndex: 7,
    websiteUrl: "https://kimi.com",
    category: "cn_official",
    notes: "团队主账号",
    icon: "kimi",
    iconColor: "#00A67E",
  });

  it("carries conversion output into the Codex settingsConfig", () => {
    const conversion = convertClaudeProviderToCodex(fullProvider)!;
    const codexProvider = buildConvertedCodexProvider(fullProvider, conversion);

    expect(codexProvider.settingsConfig.auth).toEqual({
      OPENAI_API_KEY: "sk-kimi-key",
    });
    expect(codexProvider.settingsConfig.config).toBe(conversion.config);
  });

  it("suffixed the name with (Codex) for list disambiguation", () => {
    const conversion = convertClaudeProviderToCodex(fullProvider)!;
    const codexProvider = buildConvertedCodexProvider(fullProvider, conversion);

    expect(codexProvider.name).toBe("Kimi (Codex)");
  });

  it("carries display metadata but not sortIndex", () => {
    const conversion = convertClaudeProviderToCodex(fullProvider)!;
    const codexProvider = buildConvertedCodexProvider(fullProvider, conversion);

    expect(codexProvider.websiteUrl).toBe("https://kimi.com");
    expect(codexProvider.category).toBe("cn_official");
    expect(codexProvider.icon).toBe("kimi");
    expect(codexProvider.iconColor).toBe("#00A67E");
    expect(codexProvider.sortIndex).toBeUndefined();
  });

  it("appends the bridge runtime condition to notes", () => {
    const conversion = convertClaudeProviderToCodex(fullProvider)!;
    const codexProvider = buildConvertedCodexProvider(fullProvider, conversion);

    expect(codexProvider.notes).toContain("团队主账号");
    expect(codexProvider.notes).toContain("需先启动本地路由");
  });

  it("does not duplicate the bridge note when already present (idempotent)", () => {
    const conversion = convertClaudeProviderToCodex(fullProvider)!;
    const once = buildConvertedCodexProvider(fullProvider, conversion);
    const convertedProvider = { ...fullProvider, notes: once.notes };
    const twice = buildConvertedCodexProvider(
      convertedProvider,
      conversion,
    ).notes;

    expect(twice).toBe(once.notes);
  });

  it("declares the anthropic api format so routing prompts and bridging engage", () => {
    const conversion = convertClaudeProviderToCodex(fullProvider)!;
    const codexProvider = buildConvertedCodexProvider(fullProvider, conversion);

    expect(codexProvider.meta?.apiFormat).toBe("anthropic");
  });

  it("does not let a stale source meta.apiFormat override the anthropic declaration", () => {
    const conversion = convertClaudeProviderToCodex(fullProvider)!;
    const codexProvider = buildConvertedCodexProvider(fullProvider, conversion);

    // 全量展开源 meta 时 anthropic 必须排在前、不可被覆盖
    expect(Object.keys(codexProvider.meta ?? {})[0]).toBe("apiFormat");
    expect(codexProvider.meta?.apiFormat).toBe("anthropic");
  });

  it("omits the source provider's identity fields (id/createdAt)", () => {
    const conversion = convertClaudeProviderToCodex(fullProvider)!;
    const codexProvider = buildConvertedCodexProvider(fullProvider, conversion);

    expect(codexProvider).not.toHaveProperty("id");
    expect(codexProvider).not.toHaveProperty("createdAt");
  });
});
