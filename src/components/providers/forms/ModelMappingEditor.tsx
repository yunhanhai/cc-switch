import { useMemo } from "react";
import { AlertTriangle, Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { useModelPricing } from "@/lib/query/usage";

export interface ModelMappingRuleDraft {
  source: string;
  target: string;
  enabled: boolean;
  description?: string;
}

interface ModelMappingEditorProps {
  value: string;
  onChange: (value: string) => void;
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
}: ModelMappingEditorProps) {
  const { t } = useTranslation();
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
                placeholder="glm-5.3"
                aria-label={t("provider.modelMapping.target")}
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
