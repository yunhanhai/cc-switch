import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { routingGroupsApi } from "@/lib/api/routing-groups";
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
    onSuccess: (_, vars) => {
      qc.invalidateQueries({ queryKey: ["routingGroups", vars.appType] });
    },
    onError: (e: unknown) =>
      toast.error(extractErrorMessage(e as Error) || "排序失败"),
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
    onSuccess: (_, vars) => {
      qc.invalidateQueries({ queryKey: ["routingGroups", vars.appType] });
    },
    onError: (e: unknown) =>
      toast.error(extractErrorMessage(e as Error) || "排序失败"),
  });
}
