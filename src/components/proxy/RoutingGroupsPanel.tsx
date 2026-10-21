/**
 * Routing Groups 面板（统一网关 smart 分组最小可用版）
 *
 * 每 app 多分组，成员归属制（同 app 下 provider 仅属一组，避免重复计费与健康漂移）。
 * 本轮仅做创建/重命名/启用/删除 + 成员按供应商列表多选保存；更细的拖拽与分组策略后续迭代。
 */
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Pencil, Plus, Trash2, X } from "lucide-react";
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
import { useProvidersQuery } from "@/lib/query";
import {
  useCreateRoutingGroup,
  useDeleteRoutingGroup,
  useRoutingGroups,
  useSetRoutingGroupMembers,
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

  const create = useCreateRoutingGroup();
  const update = useUpdateRoutingGroup();
  const remove = useDeleteRoutingGroup();
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
          {groups.map(({ group, members }) => (
            <div
              key={group.id}
              className="rounded-lg border border-border/60 bg-card/40 p-3"
            >
              <div className="flex items-center gap-2">
                {editingId === group.id ? (
                  <>
                    <Input
                      value={editingName}
                      onChange={(e) => setEditingName(e.target.value)}
                      className="h-7 flex-1"
                      autoFocus
                      onKeyDown={(e) => {
                        if (e.key === "Enter") {
                          update.mutate(
                            { id: group.id, appType, name: editingName.trim() },
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
                          { id: group.id, appType, name: editingName.trim() },
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
                          id: group.id,
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
                        setEditingId(group.id);
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
                      onClick={() => setConfirmId(group.id)}
                    >
                      <Trash2 className="h-3.5 w-3.5 text-destructive" />
                    </Button>
                  </>
                )}
              </div>
              <div className="mt-2 flex flex-wrap gap-1.5">
                {members.length === 0 ? (
                  <span className="text-xs text-muted-foreground">
                    {t("routingGroups.noMembers", { defaultValue: "暂无成员" })}
                  </span>
                ) : (
                  members.map((m) => (
                    <Badge
                      key={m.providerId}
                      variant="outline"
                      className="text-[11px] font-mono"
                    >
                      {providers.find(
                        (p) =>
                          String((p as unknown as { id: string }).id) ===
                          m.providerId,
                      )?.name ?? m.providerId}
                    </Badge>
                  ))
                )}
              </div>
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
                  {t("routingGroups.editMembers", { defaultValue: "编辑成员" })}
                </Button>
              </div>
            </div>
          ))}
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
