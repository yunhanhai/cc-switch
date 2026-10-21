/**
 * Routing Groups 面板（统一网关 smart 分组）
 *
 * 每 app 多分组，成员归属制（同 app 下 provider 仅属一组，避免重复计费与健康漂移）。
 * 支持：创建/重命名/启用/删除 + 成员多选保存 + 分组拖拽 + 组内成员拖拽
 */
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Check,
  Pencil,
  Plus,
  Trash2,
  X,
  GripVertical,
  Copy,
} from "lucide-react";
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
  arrayMove,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
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

export function RoutingGroupsPanel({ appType }: { appType: ProxyAppId }) {
  const { t } = useTranslation();
  const { data: groups = [], isLoading } = useRoutingGroups(appType);
  const { data: providersData } = useProvidersQuery(appType);
  const providers = useMemo(
    () => Object.values(providersData?.providers ?? {}),
    [providersData],
  );
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

  const create = useCreateRoutingGroup();
  const update = useUpdateRoutingGroup();
  const remove = useDeleteRoutingGroup();
  const reorderGroups = useReorderRoutingGroups();
  const reorderMembers = useReorderRoutingGroupMembers();
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
  );
  const setMembers = useSetRoutingGroupMembers();

  const [createName, setCreateName] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState("");
  const [confirmId, setConfirmId] = useState<string | null>(null);
  const [memberPickOpen, setMemberPickOpen] = useState<string | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());

  const openPick = (groupId: string, current: string[]) => {
    setPicked(new Set(current));
    setMemberPickOpen(groupId);
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
          <div className="mt-2 flex flex-wrap items-center gap-1.5">
            {effectiveRoute.providerIds.map((providerId, idx) => {
              const name =
                providers.find(
                  (p) =>
                    String((p as unknown as { id: string }).id) === providerId,
                )?.name ?? providerId;
              return (
                <span
                  key={`${providerId}-${idx}`}
                  className="inline-flex items-center gap-1"
                >
                  {idx > 0 && (
                    <span className="text-[11px] text-muted-foreground">→</span>
                  )}
                  <span className="inline-flex items-center gap-1 rounded-full bg-background px-2 py-0.5 text-xs font-medium">
                    <span className="inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-primary/15 px-1 text-[10px] font-bold text-primary">
                      P{idx + 1}
                    </span>
                    <span className="max-w-[14ch] truncate">{name}</span>
                  </span>
                </span>
              );
            })}
            <Button
              size="icon"
              variant="ghost"
              className="ml-auto h-6 w-6"
              title={t("common.copy", { defaultValue: "复制" })}
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
            if (e.key === "Enter" && createName.trim()) {
              create.mutate(
                { appType, name: createName.trim() },
                { onSuccess: () => setCreateName("") },
              );
            }
          }}
        />
        <Button
          size="sm"
          disabled={!createName.trim() || create.isPending}
          onClick={() =>
            create.mutate(
              { appType, name: createName.trim() },
              { onSuccess: () => setCreateName("") },
            )
          }
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
              {groups.map(({ group, members }) => (
                <SortableGroupRow
                  key={group.id}
                  groupId={group.id}
                  appType={appType}
                  group={group}
                  editingId={editingId}
                  editingName={editingName}
                  setEditingId={setEditingId}
                  setEditingName={setEditingName}
                  update={update}
                  setConfirmId={setConfirmId}
                >
                  <SortableContext
                    items={members.map(
                      (m) => `member:${group.id}:${m.providerId}`,
                    )}
                    strategy={verticalListSortingStrategy}
                  >
                    <div className="mt-2 flex flex-wrap gap-1.5">
                      {members.length === 0 ? (
                        <span className="text-xs text-muted-foreground">
                          {t("routingGroups.noMembers", {
                            defaultValue: "暂无成员",
                          })}
                        </span>
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
              ))}
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
  editingId,
  editingName,
  setEditingId,
  setEditingName,
  update,
  setConfirmId,
  children,
}: {
  groupId: string;
  group: { id: string; name: string; enabled: boolean };
  appType: string;
  editingId: string | null;
  editingName: string;
  setEditingId: (v: string | null) => void;
  setEditingName: (v: string) => void;
  update: ReturnType<typeof useUpdateRoutingGroup>;
  setConfirmId: (v: string | null) => void;
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
  return (
    <div
      ref={setNodeRef}
      style={style}
      className="rounded-lg border border-border/60 bg-card/40 p-3"
    >
      <div className="flex items-center gap-2">
        <button
          type="button"
          className="cursor-grab touch-none p-0.5 text-muted-foreground hover:text-foreground"
          aria-label="drag handle"
          {...attributes}
          {...listeners}
        >
          <GripVertical className="h-3.5 w-3.5" />
        </button>
        {isEditing ? (
          <>
            <Input
              value={editingName}
              onChange={(e) => setEditingName(e.target.value)}
              className="h-7 flex-1"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  update.mutate(
                    { id: groupId, appType, name: editingName.trim() },
                    { onSuccess: () => setEditingId(null) },
                  );
                }
                if (e.key === "Escape") setEditingId(null);
              }}
            />
            <Button
              size="icon"
              variant="ghost"
              className="h-7 w-7"
              onClick={() =>
                update.mutate(
                  { id: groupId, appType, name: editingName.trim() },
                  { onSuccess: () => setEditingId(null) },
                )
              }
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
            <span className="flex-1 truncate text-sm font-medium">
              {group.name}
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
      {children}
    </div>
  );
}

function SortableMemberBadge({ id, label }: { id: string; label: string }) {
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  } as React.CSSProperties;
  return (
    <span
      ref={setNodeRef}
      style={style}
      className="inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-mono"
      {...attributes}
      {...listeners}
    >
      <GripVertical className="h-3 w-3 shrink-0 text-muted-foreground" />
      {label}
    </span>
  );
}
