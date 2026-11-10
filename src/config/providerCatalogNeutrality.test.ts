import { describe, expect, it } from "vitest";

import { claudeDesktopProviderPresets } from "./claudeDesktopProviderPresets";
import { providerPresets } from "./claudeProviderPresets";
import { codexProviderPresets } from "./codexProviderPresets";
import { geminiProviderPresets } from "./geminiProviderPresets";
import {
  grokBuildOfficialPreset,
  grokBuildProviderPresets,
} from "./grokBuildProviderPresets";
import { hermesProviderPresets } from "./hermesProviderPresets";
import { openclawProviderPresets } from "./openclawProviderPresets";
import { opencodeProviderPresets } from "./opencodeProviderPresets";
import { piProviderPresets } from "./piProviderPresets";
import { universalProviderPresets } from "./universalProviderPresets";

interface CatalogPreset {
  name: string;
  websiteUrl?: string;
  apiKeyUrl?: string;
  isPartner?: boolean;
  primePartner?: boolean;
  partnerPromotionKey?: string;
}

const catalogs: ReadonlyArray<readonly [string, readonly CatalogPreset[]]> = [
  ["Claude Code", providerPresets],
  ["Claude Desktop", claudeDesktopProviderPresets],
  ["Codex", codexProviderPresets],
  ["Gemini", geminiProviderPresets],
  ["Grok Build", [grokBuildOfficialPreset, ...grokBuildProviderPresets]],
  ["Hermes", hermesProviderPresets],
  ["OpenClaw", openclawProviderPresets],
  ["OpenCode", opencodeProviderPresets],
  ["Pi", piProviderPresets],
  ["Universal", universalProviderPresets],
];

const referralQueryKey =
  /^(aff|ref|invitecode|from|ic|ac|rc|ch|code|source|ytag|utm_.+)$/i;
const inheritedReferralPath =
  /(?:\/invite\/|\/i\/|\/go\/u\d+|\/agent\/register\/|\/activity\/ccswitch|\/VjM74M|\/nMvAvy)/i;

function findPreset(presets: readonly CatalogPreset[], name: string) {
  return presets.find((preset) => preset.name === name)!;
}

describe("independently maintained provider catalog", () => {
  it.each(catalogs)(
    "%s has neutral links and no inherited partnership declarations",
    (_surface, presets) => {
      for (const preset of presets) {
        expect(preset.isPartner, preset.name).toBeUndefined();
        expect(preset.primePartner, preset.name).toBeUndefined();
        for (const link of [preset.websiteUrl, preset.apiKeyUrl]) {
          if (!link) continue;
          const url = new URL(link);
          expect(url.protocol, `${preset.name}: ${link}`).toBe("https:");
          expect(url.pathname, `${preset.name}: ${link}`).not.toMatch(
            inheritedReferralPath,
          );
          expect(url.hostname, `${preset.name}: ${link}`).not.toBe(
            "s.qiniu.com",
          );
          for (const key of url.searchParams.keys()) {
            expect(key, `${preset.name}: ${link}`).not.toMatch(
              referralQueryKey,
            );
          }
        }
      }
    },
  );

  it("retains only the two legacy keys needed for Gemini authentication", () => {
    const keyedEntries = catalogs.flatMap(([surface, presets]) =>
      presets
        .filter((preset) => preset.partnerPromotionKey)
        .map((preset) => [surface, preset.name, preset.partnerPromotionKey]),
    );
    expect(keyedEntries).toEqual([
      ["Gemini", "Google Official", "google-official"],
      ["Gemini", "PackyCode", "packycode"],
    ]);
    expect(findPreset(geminiProviderPresets, "PackyCode").apiKeyUrl).toBe(
      "https://www.packyapi.ai/register",
    );
  });

  it("preserves console, purchase and SPA routing information", () => {
    expect(findPreset(providerPresets, "FennoAI").apiKeyUrl).toBe(
      "https://api.fenno.ai/register?redirect=/purchase?tab=subscription%26group=16",
    );
    expect(
      findPreset(claudeDesktopProviderPresets, "PatewayAI").apiKeyUrl,
    ).toBe("https://pateway.ai/#/");
    expect(findPreset(providerPresets, "DouBaoSeed").apiKeyUrl).toBe(
      "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey?apikey=%7B%7D",
    );
    expect(findPreset(opencodeProviderPresets, "Bailian").apiKeyUrl).toBe(
      "https://bailian.console.aliyun.com/#/api-key",
    );
  });
});
