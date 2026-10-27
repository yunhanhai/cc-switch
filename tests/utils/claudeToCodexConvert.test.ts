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
    expect(conversion?.config).toContain('wire_api = "responses"');
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

  it("keeps the Codex default model instead of copying the Anthropic model name", () => {
    const conversion = convertClaudeProviderToCodex(claudeProvider());

    expect(conversion?.config).toMatch(/^model = "gpt-5\.6-sol"$/m);
    expect(conversion?.config).not.toContain("kimi-k2-turbo-preview");
  });

  it("honors an explicit model name override", () => {
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
    expect(codexProvider.notes).toBe("团队主账号");
    expect(codexProvider.icon).toBe("kimi");
    expect(codexProvider.iconColor).toBe("#00A67E");
    expect(codexProvider.sortIndex).toBeUndefined();
  });

  it("omits the source provider's identity fields (id/createdAt)", () => {
    const conversion = convertClaudeProviderToCodex(fullProvider)!;
    const codexProvider = buildConvertedCodexProvider(fullProvider, conversion);

    expect(codexProvider).not.toHaveProperty("id");
    expect(codexProvider).not.toHaveProperty("createdAt");
  });
});
