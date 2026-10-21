/**
 * Routing Groups 面板（统一网关 smart 分组）
 *
 * 每 app 多分组，成员归属制（同 app 下 provider 仅属一组，避免重复计费与健康漂移）。
 * 支持：创建/重命名/启用/删除 + 成员多选保存 + 分组拖拽 + 组内成员拖拽
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Check,
  Pencil,
  Plus,
  Trash2,
  X,
  GripVertical,
  Copy,
  ChevronDown,
  Braces,
} from "lucide-react";
import {
  DndContext,
  closestCenter,
  PointerSensor,
  KeyboardSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
  sortableKeyboardCoordinates,
  arrayMove,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { useQueries } from "@tanstack/react-query";
import { failoverApi } from "@/lib/api/failover";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { toast } from "sonner";
import { useProvidersQuery } from "@/lib/query";
import { useRateLimitCooldowns } from "@/lib/query/failover";
import { useProxyStatusQuery } from "@/lib/query/proxy";
import {
  useCreateRoutingGroup,
  useDeleteRoutingGroup,
  useRoutingGroups,
  useSetRoutingGroupMembers,
  useReorderRoutingGroups,
  useReorderRoutingGroupMembers,
  useUpdateRoutingGroup,
} from "@/lib/query/routing-groups";
import type { ProxyAppId } from "@/config/appConfig";

export function RoutingGroupsPanel({
  appType,
  onEffectiveRouteSizeChange,
}: {
  appType: ProxyAppId;
  onEffectiveRouteSizeChange?: (size: number) => void;
}) {
  const { t } = useTranslation();
  const { data: groups = [], isLoading } = useRoutingGroups(appType);
  const { data: providersData } = useProvidersQuery(appType);
  const providers = useMemo(
    () => Object.values(providersData?.providers ?? {}),
    [providersData],
  );
  const { data: cooldowns = [] } = useRateLimitCooldowns(appType);
  const cooldownMap = useMemo(
    () => new Map(cooldowns.map((c) => [c.providerId, c.remainingSecs])),
    [cooldowns],
  );
  const { data: proxyStatus } = useProxyStatusQuery();
  const waitingProviderId =
    proxyStatus?.rate_limit_waiting?.appType === appType
      ? (proxyStatus.rate_limit_waiting.providerId as string)
      : null;
  const effectiveRoute = useMemo(() => {
    const enabledGroups = groups.filter((g) => g.group.enabled);
    const providerIds = enabledGroups.flatMap((g) =>
      g.members.map((m) => m.providerId),
    );
    return {
      providerIds,
      enabledGroups,
    };
  }, [groups]);
  const lastReportedSizeRef = useRef<number | null>(null);
  useEffect(() => {
    if (!onEffectiveRouteSizeChange) return;
    const size = effectiveRoute.providerIds.length;
    // 初次 groups=[] 的 0 不上报，避免与 ResilienceSummaryBar 的兜底查询竞态
    if (!isLoading && lastReportedSizeRef.current !== size) {
      lastReportedSizeRef.current = size;
      onEffectiveRouteSizeChange(size);
    }
  }, [
    effectiveRoute.providerIds.length,
    isLoading,
    onEffectiveRouteSizeChange,
  ]);
  const effectiveFirstProviderId =
    effectiveRoute.enabledGroups[0]?.members[0]?.providerId ?? null;
  const effectiveFirstGroupId =
    effectiveRoute.enabledGroups[0]?.group.id ?? null;
  const healthQueries = useQueries({
    queries: effectiveRoute.providerIds.map((providerId) => ({
      queryKey: ["providerHealth", providerId, appType] as const,
      queryFn: () => failoverApi.getProviderHealth(providerId, appType),
      enabled: !!providerId && !!appType,
      refetchInterval: 5000,
      retry: false as const,
    })),
  });
  const providerHealthMap = useMemo(() => {
    const map = new Map<string, (typeof healthQueries)[number]["data"]>();
    effectiveRoute.providerIds.forEach((providerId, index) => {
      const entry = healthQueries[index]?.data;
      if (entry) map.set(providerId, entry);
    });
    return map;
  }, [effectiveRoute.providerIds, healthQueries]);

  const create = useCreateRoutingGroup();
  const update = useUpdateRoutingGroup();
  const remove = useDeleteRoutingGroup();
  const reorderGroups = useReorderRoutingGroups();
  const reorderMembers = useReorderRoutingGroupMembers();
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );
  const setMembers = useSetRoutingGroupMembers();

  const [createName, setCreateName] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState("");
  const [confirmId, setConfirmId] = useState<string | null>(null);
  const [memberPickOpen, setMemberPickOpen] = useState<string | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  const openPick = (groupId: string, current: string[]) => {
    setPicked(new Set(current));
    setMemberPickOpen(groupId);
  };
  const toggleCollapsed = (groupId: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(groupId)) next.delete(groupId);
      else next.add(groupId);
      return next;
    });
  };
  const handleCreateGroup = () => {
    const trimmedName = createName.trim();
    if (!trimmedName) {
      toast.error(
        t("routingGroups.renameEmpty", { defaultValue: "分组名称不能为空" }),
      );
      return;
    }
    const duplicated = groups.some(
      (entry) =>
        entry.group.name.trim().toLowerCase() === trimmedName.toLowerCase(),
    );
    if (duplicated) {
      toast.error(
        t("routingGroups.renameDuplicate", {
          defaultValue: "分组名称已存在",
          name: trimmedName,
        }),
      );
      return;
    }
    create.mutate(
      { appType, name: trimmedName },
      { onSuccess: () => setCreateName("") },
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
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">
          {t("routingGroups.title", { defaultValue: "路由分组" })}
        </h3>
        <span className="text-xs text-muted-foreground">
          {t("routingGroups.hint", {
            defaultValue: "同 app 下供应商归属制，避免重复计费",
          })}
        </span>
      </div>

      {/* 生效路由预览：已启用分组聚合的真实 P1→Pn 顺序，等价于 ProviderRouter 的数据面拼接 */}
      <div className="rounded-lg border border-primary/20 bg-primary/5 px-3 py-2.5">
        <div className="flex items-center justify-between gap-2">
          <span className="text-xs font-medium text-primary">
            {t("routingGroups.effectiveTitle", {
              defaultValue: "生效路由",
            })}
          </span>
          <span className="text-[11px] text-muted-foreground">
            {t("routingGroups.effectiveHint", {
              defaultValue: "已启用分组按顺序拼接，P1 优先",
            })}
          </span>
        </div>
        {effectiveRoute.providerIds.length === 0 ? (
          <p className="mt-1.5 text-xs text-muted-foreground">
            {effectiveRoute.enabledGroups.length === 0
              ? t("routingGroups.effectiveEmptyNoGroup", {
                  defaultValue: "暂无已启用分组，将回退到旧故障转移队列",
                })
              : t("routingGroups.effectiveEmptyNoMembers", {
                  defaultValue: "已启用分组均无成员，请先添加成员",
                })}
          </p>
        ) : (
          <div className="mt-2 space-y-1.5">
            {(() => {
              let globalIdx = 0;
              return effectiveRoute.enabledGroups.map((entry) => (
                <div
                  key={entry.group.id}
                  className="flex flex-wrap items-center gap-1.5"
                >
                  <span className="shrink-0 rounded bg-primary/10 px-1.5 py-0.5 text-[11px] font-medium text-primary">
                    {entry.group.name}
                  </span>
                  {entry.members.map((m) => {
                    globalIdx += 1;
                    const idx = globalIdx;
                    const name =
                      providers.find(
                        (p) =>
                          String((p as unknown as { id: string }).id) ===
                          m.providerId,
                      )?.name ?? m.providerId;
                    const isP1 = idx === 1;
                    const cooldownSecs = cooldownMap.get(m.providerId) ?? 0;
                    const isWaiting = waitingProviderId === m.providerId;
                    const health = providerHealthMap.get(m.providerId);
                    const isUnhealthy =
                      health != null && health.is_healthy === false;
                    const circuitState = health?.circuit_state ?? null;
                    return (
                      <span
                        key={`${m.providerId}-${idx}`}
                        className="inline-flex items-center gap-1"
                      >
                        <span
                          className={
                            isP1
                              ? "inline-flex items-center gap-1 rounded-full bg-primary px-2 py-0.5 text-xs font-semibold text-primary-foreground shadow-sm"
                              : "inline-flex items-center gap-1 rounded-full bg-background px-2 py-0.5 text-xs font-medium"
                          }
                          title={
                            isWaiting
                              ? t("routingGroups.healthWaiting", {
                                  defaultValue:
                                    "该供应商正处在 429 等待重试循环中",
                                })
                              : cooldownSecs > 0
                                ? t("routingGroups.healthCooldown", {
                                    seconds: cooldownSecs,
                                    defaultValue: `限流冷却中，剩余 ${cooldownSecs}s`,
                                  })
                                : isUnhealthy
                                  ? (health?.last_error?.trim() ??
                                    t("routingGroups.healthCircuitOpen", {
                                      state: circuitState ?? "open",
                                      defaultValue: `熔断中 (${circuitState ?? "open"})`,
                                    }))
                                  : undefined
                          }
                        >
                          <span
                            className={
                              isP1
                                ? "inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-primary-foreground/20 px-1 text-[10px] font-bold"
                                : "inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-primary/15 px-1 text-[10px] font-bold text-primary"
                            }
                          >
                            P{idx}
                          </span>
                          <span className="max-w-[14ch] truncate">{name}</span>
                          {isWaiting ? (
                            <span className="inline-flex h-3 w-3 items-center justify-center rounded-full bg-teal-500/20">
                              <span className="h-2 w-2 animate-pulse rounded-full bg-teal-500" />
                            </span>
                          ) : cooldownSecs > 0 ? (
                            <span className="rounded bg-amber-500/15 px-1 py-0.5 text-[10px] font-medium tabular-nums leading-none text-amber-600 dark:text-amber-400">
                              {cooldownSecs}s
                            </span>
                          ) : isUnhealthy ? (
                            <span
                              className={`h-2 w-2 shrink-0 rounded-full ${circuitState === "half_open" ? "bg-amber-500 animate-pulse" : "bg-red-500"}`}
                            />
                          ) : null}
                        </span>
                      </span>
                    );
                  })}
                </div>
              ));
            })()}
            <div className="flex justify-end gap-1">
              <Button
                size="icon"
                variant="ghost"
                className="h-6 w-6"
                title={t("routingGroups.copyText", {
                  defaultValue: "复制文本",
                })}
                onClick={async () => {
                  const text = effectiveRoute.providerIds.join(", ");
                  try {
                    await navigator.clipboard.writeText(text);
                    toast.success(
                      t("routingGroups.effectiveCopied", {
                        defaultValue: "已复制生效路由",
                      }),
                    );
                  } catch {
                    toast.error(
                      t("common.copyFailed", { defaultValue: "复制失败" }),
                    );
                  }
                }}
              >
                <Copy className="h-3.5 w-3.5" />
              </Button>
              <Button
                size="icon"
                variant="ghost"
                className="h-6 w-6"
                title={t("routingGroups.copyJson", {
                  defaultValue: "复制 JSON",
                })}
                onClick={async () => {
                  const text = JSON.stringify(
                    effectiveRoute.providerIds,
                    null,
                    2,
                  );
                  try {
                    await navigator.clipboard.writeText(text);
                    toast.success(
                      t("routingGroups.effectiveCopiedJson", {
                        defaultValue: "已复制 JSON 数组",
                      }),
                    );
                  } catch {
                    toast.error(
                      t("common.copyFailed", { defaultValue: "复制失败" }),
                    );
                  }
                }}
              >
                <Braces className="h-3.5 w-3.5" />
              </Button>
            </div>
          </div>
        )}
      </div>

      <div className="flex items-center gap-2">
        <Input
          placeholder={t("routingGroups.namePlaceholder", {
            defaultValue: "分组名称，如 default / fast",
          })}
          value={createName}
          onChange={(e) => setCreateName(e.target.value)}
          className="h-8"
          onKeyDown={(e) => {
            if (e.key === "Enter") handleCreateGroup();
          }}
        />
        <Button
          size="sm"
          disabled={!createName.trim() || create.isPending}
          onClick={handleCreateGroup}
        >
          <Plus className="mr-1 h-3.5 w-3.5" />
          {t("common.add", { defaultValue: "添加" })}
        </Button>
      </div>

      {groups.length === 0 ? (
        <p className="py-4 text-center text-sm text-muted-foreground">
          {t("routingGroups.empty", {
            defaultValue: "暂无分组，创建一个 default 分组开始",
          })}
        </p>
      ) : (
        <div className="space-y-2">
          <DndContext
            sensors={sensors}
            collisionDetection={closestCenter}
            accessibility={{
              announcements: {
                onDragStart: ({ active }) =>
                  t("routingGroups.dragStartHint", {
                    id: String(active.id),
                    defaultValue: `开始拖拽 ${String(active.id)}`,
                  }),
                onDragOver: () => "",
                onDragCancel: () =>
                  t("routingGroups.dragCancelHint", {
                    defaultValue: "拖拽已取消",
                  }),
                onDragEnd: ({ active, over }) =>
                  over
                    ? t("routingGroups.dragEndHint", {
                        activeId: String(active.id),
                        overId: String(over.id),
                        defaultValue: `已将 ${String(active.id)} 移动到 ${String(over.id)} 位置`,
                      })
                    : t("routingGroups.dragCancelHint", {
                        defaultValue: "拖拽已取消",
                      }),
              },
            }}
            onDragEnd={(event) => {
              const { active, over } = event;
              if (!over || active.id === over.id) return;
              const aId = String(active.id);
              const oId = String(over.id);
              if (aId.startsWith("group:") && oId.startsWith("group:")) {
                const ids = groups.map((g) => g.group.id);
                const oldIdx = ids.indexOf(aId.slice(6));
                const newIdx = ids.indexOf(oId.slice(6));
                if (oldIdx === -1 || newIdx === -1) return;
                const reordered = arrayMove(ids, oldIdx, newIdx);
                reorderGroups.mutate({ appType, orderedIds: reordered });
                return;
              }
              if (aId.startsWith("member:") && oId.startsWith("member:")) {
                const aParts = aId.split(":");
                const oParts = oId.split(":");
                const groupIdA = aParts[1];
                const groupIdB = oParts[1];
                if (groupIdA !== groupIdB) return;
                const entry = groups.find((g) => g.group.id === groupIdA);
                if (!entry) return;
                const ids = entry.members.map((m) => m.providerId);
                const oldIdx = ids.indexOf(aParts[2]);
                const newIdx = ids.indexOf(oParts[2]);
                if (oldIdx === -1 || newIdx === -1) return;
                const reordered = arrayMove(ids, oldIdx, newIdx);
                reorderMembers.mutate({
                  appType,
                  groupId: groupIdA,
                  orderedProviderIds: reordered,
                });
              }
            }}
          >
            <SortableContext
              items={groups.map((g) => `group:${g.group.id}`)}
              strategy={verticalListSortingStrategy}
            >
              {groups.map(({ group, members }) => {
                const isP1Group = group.id === effectiveFirstGroupId;
                const isCollapsed = collapsed.has(group.id);
                const memberCountLabel = t("routingGroups.memberCount", {
                  count: members.length,
                  defaultValue: `${members.length} 个成员`,
                });
                return (
                  <SortableGroupRow
                    key={group.id}
                    groupId={group.id}
                    appType={appType}
                    group={group}
                    allGroups={groups.map((entry) => entry.group)}
                    editingId={editingId}
                    editingName={editingName}
                    setEditingId={setEditingId}
                    setEditingName={setEditingName}
                    update={update}
                    setConfirmId={setConfirmId}
                    isP1Group={isP1Group}
                    isCollapsed={isCollapsed}
                    onToggleCollapsed={() => toggleCollapsed(group.id)}
                    memberCountLabel={memberCountLabel}
                  >
                    <SortableContext
                      items={members.map(
                        (m) => `member:${group.id}:${m.providerId}`,
                      )}
                      strategy={verticalListSortingStrategy}
                    >
                      <div className="mt-2 flex flex-wrap gap-1.5">
                        {members.length === 0 ? (
                          <div className="flex w-full items-center justify-between gap-2 rounded-md border border-dashed px-2 py-1.5">
                            <span className="text-xs text-muted-foreground">
                              {t("routingGroups.noMembers", {
                                defaultValue: "暂无成员",
                              })}
                            </span>
                            <Button
                              size="sm"
                              variant="ghost"
                              className="h-6 text-xs"
                              onClick={() => openPick(group.id, [])}
                            >
                              <Plus className="mr-1 h-3 w-3" />
                              {t("routingGroups.addMembers", {
                                defaultValue: "添加成员",
                              })}
                            </Button>
                          </div>
                        ) : (
                          members.map((m) => (
                            <SortableMemberBadge
                              key={m.providerId}
                              id={`member:${group.id}:${m.providerId}`}
                              label={
                                providers.find(
                                  (p) =>
                                    String(
                                      (p as unknown as { id: string }).id,
                                    ) === m.providerId,
                                )?.name ?? m.providerId
                              }
                              isP1={effectiveFirstProviderId === m.providerId}
                              cooldownSecs={cooldownMap.get(m.providerId) ?? 0}
                              isWaiting={waitingProviderId === m.providerId}
                              health={
                                providerHealthMap.get(m.providerId) ?? null
                              }
                            />
                          ))
                        )}
                      </div>
                    </SortableContext>
                    <div className="mt-2 flex gap-2">
                      <Button
                        size="sm"
                        variant="outline"
                        className="h-7 text-xs"
                        onClick={() =>
                          openPick(
                            group.id,
                            members.map((m) => m.providerId),
                          )
                        }
                      >
                        {t("routingGroups.editMembers", {
                          defaultValue: "编辑成员",
                        })}
                      </Button>
                    </div>
                  </SortableGroupRow>
                );
              })}
            </SortableContext>
          </DndContext>
        </div>
      )}

      <Dialog
        open={!!memberPickOpen}
        onOpenChange={(open) => !open && setMemberPickOpen(null)}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>
              {t("routingGroups.pickMembers", { defaultValue: "选择成员" })}
            </DialogTitle>
          </DialogHeader>
          <div className="max-h-[50vh] space-y-1 overflow-y-auto py-2">
            {providers.length === 0 ? (
              <p className="py-4 text-center text-sm text-muted-foreground">
                {t("routingGroups.noProviders", {
                  defaultValue: "暂无可用供应商",
                })}
              </p>
            ) : (
              providers.map((p) => {
                const id = String((p as unknown as { id: string }).id);
                const checked = picked.has(id);
                return (
                  <label
                    key={id}
                    className="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-muted/50"
                  >
                    <input
                      type="checkbox"
                      checked={checked}
                      onChange={(e) => {
                        const next = new Set(picked);
                        if (e.target.checked) next.add(id);
                        else next.delete(id);
                        setPicked(next);
                      }}
                    />
                    <span className="flex-1 truncate text-sm">
                      {String((p as unknown as { name: string }).name)}
                    </span>
                  </label>
                );
              })
            )}
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={() => setMemberPickOpen(null)}>
              {t("common.cancel")}
            </Button>
            <Button
              onClick={() => {
                if (!memberPickOpen) return;
                setMembers.mutate(
                  {
                    groupId: memberPickOpen,
                    appType,
                    providerIds: [...picked],
                  },
                  { onSuccess: () => setMemberPickOpen(null) },
                );
              }}
              disabled={setMembers.isPending}
            >
              {t("common.save")}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {confirmId && (
        <ConfirmDialog
          isOpen
          title={t("common.confirm", { defaultValue: "确认" })}
          message={t("routingGroups.deleteConfirm", {
            defaultValue: "确定删除该分组吗？成员归属将一并解除。",
          })}
          variant="destructive"
          onConfirm={() => {
            remove.mutate(
              { id: confirmId, appType },
              { onSuccess: () => setConfirmId(null) },
            );
          }}
          onCancel={() => setConfirmId(null)}
        />
      )}
    </div>
  );
}

