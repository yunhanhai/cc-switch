/**
 * 模型别名面板（同模型跨渠道无缝切换）
 *
 * 一个逻辑模型（如 `sonnet`）在多个渠道各有不同物理名。配置后：
 * - 选路阶段会跳过不认识该逻辑模型的渠道
 * - 转发阶段按当前渠道把模型名换成该渠道的物理名
 *
 * 因此「渠道断了自动换」时，模型名会跟着换，用户侧无感。
 */
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Pencil, Plus, Sparkles, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { useProvidersQuery } from "@/lib/query";
import {
  useModelAliases,
  useModelAliasSuggestions,
  useSetModelAliasEntries,
} from "@/lib/query/model-aliases";
import { groupModelAliases } from "@/lib/api/model-aliases";
import type { ProxyAppId } from "@/config/appConfig";

/**
 * 该 app 的模型名是否位于请求体（`body["model"]`）里。
 *
 * **Gemini 例外**：模型名在 URL 路径中（`/v1beta/models/<model>:generateContent`），
 * 转发时改 body 不生效。后端已把 Gemini 排除在 alias 路由之外（选路不过滤、
 * 转发不注入索引），这里同步禁用编辑，避免用户配了却完全不生效。
 *
 * 与后端 `proxy::handler_context::model_name_lives_in_body` 必须保持一致。
 */
const modelNameLivesInBody = (appType: ProxyAppId) => appType !== "gemini";

interface DraftEntry {
  providerId: string;
  targetModel: string;
}

interface DraftGroup {
  alias: string;
  entries: DraftEntry[];
}

