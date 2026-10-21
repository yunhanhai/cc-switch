import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import {
  getModelCatalog,
  refreshModelCatalog,
  type CatalogModel,
  type ModelCatalogSnapshot,
} from "@/lib/api/model-fetch";

/** 轻量"仅显示可用模型"开关，供 ModelMappingEditor 复用。 */
export function ModelCatalogPicker({
  models,
  filterEnabled,
  onFilterEnabledChange,
}: {
  models: string[];
  filterEnabled?: boolean;
  onFilterEnabledChange?: (enabled: boolean) => void;
}) {
  const { t } = useTranslation();
  const [q, setQ] = useState("");
  const filtered = useMemo(() => {
    const query = q.trim().toLowerCase();
    if (!query) return models;
    return models.filter((m) => m.toLowerCase().includes(query));
  }, [models, q]);

  return (
    <div className="space-y-2 rounded-lg border border-border/40 bg-background/40 p-2.5">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs font-medium text-muted-foreground">
          {t("provider.modelCatalog.pickerTitle", {
            defaultValue: "可用模型",
          })}
        </span>
        <div className="flex items-center gap-1.5">
          <Checkbox
            id="catalog-filter"
            checked={!!filterEnabled}
            onCheckedChange={(checked) =>
              onFilterEnabledChange?.(checked === true)
            }
          />
          <Label htmlFor="catalog-filter" className="text-xs font-normal">
            {t("provider.modelCatalog.filterAvailable", {
              defaultValue: "仅可用模型",
            })}
          </Label>
        </div>
      </div>
      <Input
        placeholder={t("provider.modelCatalog.filterPlaceholder", {
          defaultValue: "筛选模型...",
        })}
        value={q}
        onChange={(e) => setQ(e.target.value)}
        className="h-7 text-xs"
      />
      <div className="flex flex-wrap gap-1.5">
        {filtered.length === 0 ? (
          <span className="text-xs text-muted-foreground">
            {t("provider.modelCatalog.empty", { defaultValue: "暂无模型" })}
          </span>
        ) : (
          filtered.slice(0, 80).map((model) => (
            <Badge
              key={model}
              variant="outline"
              className="font-mono text-[11px]"
            >
              {model}
            </Badge>
          ))
        )}
      </div>
      {filtered.length > 80 ? (
        <p className="text-[11px] text-muted-foreground">
          {t("provider.modelCatalog.more", {
            defaultValue: "还有 {{count}} 个",
            count: filtered.length - 80,
          })}
        </p>
      ) : null}
    </div>
  );
}

interface ModelCatalogPanelProps {
  baseUrl: string;
  apiKey: string;
  isFullUrl?: boolean;
  modelsUrl?: string;
  apiFormat?: string;
}

export function ModelCatalogPanel({
  baseUrl,
  apiKey,
  isFullUrl,
  modelsUrl,
  apiFormat,
}: ModelCatalogPanelProps) {
  const { t } = useTranslation();
  const [snapshot, setSnapshot] = useState<ModelCatalogSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [filter, setFilter] = useState("");
  const [showVendorOnly, setShowVendorOnly] = useState(false);

  const filtered = useMemo(() => {
    const models: CatalogModel[] = snapshot?.models ?? [];
    let list = models;
    if (showVendorOnly) {
      list = list.filter((m) => m.source === "vendor" || m.source === "both");
    }
    const q = filter.trim().toLowerCase();
    if (q) {
      list = list.filter(
        (m) =>
          m.id.toLowerCase().includes(q) ||
          m.displayName.toLowerCase().includes(q),
      );
    }
    return list;
  }, [snapshot, filter, showVendorOnly]);

  const fetchCatalog = async (force: boolean) => {
    if (!baseUrl.trim() || !apiKey.trim()) return;
    setLoading(true);
    try {
      const data = force
        ? await refreshModelCatalog(
            baseUrl,
            apiKey,
            isFullUrl,
            modelsUrl,
            undefined,
            {
              apiFormat,
            },
          )
        : await getModelCatalog(
            baseUrl,
            apiKey,
            isFullUrl,
            modelsUrl,
            undefined,
            {
              apiFormat,
            },
          );
      setSnapshot(data);
    } catch (e) {
      // 错误已在 snapshot.vendorError 中体现；静默失败仅用于离线场景
      setSnapshot((prev) =>
        prev
          ? { ...prev, vendorError: String(e) }
          : {
              version: 1,
              fetchedAtSecs: Math.floor(Date.now() / 1000),
              providerKey: baseUrl,
              models: [],
              vendorError: String(e),
            },
      );
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-3 rounded-xl border border-border/60 bg-card/40 p-3">
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-medium">
          {t("provider.modelCatalog.title", { defaultValue: "模型目录" })}
        </span>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={loading}
            onClick={() => fetchCatalog(false)}
          >
            {loading ? (
              <RefreshCw className="mr-1.5 h-3.5 w-3.5 animate-spin" />
            ) : (
              <RefreshCw className="mr-1.5 h-3.5 w-3.5" />
            )}
            {t("provider.modelCatalog.load", { defaultValue: "加载" })}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            disabled={loading}
            onClick={() => fetchCatalog(true)}
          >
            <RefreshCw className="mr-1.5 h-3.5 w-3.5" />
            {t("provider.modelCatalog.refresh", { defaultValue: "刷新" })}
          </Button>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <Input
          placeholder={t("provider.modelCatalog.filterPlaceholder", {
            defaultValue: "筛选模型...",
          })}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="h-8"
        />
        <label className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <input
            type="checkbox"
            checked={showVendorOnly}
            onChange={(e) => setShowVendorOnly(e.target.checked)}
          />
          {t("provider.modelCatalog.vendorOnly", {
            defaultValue: "仅厂商",
          })}
        </label>
      </div>
      {snapshot?.vendorError ? (
        <p className="text-xs text-amber-600 dark:text-amber-400">
          {snapshot.vendorError}
        </p>
      ) : null}
      <div className="max-h-64 space-y-1 overflow-auto pr-1">
        {filtered.length === 0 ? (
          <p className="py-6 text-center text-xs text-muted-foreground">
            {t("provider.modelCatalog.empty", { defaultValue: "暂无模型" })}
          </p>
        ) : (
          filtered.map((model) => (
            <div
              key={model.id}
              className="flex items-center justify-between gap-2 rounded-lg border border-border/40 bg-background/60 px-2.5 py-1.5"
            >
              <span className="truncate text-xs font-mono">{model.id}</span>
              <Badge
                variant="outline"
                className="shrink-0 text-[10px]"
                title={model.source}
              >
                {model.source}
              </Badge>
            </div>
          ))
        )}
      </div>
      {snapshot ? (
        <p className="text-[11px] text-muted-foreground">
          {t("provider.modelCatalog.fetchedAt", {
            defaultValue: "更新于 {{time}}",
            time: new Date(snapshot.fetchedAtSecs * 1000).toLocaleString(),
          })}
          {" · "}
          {t("provider.modelCatalog.count", {
            defaultValue: "{{count}} 个模型",
            count: snapshot.models.length,
          })}
        </p>
      ) : null}
    </div>
  );
}