function SortableGroupRow({
  groupId,
  group,
  appType,
  allGroups,
  editingId,
  editingName,
  setEditingId,
  setEditingName,
  update,
  setConfirmId,
  isP1Group,
  isCollapsed,
  onToggleCollapsed,
  memberCountLabel,
  children,
}: {
  groupId: string;
  group: { id: string; name: string; enabled: boolean };
  appType: string;
  allGroups: Array<{ id: string; name: string }>;
  editingId: string | null;
  editingName: string;
  setEditingId: (v: string | null) => void;
  setEditingName: (v: string) => void;
  update: ReturnType<typeof useUpdateRoutingGroup>;
  setConfirmId: (v: string | null) => void;
  isP1Group: boolean;
  isCollapsed: boolean;
  onToggleCollapsed: () => void;
  memberCountLabel: string;
  children: React.ReactNode;
}) {
  const { t } = useTranslation();
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id: `group:${groupId}` });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  } as React.CSSProperties;
  const isEditing = editingId === groupId;
  const handleSubmitRename = () => {
    const trimmedName = editingName.trim();
    if (!trimmedName) {
      toast.error(
        t("routingGroups.renameEmpty", { defaultValue: "分组名称不能为空" }),
      );
      return;
    }
    const duplicated = allGroups.some(
      (entry) =>
        entry.id !== groupId &&
        entry.name.trim().toLowerCase() === trimmedName.toLowerCase(),
    );
    if (duplicated) {
      toast.error(
        t("routingGroups.renameDuplicate", {
          defaultValue: "分组名称已存在",
          name: trimmedName,
        }),
      );
      return;
    }
    update.mutate(
      { id: groupId, appType, name: trimmedName },
      { onSuccess: () => setEditingId(null) },
    );
  };
  return (
    <div
      ref={setNodeRef}
      style={style}
      className={
        isP1Group
          ? "rounded-lg border border-primary/30 bg-primary/5 p-3 shadow-sm"
          : "rounded-lg border border-border/60 bg-card/40 p-3"
      }
    >
      <div className="flex items-center gap-2">
        <button
          type="button"
          className="cursor-grab touch-none p-0.5 text-muted-foreground hover:text-foreground"
          aria-label={t("routingGroups.dragGroupHandle", {
            defaultValue: "拖拽排序分组",
          })}
          title={t("routingGroups.dragGroupHint", {
            defaultValue: "拖拽或用键盘移动以调整分组顺序",
          })}
          {...attributes}
          {...listeners}
        >
          <GripVertical className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          onClick={onToggleCollapsed}
          className="inline-flex items-center gap-1 rounded px-1 py-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
          title={
            isCollapsed
              ? t("routingGroups.expand", { defaultValue: "展开" })
              : t("routingGroups.collapse", { defaultValue: "收起" })
          }
          aria-label={isCollapsed ? "expand" : "collapse"}
        >
          <ChevronDown
            className={`h-3.5 w-3.5 transition-transform ${isCollapsed ? "-rotate-90" : "rotate-0"}`}
          />
        </button>
        {isEditing ? (
          <>
            <Input
              value={editingName}
              onChange={(e) => setEditingName(e.target.value)}
              className="h-7 flex-1"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter") handleSubmitRename();
                if (e.key === "Escape") setEditingId(null);
              }}
            />
            <Button
              size="icon"
              variant="ghost"
              className="h-7 w-7"
              onClick={handleSubmitRename}
              disabled={!editingName.trim() || update.isPending}
            >
              <Check className="h-3.5 w-3.5" />
            </Button>
            <Button
              size="icon"
              variant="ghost"
              className="h-7 w-7"
              onClick={() => setEditingId(null)}
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          </>
        ) : (
          <>
            <span className="flex flex-1 items-center gap-1.5 truncate text-sm font-medium">
              <span className="truncate">{group.name}</span>
              {isP1Group && (
                <span className="shrink-0 rounded bg-primary px-1 py-0.5 text-[10px] font-bold leading-none text-primary-foreground">
                  P1
                </span>
              )}
            </span>
            <span className="shrink-0 text-[11px] text-muted-foreground">
              {memberCountLabel}
            </span>
            <Badge
              variant={group.enabled ? "secondary" : "outline"}
              className="text-[10px] font-normal"
            >
              {group.enabled ? t("common.enabled") : t("common.notSet")}
            </Badge>
            <Switch
              checked={group.enabled}
              onCheckedChange={(checked) =>
                update.mutate({
                  id: groupId,
                  appType,
                  enabled: checked,
                })
              }
            />
            <Button
              size="icon"
              variant="ghost"
              className="h-7 w-7"
              title={t("common.edit")}
              onClick={() => {
                setEditingId(groupId);
                setEditingName(group.name);
              }}
            >
              <Pencil className="h-3.5 w-3.5" />
            </Button>
            <Button
              size="icon"
              variant="ghost"
              className="h-7 w-7"
              title={t("common.delete")}
              onClick={() => setConfirmId(groupId)}
            >
              <Trash2 className="h-3.5 w-3.5 text-destructive" />
            </Button>
          </>
        )}
      </div>
      {!isCollapsed && children}
    </div>
  );
}

