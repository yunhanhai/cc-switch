import { describe, expect, it } from "vitest";
import {
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
