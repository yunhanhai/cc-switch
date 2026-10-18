import { useMemo } from "react";
import { Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";

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
          <h3 className="font-medium">
            {t("provider.modelMapping.title", {
              defaultValue: "模型映射",
            })}
          </h3>
          <p className="mt-1 text-xs text-muted-foreground">
            {t("provider.modelMapping.description", {
              defaultValue:
                "客户端使用显示模型名，代理转发时替换为实际模型。支持 * 通配符，精确规则优先。",
            })}
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
          {t("provider.modelMapping.add", { defaultValue: "添加规则" })}
        </Button>
      </div>

      {rules.length === 0 ? (
        <p className="text-xs text-muted-foreground">
          {t("provider.modelMapping.empty", {
            defaultValue: "尚未配置规则，未命中时保留客户端模型名。",
          })}
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
                aria-label={t("provider.modelMapping.enabled", {
                  defaultValue: "启用规则",
                })}
                onCheckedChange={(enabled) => {
                  const next = [...rules];
                  next[index] = { ...rule, enabled };
                  updateRules(next);
                }}
              />
              <Input
                value={rule.source}
                placeholder="gpt-5.6-sol"
                aria-label={t("provider.modelMapping.source", {
                  defaultValue: "显示模型",
                })}
                onChange={(event) => {
                  const next = [...rules];
                  next[index] = { ...rule, source: event.target.value };
                  updateRules(next);
                }}
              />
              <Input
                value={rule.target}
                placeholder="glm-5.3"
                aria-label={t("provider.modelMapping.target", {
                  defaultValue: "实际模型",
                })}
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
                aria-label={t("provider.modelMapping.remove", {
                  defaultValue: "删除规则",
                })}
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
