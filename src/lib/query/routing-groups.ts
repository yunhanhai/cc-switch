import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  routingGroupsApi,
  type RoutingGroupWithMembers,
} from "@/lib/api/routing-groups";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { extractErrorMessage } from "@/utils/errorUtils";

export function useRoutingGroups(appType: string, enabled = true) {
  return useQuery({
    queryKey: ["routingGroups", appType],
    queryFn: () => routingGroupsApi.list(appType),
    enabled: enabled && !!appType,
  });
}

export function useCreateRoutingGroup() {
  const qc = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: ({ appType, name }: { appType: string; name: string }) =>
      routingGroupsApi.create(appType, name),
    onSuccess: (_, vars) => {
      qc.invalidateQueries({ queryKey: ["routingGroups", vars.appType] });
      toast.success(
        t("routingGroups.createSuccess", { defaultValue: "路由分组已创建" }),
      );
    },
    onError: (e: unknown) =>
      toast.error(
        extractErrorMessage(e as Error) ||
          t("common.error", { defaultValue: "操作失败" }),
      ),
  });
}

export function useUpdateRoutingGroup() {
  const qc = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: (p: {
      id: string;
      appType: string;
      name?: string;
      enabled?: boolean;
    }) => routingGroupsApi.update(p.id, { name: p.name, enabled: p.enabled }),
    onSuccess: (_, vars) => {
      qc.invalidateQueries({ queryKey: ["routingGroups", vars.appType] });
      toast.success(
        t("routingGroups.updateSuccess", { defaultValue: "已更新" }),
      );
    },
    onError: (e: unknown) =>
      toast.error(
        extractErrorMessage(e as Error) ||
          t("common.error", { defaultValue: "操作失败" }),
      ),
  });
}

export function useDeleteRoutingGroup() {
  const qc = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: (p: { id: string; appType: string }) =>
      routingGroupsApi.remove(p.id),
    onSuccess: (_, vars) => {
      qc.invalidateQueries({ queryKey: ["routingGroups", vars.appType] });
      toast.success(
        t("routingGroups.deleteSuccess", { defaultValue: "已删除" }),
      );
    },
    onError: (e: unknown) =>
      toast.error(
        extractErrorMessage(e as Error) ||
          t("common.error", { defaultValue: "操作失败" }),
      ),
  });
}

export function useSetRoutingGroupMembers() {
  const qc = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: (p: {
      groupId: string;
      appType: string;
      providerIds: string[];
    }) => routingGroupsApi.setMembers(p.groupId, p.providerIds),
    onSuccess: (_, vars) => {
      qc.invalidateQueries({ queryKey: ["routingGroups", vars.appType] });
      toast.success(
        t("routingGroups.membersSaved", { defaultValue: "成员已更新" }),
      );
    },
    onError: (e: unknown) =>
      toast.error(
        extractErrorMessage(e as Error) ||
          t("common.error", { defaultValue: "操作失败" }),
      ),
  });
}

export function useReorderRoutingGroups() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (p: { appType: string; orderedIds: string[] }) =>
      routingGroupsApi.reorderGroups(p.appType, p.orderedIds),
    onMutate: async (vars) => {
      await qc.cancelQueries({ queryKey: ["routingGroups", vars.appType] });
      const prev =
        qc.getQueryData<RoutingGroupWithMembers[]>([
          "routingGroups",
          vars.appType,
        ]) ?? null;
      if (prev) {
        const byId = new Map(prev.map((g) => [g.group.id, g]));
        const reordered: RoutingGroupWithMembers[] = [];
        for (const id of vars.orderedIds) {
          const g = byId.get(id);
          if (g) reordered.push(g);
        }
        // 追加未在 orderedIds 中的分组（防御性，避免因并发丢失）
        for (const g of prev) {
          if (!vars.orderedIds.includes(g.group.id)) reordered.push(g);
        }
        qc.setQueryData<RoutingGroupWithMembers[]>(
          ["routingGroups", vars.appType],
          reordered,
        );
      }
      return { prev };
    },
    onError: (e: unknown, vars, ctx) => {
      const typed = ctx as
        | { prev: RoutingGroupWithMembers[] | null }
        | undefined;
      if (typed?.prev) {
        qc.setQueryData(["routingGroups", vars.appType], typed.prev);
      }
      toast.error(extractErrorMessage(e as Error) || "排序失败");
    },
    onSettled: (_, __, vars) => {
      qc.invalidateQueries({ queryKey: ["routingGroups", vars.appType] });
    },
  });
}

export function useReorderRoutingGroupMembers() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (p: {
      appType: string;
      groupId: string;
      orderedProviderIds: string[];
    }) => routingGroupsApi.reorderMembers(p.groupId, p.orderedProviderIds),
    onMutate: async (vars) => {
      await qc.cancelQueries({ queryKey: ["routingGroups", vars.appType] });
      const prev =
        qc.getQueryData<RoutingGroupWithMembers[]>([
          "routingGroups",
          vars.appType,
        ]) ?? null;
      if (prev) {
        const next = prev.map((entry) => {
          if (entry.group.id !== vars.groupId) return entry;
          const byPid = new Map(entry.members.map((m) => [m.providerId, m]));
          const reorderedMembers = vars.orderedProviderIds
            .map((pid) => byPid.get(pid))
            .filter(Boolean) as typeof entry.members;
          // 若有缺失则追加剩余（防御性）
          for (const m of entry.members) {
            if (!vars.orderedProviderIds.includes(m.providerId)) {
              reorderedMembers.push(m);
            }
          }
          // 同步 sort_index 为新下标，保持与后端一致的乐观视图
          const withIndex = reorderedMembers.map((m, idx) => ({
            ...m,
            sortIndex: idx,
          }));
          return { ...entry, members: withIndex };
        });
        qc.setQueryData<RoutingGroupWithMembers[]>(
          ["routingGroups", vars.appType],
          next,
        );
      }
      return { prev };
    },
    onError: (e: unknown, vars, ctx) => {
      const typed = ctx as
        | { prev: RoutingGroupWithMembers[] | null }
        | undefined;
      if (typed?.prev) {
        qc.setQueryData(["routingGroups", vars.appType], typed.prev);
      }
      toast.error(extractErrorMessage(e as Error) || "排序失败");
    },
    onSettled: (_, __, vars) => {
      qc.invalidateQueries({ queryKey: ["routingGroups", vars.appType] });
    },
  });
}