function SortableMemberBadge({
  id,
  label,
  isP1,
  cooldownSecs,
  isWaiting,
  health,
}: {
  id: string;
  label: string;
  isP1: boolean;
  cooldownSecs: number;
  isWaiting: boolean;
  health: {
    is_healthy: boolean;
    circuit_state?: string | null;
    last_error?: string | null;
  } | null;
}) {
  const { t } = useTranslation();
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  } as React.CSSProperties;
  const isUnhealthy = health != null && health.is_healthy === false;
  const badgeTitle = isWaiting
    ? t("routingGroups.healthWaiting", {
        defaultValue: "该供应商正处在 429 等待重试循环中",
      })
    : cooldownSecs > 0
      ? t("routingGroups.healthCooldown", {
          seconds: cooldownSecs,
          defaultValue: `限流冷却中，剩余 ${cooldownSecs}s`,
        })
      : isUnhealthy
        ? (health?.last_error?.trim() ??
          t("routingGroups.healthCircuitOpen", {
            state: health?.circuit_state ?? "open",
            defaultValue: `熔断中 (${health?.circuit_state ?? "open"})`,
          }))
        : t("routingGroups.dragMemberHint", {
            defaultValue: "拖拽或用键盘移动以调整成员顺序",
          });
  return (
    <span
      ref={setNodeRef}
      style={style}
      className={
        isP1
          ? "inline-flex items-center gap-1 rounded-full border border-primary/30 bg-primary px-2 py-0.5 text-[11px] font-semibold text-primary-foreground shadow-sm"
          : "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-mono"
      }
      title={badgeTitle}
      {...attributes}
      {...listeners}
    >
      <GripVertical
        className={
          isP1
            ? "h-3 w-3 shrink-0 text-primary-foreground/70"
            : "h-3 w-3 shrink-0 text-muted-foreground"
        }
      />
      {label}
      {isWaiting ? (
        <span className="inline-flex h-2.5 w-2.5 items-center justify-center">
          <span className="h-2 w-2 animate-pulse rounded-full bg-teal-500" />
        </span>
      ) : cooldownSecs > 0 ? (
        <span className="rounded bg-amber-500/20 px-1 py-0.5 text-[10px] leading-none text-amber-700 dark:text-amber-300">
          {cooldownSecs}s
        </span>
      ) : isUnhealthy ? (
        <span
          className={`h-2 w-2 shrink-0 rounded-full ${health?.circuit_state === "half_open" ? "bg-amber-500 animate-pulse" : "bg-red-500"}`}
        />
      ) : null}
    </span>
  );
}
