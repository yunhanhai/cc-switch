import { describe, expect, it } from "vitest";
import en from "@/i18n/locales/en.json";
import ja from "@/i18n/locales/ja.json";
import zhTW from "@/i18n/locales/zh-TW.json";
import zh from "@/i18n/locales/zh.json";

type TranslationTree = Record<string, unknown>;

function flattenStrings(
  value: unknown,
  path: string[] = [],
  result = new Map<string, string>(),
): Map<string, string> {
  if (typeof value === "string") {
    result.set(path.join("."), value);
  } else if (typeof value === "object" && value !== null) {
    for (const [key, child] of Object.entries(value)) {
      flattenStrings(child, [...path, key], result);
    }
  }
  return result;
}

function interpolationVariables(value: string): string[] {
  return Array.from(
    value.matchAll(/\{\{\s*([^}]+?)\s*\}\}/g),
    ([, name]) => name,
  ).sort();
}

const reference = flattenStrings(en);
const piKeysOutsideNamespace = new Set([
  "apps.pi",
  "deeplink.api",
  "sessionManager.piDiscoveryUnavailable",
  "sessionManager.piRelativeSessionDir",
  "settings.browsePlaceholderPi",
  "settings.piConfigDir",
  "settings.piConfigDirDescription",
]);
const piReference = new Map(
  [...reference].filter(
    ([key]) => key.startsWith("pi.") || piKeysOutsideNamespace.has(key),
  ),
);
const piProductReferences = new Map(
  [...reference].filter(([, value]) => /\bPi\b/.test(value)),
);
const locales = [
  ["zh", zh],
  ["ja", ja],
  ["zh-TW", zhTW],
] as const;

describe("locale coverage", () => {
  // 全局键集合一致性：四语必须逐一对应（当前各 3005 键）。
  //
  // 现有用例只保证 `pi.*` 被翻译，因此像 `settings.oneClickInstall`
  // 这种「只存在于某一个语言、且代码从不引用」的孤儿键能长期逃过检查
  // （见提交 cebf2ace）。这里以 en 为基准做双向集合比对，同时捕获
  // 「漏译」与「多余/拼写错误」两类问题。
  it.each(locales)(
    "has exactly the same key set as en in %s",
    (_name, tree) => {
      const translations = flattenStrings(tree as TranslationTree);
      const missing = [...reference.keys()].filter(
        (key) => !translations.has(key),
      );
      const extra = [...translations.keys()].filter(
        (key) => !reference.has(key),
      );

      expect({ missing, extra }).toEqual({ missing: [], extra: [] });
    },
  );

  it.each(locales)(
    "preserves every interpolation variable outside pi.* in %s",
    (_name, tree) => {
      const translations = flattenStrings(tree as TranslationTree);
      // pi.* 已有专门用例；这里补其余全部命名空间，避免占位符写错
      // 导致运行时显示原文（如把 {{count}} 写成 {{cnt}}）。
      const mismatched = [...reference].flatMap(([key, expected]) => {
        if (key.startsWith("pi.") || piKeysOutsideNamespace.has(key)) {
          return [];
        }
        const actual = translations.get(key);
        return actual !== undefined &&
          interpolationVariables(actual).join("\0") !==
            interpolationVariables(expected).join("\0")
          ? [key]
          : [];
      });

      expect(mismatched).toEqual([]);
    },
  );

  it.each(locales)("covers every Pi translation key in %s", (_name, tree) => {
    const translations = flattenStrings(tree as TranslationTree);
    const missing = [...piReference.keys()].filter(
      (key) => !translations.has(key),
    );

    expect(missing).toEqual([]);
  });

  it.each(locales)(
    "preserves every Pi interpolation variable in %s",
    (_name, tree) => {
      const translations = flattenStrings(tree as TranslationTree);
      const mismatched = [...piReference].flatMap(([key, expected]) => {
        const actual = translations.get(key);
        return actual !== undefined &&
          interpolationVariables(actual).join("\0") !==
            interpolationVariables(expected).join("\0")
          ? [key]
          : [];
      });

      expect(mismatched).toEqual([]);
    },
  );

  it.each(locales)(
    "preserves explicit Pi product mentions in %s",
    (_name, tree) => {
      const translations = flattenStrings(tree as TranslationTree);
      const missingMentions = [...piProductReferences.keys()].filter((key) => {
        const actual = translations.get(key);
        return actual === undefined || !/\bPi\b/.test(actual);
      });

      expect(missingMentions).toEqual([]);
    },
  );
});
