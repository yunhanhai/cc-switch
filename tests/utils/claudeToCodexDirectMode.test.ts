import { describe, expect, it } from "vitest";
import {
  appendDirectRuntimeNote,
  deriveOpenAiBaseUrlFromAnthropic,
  generateDirectCodexConfig,
  shouldUpgradeToDirectMode,
} from "@/utils/claudeToCodexConvert";

describe("deriveOpenAiBaseUrlFromAnthropic", () => {
  it("strips the known /anthropic compatibility suffix", () => {
    expect(
      deriveOpenAiBaseUrlFromAnthropic("https://api.kimi.com/anthropic"),
    ).toBe("https://api.kimi.com");
  });

  it("strips /claude, /claude-code and /messages suffixes", () => {
    expect(deriveOpenAiBaseUrlFromAnthropic("https://gw.example/claude")).toBe(
      "https://gw.example",
    );
    expect(
      deriveOpenAiBaseUrlFromAnthropic("https://gw.example/claude-code"),
    ).toBe("https://gw.example");
    expect(
      deriveOpenAiBaseUrlFromAnthropic("https://gw.example/messages"),
    ).toBe("https://gw.example");
  });

  it("is case-insensitive on the suffix", () => {
    expect(
      deriveOpenAiBaseUrlFromAnthropic("https://api.example.com/Anthropic"),
    ).toBe("https://api.example.com");
  });

  it("trims trailing slashes before matching", () => {
    expect(
      deriveOpenAiBaseUrlFromAnthropic("https://api.example.com/anthropic/"),
    ).toBe("https://api.example.com");
  });

  it("returns undefined for a bare origin (nothing to strip)", () => {
    expect(
      deriveOpenAiBaseUrlFromAnthropic("https://api.example.com"),
    ).toBeUndefined();
    expect(
      deriveOpenAiBaseUrlFromAnthropic("https://api.example.com/"),
    ).toBeUndefined();
  });

  it("returns undefined for unknown path shapes (unsafe to guess)", () => {
    expect(
      deriveOpenAiBaseUrlFromAnthropic("https://api.example.com/compat/v1"),
    ).toBeUndefined();
  });

  it("returns undefined for empty input", () => {
    expect(deriveOpenAiBaseUrlFromAnthropic("")).toBeUndefined();
    expect(deriveOpenAiBaseUrlFromAnthropic("   ")).toBeUndefined();
  });
});

describe("generateDirectCodexConfig", () => {
  it("emits wire_api = responses against the OpenAI candidate endpoint", () => {
    const config = generateDirectCodexConfig(
      "Kimi",
      "https://api.kimi.com",
      "kimi-k2-turbo-preview",
    );

    expect(config).toContain('wire_api = "responses"');
    expect(config).toContain('base_url = "https://api.kimi.com"');
    expect(config).toContain('model = "kimi-k2-turbo-preview"');
    expect(config).toContain("[model_providers.custom]");
    expect(config).not.toContain("anthropic");
  });
});

describe("appendDirectRuntimeNote", () => {
  it("appends the direct-mode note with the source name", () => {
    const notes = appendDirectRuntimeNote(undefined, "Kimi");
    expect(notes).toContain("Kimi");
    expect(notes).toContain("直连模式");
    expect(notes).toContain("无需本地路由");
  });

  it("keeps existing notes and appends on its own line", () => {
    const notes = appendDirectRuntimeNote("团队主账号", "Kimi");
    expect(notes).toContain("团队主账号");
    expect(notes).toContain("直连模式");
    expect(notes).toMatch(/\n/);
  });

  it("is idempotent — does not duplicate the direct note", () => {
    const once = appendDirectRuntimeNote(undefined, "Kimi");
    expect(appendDirectRuntimeNote(once, "Kimi")).toBe(once);
  });
});

describe("shouldUpgradeToDirectMode", () => {
  const models = [{ id: "kimi-k2-turbo-preview" }, { id: "kimi-k1" }];

  it("upgrades when the source model is served by the OpenAI endpoint", () => {
    expect(shouldUpgradeToDirectMode(models, "kimi-k2-turbo-preview")).toBe(
      true,
    );
  });

  it("does NOT upgrade when the endpoint only serves other models", () => {
    // 这正是修复的缺口：端点可达且有模型，但源模型不在其中 →
    // 直连会指向上游不认识的模型，必须保持桥接。
    expect(shouldUpgradeToDirectMode(models, "claude-sonnet-4")).toBe(false);
  });

  it("does NOT upgrade when there is no source model to verify", () => {
    expect(shouldUpgradeToDirectMode(models, undefined)).toBe(false);
    expect(shouldUpgradeToDirectMode(models, "  ")).toBe(false);
  });

  it("does NOT upgrade for an empty or missing model list", () => {
    expect(shouldUpgradeToDirectMode([], "kimi-k2-turbo-preview")).toBe(false);
    expect(shouldUpgradeToDirectMode(undefined, "kimi-k2-turbo-preview")).toBe(
      false,
    );
    expect(shouldUpgradeToDirectMode(null, "kimi-k2-turbo-preview")).toBe(
      false,
    );
  });

  it("matches case-insensitively and ignores surrounding whitespace", () => {
    expect(shouldUpgradeToDirectMode(models, "  Kimi-K2-Turbo-Preview ")).toBe(
      true,
    );
  });

  it("accepts plain string entries as well as {id} objects", () => {
    expect(
      shouldUpgradeToDirectMode(
        ["kimi-k2-turbo-preview"],
        "kimi-k2-turbo-preview",
      ),
    ).toBe(true);
  });

  it("ignores entries without a usable id", () => {
    expect(
      shouldUpgradeToDirectMode([{}, { id: "" }], "kimi-k2-turbo-preview"),
    ).toBe(false);
  });
});
