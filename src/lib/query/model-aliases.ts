import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import {
  modelAliasesApi,
  type ModelAlias,
  type ModelAliasSuggestion,
} from "@/lib/api/model-aliases";
import { extractErrorMessage } from "@/utils/errorUtils";

const modelAliasesKey = (appType: string) => ["modelAliases", appType] as const;

export function useModelAliases(appType: string, enabled = true) {
  return useQuery({
    queryKey: modelAliasesKey(appType),
    queryFn: () => modelAliasesApi.list(appType),
    enabled: enabled && !!appType,
  });
}

/**
 * 整体替换某逻辑模型的渠道映射。
 *
 * 采用「按逻辑模型整组保存」而不是逐条增删，是因为 UI 的编辑单元就是一个
 * 逻辑模型 + 它的全部渠道；逐条保存会让中间状态（少一个渠道）短暂生效，
 * 而这段时间里的请求可能选到错误渠道。
 */
export function useSetModelAliasEntries() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (params: {
      appType: string;
      alias: string;
      entries: Array<[string, string]>;
    }) =>
      modelAliasesApi.setEntries(params.appType, params.alias, params.entries),
    onSuccess: (_, params) => {
      queryClient.invalidateQueries({
        queryKey: modelAliasesKey(params.appType),
      });
      toast.success(
        t("modelAliases.saved", { defaultValue: "模型别名已保存" }),
      );
    },
    onError: (error: unknown) =>
      toast.error(
        extractErrorMessage(error as Error) ||
          t("modelAliases.saveFailed", { defaultValue: "保存失败" }),
      ),
  });
}

/**
 * 按主键删除单条 alias 记录。
 *
 * 当前 UI 走的是 `useSetModelAliasEntries`（整组替换，传空数组即删除整个逻辑模型），
 * 因此**没有调用点**；保留它是为脚本化 / 未来的逐条编辑入口（后端对应命令同样保留）。
 * 若确认不再需要，前后端应成对移除。
 */
export function useDeleteModelAlias() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (params: { appType: string; id: string }) =>
      modelAliasesApi.remove(params.appType, params.id),
    onSuccess: (_, params) => {
      queryClient.invalidateQueries({
        queryKey: modelAliasesKey(params.appType),
      });
      toast.success(t("modelAliases.deleted", { defaultValue: "已删除" }));
    },
    onError: (error: unknown) =>
      toast.error(
        extractErrorMessage(error as Error) ||
          t("modelAliases.deleteFailed", { defaultValue: "删除失败" }),
      ),
  });
}

/**
 * 供调用方判断是否已启用别名路由（决定是否展示提示条）。
 *
 * 面板内的「已启用」徽章改为直接用 `useModelAliases` 的结果长度推导，
 * 以避免多一次 IPC；该 hook 保留给需要独立判断的场景。
 */
export function useHasModelAliases(appType: string, enabled = true) {
  const { data } = useModelAliases(appType, enabled);
  return (data?.length ?? 0) > 0;
}

/**
 * 从已有 `model_mapping` 归并出的建议。
 *
 * 按需触发（`enabled` 由调用方控制），因为要扫描该 app 下所有 provider 的
 * settings_config，不适合每次进页面都跑。
 */
export function useModelAliasSuggestions(appType: string, enabled = false) {
  return useQuery({
    queryKey: ["modelAliasSuggestions", appType],
    queryFn: () => modelAliasesApi.suggest(appType),
    enabled: enabled && !!appType,
    staleTime: 0,
  });
}

export type { ModelAlias, ModelAliasSuggestion };
