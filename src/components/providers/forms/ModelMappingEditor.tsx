import { useId, useMemo } from "react";
import { AlertTriangle, Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { useModelPricing } from "@/lib/query/usage";
import { ModelCatalogPicker } from "./ModelCatalogPanel";

/** 常见"显示模型"名，用于 source 列的输入联想，降低手输成本。 */
const COMMON_DISPLAY_MODELS: readonly string[] = [
  "gpt-5.6-sol",
  "gpt-4o",
  "gpt-4o-mini",
  "o1",
  "o1-mini",
  "o3-mini",
  "claude-opus-4-6",
  "claude-sonnet-4-5",
  "claude-haiku-4-5",
  "claude-3-5-sonnet-20241022",
  "claude-3-5-haiku-20241022",
  "gemini-2.0-flash",
  "gemini-2.0-pro",
  "glm-5.3",
  "glm-4.7",
  "deepseek-chat",
  "deepseek-reasoner",
  "qwen3-max",
  "qwen2.5-coder-32b",
] as const;

export interface ModelMappingRuleDraft {
  source: string;
  target: string;
  enabled: boolean;
  description?: string;
}

interface ModelMappingEditorProps {
  value: string;
  onChange: (value: string) => void;
  /** 动态目录：供"目标模型"下拉/筛选使用（厂商拉取 + models.dev 兜底）。 */
  catalogModels?: string[];
  /** 是否仅显示目录中的可用模型（筛选模式）。 */
  catalogFilterEnabled?: boolean;
  onCatalogFilterEnabledChange?: (enabled: boolean) => void;
  /** 选中目录中的模型时回填到哪条规则的目标列；缺省为最后一条空 target */
  onCatalogSelect?: (modelId: string) => void;
}

const parseConfig = (value: string): Record<string, unknown> => {
  try {
    const parsed: unknown = JSON.parse(value);
    return typeof parsed === "object" &&
      parsed !== null &&
      !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : {};
  } catch {
    return {};
  }
};

const parseRules = (
  config: Record<string, unknown>,
): ModelMappingRuleDraft[] => {
  if (!Array.isArray(config.model_mapping)) return [];
  return config.model_mapping.flatMap((item) => {
    if (typeof item !== "object" || item === null || Array.isArray(item)) {
      return [];
    }
    const rule = item as Record<string, unknown>;
    return [
      {
        source: typeof rule.source === "string" ? rule.source : "",
        target: typeof rule.target === "string" ? rule.target : "",
        enabled: rule.enabled !== false,
        ...(typeof rule.description === "string"
          ? { description: rule.description }
          : {}),
      },
    ];
  });
};

export function ModelMappingEditor({
  value,
  onChange,
  catalogModels,
  catalogFilterEnabled,
  onCatalogFilterEnabledChange,
  onCatalogSelect,
}: ModelMappingEditorProps) {
  const { t } = useTranslation();
  const catalogListId = useId();
  const sourceListId = useId();
  const config = useMemo(() => parseConfig(value), [value]);
  const rules = useMemo(() => parseRules(config), [config]);

  // 定价表：映射目标若没有定价，用量统计中该模型的费用会显示为 0，
  // 因此对未定价目标给出显式警告（通配符目标无法判定，跳过）。
  const { data: pricing } = useModelPricing();
  const pricedIds = useMemo(() => {
    const ids = new Set<string>();
    for (const item of pricing ?? []) {
      ids.add(item.modelId.trim().toLowerCase());
    }
    return ids;
  }, [pricing]);
  const unpricedTargets = useMemo(() => {
    if (!pricing) return [];
    return rules
      .filter((rule) => rule.enabled && rule.target.trim())
      .map((rule) => rule.target.trim())
      .filter((target) => !target.includes("*"))
      .filter((target) => !pricedIds.has(target.toLowerCase()));
  }, [rules, pricing, pricedIds]);

  const duplicateSources = useMemo(() => {
    const seen = new Map<string, number>();
    for (const r of rules) {
      const key = r.source.trim().toLowerCase();
      if (!key) continue;
      seen.set(key, (seen.get(key) ?? 0) + 1);
    }
    return new Set(
      [...seen.entries()].filter(([, c]) => c > 1).map(([k]) => k),
    );
  }, [rules]);

  const updateRules = (nextRules: ModelMappingRuleDraft[]) => {
    const nextConfig = {
      ...config,
      model_mapping: nextRules.map((rule) => ({
        source: rule.source,
        target: rule.target,
        enabled: rule.enabled,
        ...(rule.description?.trim()
          ? { description: rule.description.trim() }
          : {}),
      })),
    };
    onChange(JSON.stringify(nextConfig, null, 2));
  };

  const handleCatalogSelect = (modelId: string) => {
    if (onCatalogSelect) {
      onCatalogSelect(modelId);
      return;
    }
    // 默认策略：优先填到最后一条空 target 的规则，否则追加一条
    let targetIndex = -1;
    for (let i = rules.length - 1; i >= 0; i -= 1) {
      if (!rules[i]!.target.trim()) {
        targetIndex = i;
        break;
      }
    }
    if (targetIndex >= 0) {
      const next = [...rules];
      next[targetIndex] = { ...next[targetIndex]!, target: modelId };
      updateRules(next);
    } else {
      updateRules([...rules, { source: "", target: modelId, enabled: true }]);
    }
  };

  return (
    <section className="space-y-3 rounded-lg border border-border/50 bg-muted/20 p-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="font-medium">{t("provider.modelMapping.title")}</h3>
          <p className="mt-1 text-xs text-muted-foreground">
            {t("provider.modelMapping.description")}
          </p>
        </div>
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={() =>
            updateRules([...rules, { source: "", target: "", enabled: true }])
          }
        >
          <Plus className="mr-1 h-4 w-4" />
          {t("provider.modelMapping.add")}
        </Button>
      </div>

      {catalogModels && catalogModels.length > 0 ? (
        <ModelCatalogPicker
          models={catalogModels}
          filterEnabled={catalogFilterEnabled}
          onFilterEnabledChange={onCatalogFilterEnabledChange}
          onSelect={handleCatalogSelect}
        />
      ) : null}

      {unpricedTargets.length > 0 && (
        <div className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          <span>
            {t("provider.modelMapping.unpricedTargets", {
              targets: [...new Set(unpricedTargets)].join(", "),
            })}
          </span>
        </div>
      )}
      {duplicateSources.size > 0 && (
        <div className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          <span>
            {t("provider.modelMapping.duplicateSources", {
              defaultValue:
                "存在重复的显示模型：{{sources}}，后一条会覆盖前一条",
              sources: [...duplicateSources].join(", "),
            })}
          </span>
        </div>
      )}
      <datalist id={sourceListId}>
        {COMMON_DISPLAY_MODELS.map((m) => (
          <option key={m} value={m} />
        ))}
      </datalist>
      {catalogModels && catalogModels.length > 0 ? (
        <datalist id={catalogListId}>
          {catalogModels.slice(0, 200).map((m) => (
            <option key={m} value={m} />
          ))}
        </datalist>
      ) : null}

      {rules.length === 0 ? (
        <p className="text-xs text-muted-foreground">
          {t("provider.modelMapping.empty")}
        </p>
      ) : (
        <div className="space-y-2">
          {rules.map((rule, index) => (
            <div
              key={`model-mapping-${index}`}
              className="grid grid-cols-[auto_minmax(0,1fr)_minmax(0,1fr)_auto] items-center gap-2"
            >
              <Switch
                checked={rule.enabled}
                aria-label={t("provider.modelMapping.enabled")}
                onCheckedChange={(enabled) => {
                  const next = [...rules];
                  next[index] = { ...rule, enabled };
                  updateRules(next);
                }}
              />
              <Input
                value={rule.source}
                list={sourceListId}
                title={t("provider.modelCatalog.sourceInputHint", {
                  defaultValue: "常用显示模型可选，下拉直接填入",
                })}
                placeholder="gpt-5.6-sol"
                aria-label={t("provider.modelMapping.source")}
                onChange={(event) => {
                  const next = [...rules];
                  next[index] = { ...rule, source: event.target.value };
                  updateRules(next);
                }}
              />
              <Input
                value={rule.target}
                list={
                  catalogModels && catalogModels.length > 0
                    ? catalogListId
                    : undefined
                }
                placeholder="glm-5.3"
                aria-label={t("provider.modelMapping.target")}
                title={
                  catalogModels && catalogModels.length > 0
                    ? t("provider.modelCatalog.targetInputHint", {
                        defaultValue:
                          "已接入目录，可直接下拉选择或点击上方徽标填入",
                      })
                    : undefined
                }
                onChange={(event) => {
                  const next = [...rules];
                  next[index] = { ...rule, target: event.target.value };
                  updateRules(next);
                }}
              />
              <Button
                type="button"
                size="icon"
                variant="ghost"
                aria-label={t("provider.modelMapping.remove")}
                onClick={() => updateRules(rules.filter((_, i) => i !== index))}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