export function ModelAliasPanel({ appType }: { appType: ProxyAppId }) {
  const { t } = useTranslation();
  const editable = modelNameLivesInBody(appType);
  const { data: aliases = [], isLoading } = useModelAliases(appType);
  const { data: providersData } = useProvidersQuery(appType);
  const setEntries = useSetModelAliasEntries();

  const providers = useMemo(
    () => Object.values(providersData?.providers ?? {}),
    [providersData],
  );
  const groups = useMemo(() => groupModelAliases(aliases), [aliases]);

  const [createAlias, setCreateAlias] = useState("");
  const [draft, setDraft] = useState<DraftGroup | null>(null);
  // 建议按需扫描：只有用户点了按钮才查询（要扫所有 provider 的 settings_config）
  const [scanEnabled, setScanEnabled] = useState(false);
  const { data: suggestions = [], isFetching: isScanning } =
    useModelAliasSuggestions(appType, scanEnabled);

  const providerName = (providerId: string) =>
    providers.find(
      (provider) =>
        String((provider as unknown as { id: string }).id) === providerId,
    )?.name ?? providerId;

  const beginEdit = (alias: string, entries: DraftEntry[]) => {
    setDraft({ alias, entries: entries.map((entry) => ({ ...entry })) });
  };

  const beginCreate = () => {
    const alias = createAlias.trim();
    if (!alias) return;
    setDraft({ alias, entries: [{ providerId: "", targetModel: "" }] });
    setCreateAlias("");
  };

  const saveDraft = () => {
    if (!draft) return;
    // 过滤掉空行：用户可能点了「添加渠道」但没填
    const entries = draft.entries
      .map((entry) => ({
        providerId: entry.providerId.trim(),
        targetModel: entry.targetModel.trim(),
      }))
      .filter((entry) => entry.providerId && entry.targetModel);
    setEntries.mutate(
      {
        appType,
        alias: draft.alias,
        entries: entries.map(
          (entry) => [entry.providerId, entry.targetModel] as [string, string],
        ),
      },
      { onSuccess: () => setDraft(null) },
    );
  };

  if (isLoading) {
    return (
      <div className="py-6 text-center text-sm text-muted-foreground">
        {t("common.loading")}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-start justify-between gap-3">
        <div className="space-y-1">
          <h3 className="text-sm font-medium">
            {t("modelAliases.title", { defaultValue: "模型别名" })}
          </h3>
          <p className="text-xs text-muted-foreground">
            {t("modelAliases.hint", {
              defaultValue:
                "同一个逻辑模型在不同渠道叫法不同时，配置这里即可跨渠道无缝切换",
            })}
          </p>
        </div>
        {groups.length > 0 && (
          <Badge variant="secondary" className="shrink-0 text-[10px]">
            {t("modelAliases.active", { defaultValue: "已启用" })}
          </Badge>
        )}
      </div>

      {!editable && (
        <p className="rounded-md border border-amber-500/30 bg-amber-500/10 px-2 py-1.5 text-[11px] text-amber-700 dark:text-amber-400">
          {t("modelAliases.geminiUnsupported", {
            defaultValue:
              "Gemini 的模型名在 URL 路径中，暂不支持模型别名路由；此处的配置不会生效",
          })}
        </p>
      )}

      {/* 从现有 model_mapping 归并建议 */}
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          variant="outline"
          className="h-7 text-xs"
          disabled={isScanning || !editable}
          onClick={() => setScanEnabled(true)}
        >
          <Sparkles className="mr-1 h-3 w-3" />
          {isScanning
            ? t("modelAliases.scanning", { defaultValue: "扫描中…" })
            : t("modelAliases.scan", {
                defaultValue: "从现有映射生成建议",
              })}
        </Button>
        {scanEnabled && !isScanning && suggestions.length === 0 && (
          <span className="text-[11px] text-muted-foreground">
            {t("modelAliases.noSuggestions", {
              defaultValue:
                "没有可归并的映射（需同一模型名出现在两个以上渠道）",
            })}
          </span>
        )}
      </div>

      {suggestions.length > 0 && (
        <div className="space-y-2 rounded-lg border border-primary/30 bg-primary/5 p-3">
          <p className="text-xs font-medium text-primary">
            {t("modelAliases.suggestionsTitle", {
              defaultValue: "发现 {{count}} 组可归并的模型",
              count: suggestions.length,
            })}
          </p>
          {suggestions.map((suggestion) => (
            <div
              key={suggestion.alias}
              className="flex items-center gap-2 rounded border border-border/40 bg-background/60 px-2 py-1.5"
            >
              <span className="w-24 shrink-0 truncate font-mono text-xs font-medium">
                {suggestion.alias}
              </span>
              <span className="flex-1 truncate text-[11px] text-muted-foreground">
                {suggestion.entries
                  .map(
                    ([providerId, target]) =>
                      `${providerName(providerId)} → ${target}`,
                  )
                  .join("  |  ")}
              </span>
              <Button
                size="sm"
                variant="ghost"
                className="h-6 shrink-0 text-xs"
                disabled={setEntries.isPending}
                onClick={() =>
                  setEntries.mutate({
                    appType,
                    alias: suggestion.alias,
                    entries: suggestion.entries,
                  })
                }
              >
                {t("modelAliases.adopt", { defaultValue: "采纳" })}
              </Button>
            </div>
          ))}
        </div>
      )}

      {/* 新建逻辑模型 */}
      <div className="flex items-center gap-2">
        <Input
          placeholder={t("modelAliases.aliasPlaceholder", {
            defaultValue: "逻辑模型名，如 sonnet",
          })}
          value={createAlias}
          onChange={(event) => setCreateAlias(event.target.value)}
          className="h-8"
          disabled={!editable}
          onKeyDown={(event) => {
            if (event.key === "Enter") beginCreate();
          }}
        />
        <Button
          size="sm"
          disabled={!createAlias.trim() || !editable}
          onClick={beginCreate}
        >
          <Plus className="mr-1 h-3.5 w-3.5" />
          {t("common.add", { defaultValue: "添加" })}
        </Button>
      </div>

      {groups.length === 0 && !draft ? (
        <p className="py-4 text-center text-sm text-muted-foreground">
          {t("modelAliases.empty", {
            defaultValue:
              "暂无配置。添加一个逻辑模型，并为每个渠道填入它在该渠道的真实模型名",
          })}
        </p>
      ) : (
        <div className="space-y-2">
          {groups.map((group) => {
            const isEditing = draft?.alias === group.alias;
            return (
              <div
                key={group.alias}
                className="rounded-lg border border-border/60 bg-card/40 p-3"
              >
                <div className="flex items-center gap-2">
                  <span className="flex-1 truncate font-mono text-sm font-medium">
                    {group.alias}
                  </span>
                  <span className="shrink-0 text-[11px] text-muted-foreground">
                    {t("modelAliases.channelCount", {
                      count: group.entries.length,
                      defaultValue: `${group.entries.length} 个渠道`,
                    })}
                  </span>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="h-7 w-7"
                    title={t("common.edit", { defaultValue: "编辑" })}
                    disabled={!editable}
                    onClick={() => beginEdit(group.alias, group.entries)}
                  >
                    <Pencil className="h-3.5 w-3.5" />
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="h-7 w-7"
                    title={t("modelAliases.clearAlias", {
                      defaultValue: "删除该逻辑模型",
                    })}
                    disabled={!editable}
                    onClick={() =>
                      setEntries.mutate({
                        appType,
                        alias: group.alias,
                        entries: [],
                      })
                    }
                  >
                    <Trash2 className="h-3.5 w-3.5 text-destructive" />
                  </Button>
                </div>

                <div className="mt-2 space-y-1">
                  {group.entries.map((entry) => (
                    <div
                      key={`${group.alias}:${entry.providerId}`}
                      className="flex items-center gap-2 rounded border border-border/40 bg-background/50 px-2 py-1 text-xs"
                    >
                      <span className="w-32 shrink-0 truncate text-muted-foreground">
                        {providerName(entry.providerId)}
                      </span>
                      <span className="text-muted-foreground">→</span>
                      <span className="flex-1 truncate font-mono">
                        {entry.targetModel}
                      </span>
                    </div>
                  ))}
                </div>

                {isEditing && draft && (
                  <div className="mt-2 space-y-2 rounded-md border border-primary/30 bg-primary/5 p-2">
                    {draft.entries.map((entry, index) => (
                      <div key={index} className="flex items-center gap-2">
                        <select
                          className="h-7 w-32 shrink-0 rounded border border-border bg-background px-1 text-xs"
                          value={entry.providerId}
                          onChange={(event) => {
                            const next = [...draft.entries];
                            next[index] = {
                              ...next[index],
                              providerId: event.target.value,
                            };
                            setDraft({ ...draft, entries: next });
                          }}
                        >
                          <option value="">
                            {t("modelAliases.pickChannel", {
                              defaultValue: "选择渠道",
                            })}
                          </option>
                          {providers.map((provider) => {
                            const id = String(
                              (provider as unknown as { id: string }).id,
                            );
                            return (
                              <option key={id} value={id}>
                                {String(
                                  (provider as unknown as { name: string })
                                    .name,
                                )}
                              </option>
                            );
                          })}
                        </select>
                        <Input
                          className="h-7 flex-1 font-mono text-xs"
                          placeholder={t("modelAliases.targetPlaceholder", {
                            defaultValue: "该渠道的真实模型名",
                          })}
                          value={entry.targetModel}
                          onChange={(event) => {
                            const next = [...draft.entries];
                            next[index] = {
                              ...next[index],
                              targetModel: event.target.value,
                            };
                            setDraft({ ...draft, entries: next });
                          }}
                        />
                        <Button
                          size="icon"
                          variant="ghost"
                          className="h-7 w-7"
                          title={t("modelAliases.removeChannel", {
                            defaultValue: "移除该渠道",
                          })}
                          onClick={() =>
                            setDraft({
                              ...draft,
                              entries: draft.entries.filter(
                                (_, entryIndex) => entryIndex !== index,
                              ),
                            })
                          }
                        >
                          <X className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    ))}
                    <div className="flex items-center justify-between">
                      <Button
                        size="sm"
                        variant="ghost"
                        className="h-7 text-xs"
                        onClick={() =>
                          setDraft({
                            ...draft,
                            entries: [
                              ...draft.entries,
                              { providerId: "", targetModel: "" },
                            ],
                          })
                        }
                      >
                        <Plus className="mr-1 h-3 w-3" />
                        {t("modelAliases.addChannel", {
                          defaultValue: "添加渠道",
                        })}
                      </Button>
                      <div className="flex gap-2">
                        <Button
                          size="sm"
                          variant="ghost"
                          className="h-7 text-xs"
                          onClick={() => setDraft(null)}
                        >
                          {t("common.cancel", { defaultValue: "取消" })}
                        </Button>
                        <Button
                          size="sm"
                          className="h-7 text-xs"
                          disabled={setEntries.isPending}
                          onClick={saveDraft}
                        >
                          <Check className="mr-1 h-3 w-3" />
                          {t("common.save", { defaultValue: "保存" })}
                        </Button>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* 新建时的编辑区（还没有对应 group） */}
      {draft && !groups.some((group) => group.alias === draft.alias) && (
        <div className="rounded-lg border border-primary/30 bg-primary/5 p-3">
          <div className="flex items-center gap-2">
            <span className="flex-1 truncate font-mono text-sm font-medium">
              {draft.alias}
            </span>
            <Button
              size="icon"
              variant="ghost"
              className="h-7 w-7"
              onClick={() => setDraft(null)}
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          </div>
          <div className="mt-2 space-y-2">
            {draft.entries.map((entry, index) => (
              <div key={index} className="flex items-center gap-2">
                <select
                  className="h-7 w-32 shrink-0 rounded border border-border bg-background px-1 text-xs"
                  value={entry.providerId}
                  onChange={(event) => {
                    const next = [...draft.entries];
                    next[index] = {
                      ...next[index],
                      providerId: event.target.value,
                    };
                    setDraft({ ...draft, entries: next });
                  }}
                >
                  <option value="">
                    {t("modelAliases.pickChannel", {
                      defaultValue: "选择渠道",
                    })}
                  </option>
                  {providers.map((provider) => {
                    const id = String(
                      (provider as unknown as { id: string }).id,
                    );
                    return (
                      <option key={id} value={id}>
                        {String((provider as unknown as { name: string }).name)}
                      </option>
                    );
                  })}
                </select>
                <Input
                  className="h-7 flex-1 font-mono text-xs"
                  placeholder={t("modelAliases.targetPlaceholder", {
                    defaultValue: "该渠道的真实模型名",
                  })}
                  value={entry.targetModel}
                  onChange={(event) => {
                    const next = [...draft.entries];
                    next[index] = {
                      ...next[index],
                      targetModel: event.target.value,
                    };
                    setDraft({ ...draft, entries: next });
                  }}
                />
              </div>
            ))}
            <div className="flex justify-end gap-2">
              <Button
                size="sm"
                variant="ghost"
                className="h-7 text-xs"
                onClick={() =>
                  setDraft({
                    ...draft,
                    entries: [
                      ...draft.entries,
                      { providerId: "", targetModel: "" },
                    ],
                  })
                }
              >
                <Plus className="mr-1 h-3 w-3" />
                {t("modelAliases.addChannel", { defaultValue: "添加渠道" })}
              </Button>
              <Button
                size="sm"
                className="h-7 text-xs"
                disabled={setEntries.isPending}
                onClick={saveDraft}
              >
                <Check className="mr-1 h-3 w-3" />
                {t("common.save", { defaultValue: "保存" })}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
