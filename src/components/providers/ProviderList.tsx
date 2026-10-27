import { CSS } from "@dnd-kit/utilities";
import { DndContext, closestCenter } from "@dnd-kit/core";
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  Activity,
  AlertTriangle,
  CheckSquare,
  ChevronDown,
  Search,
  Square,
  Trash2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import type { Provider } from "@/types";
import type { AppId } from "@/lib/api";
import { providersApi } from "@/lib/api/providers";
import { extractErrorMessage } from "@/utils/errorUtils";
import { useDragSort } from "@/hooks/useDragSort";
import {
  useOpenClawLiveProviderIds,
  useOpenClawDefaultModel,
} from "@/hooks/useOpenClaw";
import {
  useHermesLiveProviderIds,
  useHermesModelConfig,
} from "@/hooks/useHermes";
import { useStreamCheck } from "@/hooks/useStreamCheck";
import { getLatestStreamChecks } from "@/lib/api/connectivity-check";
import { ProviderCard } from "@/components/providers/ProviderCard";
import { ProviderEmptyState } from "@/components/providers/ProviderEmptyState";
import {
  useAutoFailoverEnabled,
  useFailoverQueue,
  useAddToFailoverQueue,
  useRemoveFromFailoverQueue,
} from "@/lib/query/failover";
import {
  useCurrentOmoProviderId,
  useCurrentOmoSlimProviderId,
} from "@/lib/query/omo";
import { useCallback } from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { isTextEditableTarget } from "@/utils/domUtils";
import { usePiCurrentState } from "@/lib/query/pi";
import { isProxyAppId } from "@/config/appConfig";

interface ProviderListProps {
  providers: Record<string, Provider>;
  currentProviderId: string;
  appId: AppId;
  onSwitch: (provider: Provider) => void;
  onEdit: (provider: Provider) => void;
  onDelete: (provider: Provider) => void;
  onRemoveFromConfig?: (provider: Provider) => void;
  onDisableOmo?: () => void;
  onDisableOmoSlim?: () => void;
  onDuplicate: (provider: Provider) => void;
  /** Claude 卡片专属：把该供应商的 API 设置转换为 Codex 供应商 */
  onConvertToCodex?: (provider: Provider) => void;
  onConfigureUsage?: (provider: Provider) => void;
  onOpenWebsite: (url: string) => void;
  onOpenTerminal?: (provider: Provider) => void;
  onCreate?: () => void;
  isLoading?: boolean;
  isProxyRunning?: boolean; // 代理服务运行状态
  /** 代理进程是否在运行；与 isProxyRunning 的区别见 SortableProviderCard 的同名 prop */
  isProxyServiceRunning?: boolean;
  isProxyTakeover?: boolean; // 代理接管模式（Live配置已被接管）
  activeProviderId?: string; // 代理当前实际使用的供应商 ID（用于故障转移模式下标注绿色边框）
  onSetAsDefault?: (provider: Provider, modelId?: string) => void; // OpenClaw: set as default model
}

export function ProviderList({
  providers,
  currentProviderId,
  appId,
  onSwitch,
  onEdit,
  onDelete,
  onRemoveFromConfig,
  onDisableOmo,
  onDisableOmoSlim,
  onDuplicate,
  onConvertToCodex,
  onConfigureUsage,
  onOpenWebsite,
  onOpenTerminal,
  onCreate,
  isLoading = false,
  isProxyRunning = false,
  isProxyServiceRunning = false,
  isProxyTakeover = false,
  activeProviderId,
  onSetAsDefault,
}: ProviderListProps) {
  const { t } = useTranslation();
  const { checkProvider, checkAllProviders, isChecking, checkingAll } =
    useStreamCheck(appId);
  const { sortedProviders, sensors, handleDragEnd } = useDragSort(
    providers,
    appId,
  );

  const { data: opencodeLiveIds } = useQuery({
    queryKey: ["opencodeLiveProviderIds"],
    queryFn: () => providersApi.getOpenCodeLiveProviderIds(),
    enabled: appId === "opencode",
  });

  // OpenClaw: 查询 live 配置中的供应商 ID 列表，用于判断 isInConfig
  const { data: openclawLiveIds } = useOpenClawLiveProviderIds(
    appId === "openclaw",
  );

  // Hermes: 查询 live 配置中的供应商 ID 列表，用于判断 isInConfig
  const { data: hermesLiveIds } = useHermesLiveProviderIds(appId === "hermes");

  // Hermes: 读取当前 model.provider，用于判断哪个供应商是"当前激活"（高亮）
  const { data: hermesModelConfig } = useHermesModelConfig(appId === "hermes");
  const hermesCurrentProviderId = hermesModelConfig?.provider;

  // 判断供应商是否已添加到配置（累加模式应用：OpenCode/OpenClaw/Hermes）
  const isProviderInConfig = useCallback(
    (providerId: string): boolean => {
      if (appId === "opencode") {
        return opencodeLiveIds?.includes(providerId) ?? false;
      }
      if (appId === "openclaw") {
        return openclawLiveIds?.includes(providerId) ?? false;
      }
      if (appId === "hermes") {
        return hermesLiveIds?.includes(providerId) ?? false;
      }
      return true; // 其他应用始终返回 true
    },
    [appId, opencodeLiveIds, openclawLiveIds, hermesLiveIds],
  );

  // OpenClaw: query default model to determine which provider is default
  const { data: openclawDefaultModel } = useOpenClawDefaultModel(
    appId === "openclaw",
  );

  const isProviderDefaultModel = useCallback(
    (providerId: string): boolean => {
      if (appId !== "openclaw" || !openclawDefaultModel?.primary) return false;
      return openclawDefaultModel.primary.startsWith(providerId + "/");
    },
    [appId, openclawDefaultModel],
  );

  // Only apps with an explicit local-routing capability participate in
  // failover. Additive apps such as Pi never query or render this state.
  const supportsFailover = isProxyAppId(appId);
  const { data: isAutoFailoverEnabled } = useAutoFailoverEnabled(
    appId,
    supportsFailover,
  );
  const { data: failoverQueue } = useFailoverQueue(appId, supportsFailover);
  const addToQueue = useAddToFailoverQueue();
  const removeFromQueue = useRemoveFromFailoverQueue();

  const isFailoverModeActive =
    supportsFailover &&
    isProxyTakeover === true &&
    isAutoFailoverEnabled === true;

  const isOpenCode = appId === "opencode";
  const { data: currentOmoId } = useCurrentOmoProviderId(isOpenCode);
  const { data: currentOmoSlimId } = useCurrentOmoSlimProviderId(isOpenCode);

  const getFailoverPriority = useCallback(
    (providerId: string): number | undefined => {
      if (!isFailoverModeActive || !failoverQueue) return undefined;
      const index = failoverQueue.findIndex(
        (item) => item.providerId === providerId,
      );
      return index >= 0 ? index + 1 : undefined;
    },
    [isFailoverModeActive, failoverQueue],
  );

  const isInFailoverQueue = useCallback(
    (providerId: string): boolean => {
      if (!isFailoverModeActive || !failoverQueue) return false;
      return failoverQueue.some((item) => item.providerId === providerId);
    },
    [isFailoverModeActive, failoverQueue],
  );

  const handleToggleFailover = useCallback(
    (providerId: string, enabled: boolean) => {
      if (enabled) {
        addToQueue.mutate({ appType: appId, providerId });
      } else {
        removeFromQueue.mutate({ appType: appId, providerId });
      }
    },
    [appId, addToQueue, removeFromQueue],
  );

  const [searchTerm, setSearchTerm] = useState("");
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // 定制：排序模式（manual=拖拽原序 / name / health / latency）
  const [sortMode, setSortMode] = useState<
    "manual" | "name" | "health" | "latency"
  >("manual");
  // 定制：多选模式与已选集合
  const [selectMode, setSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  // 定制：最近一次连通性检测结果（延迟/健康排序 + 选中不可达 + 卡片延迟徽章）
  // 常驻查询：进入页面即有数据，批量检测后失效刷新
  const { data: latestChecks = [], refetch: refetchLatestChecks } = useQuery({
    queryKey: ["latestStreamChecks", appId],
    queryFn: () => getLatestStreamChecks(appId),
  });
  const { data: claudeDesktopStatus } = useQuery({
    queryKey: ["claudeDesktopStatus"],
    queryFn: () => providersApi.getClaudeDesktopStatus(),
    enabled: appId === "claude-desktop",
    refetchInterval: appId === "claude-desktop" ? 5000 : false,
  });
  const {
    data: piCurrentState,
    isSuccess: isPiCurrentStateSuccess,
    isError: isPiCurrentStateError,
    error: piCurrentStateError,
  } = usePiCurrentState(appId === "pi");
  const isPiAuthoritativeStateReady = appId !== "pi" || isPiCurrentStateSuccess;
  const isPiProviderInConfig = useCallback(
    (provider: Provider): boolean => {
      if (!isPiAuthoritativeStateReady) return false;
      return piCurrentState?.enabledProviderIds.includes(provider.id) ?? false;
    },
    [isPiAuthoritativeStateReady, piCurrentState],
  );

  // 连通性检查不发真实请求、无封号/计费风险，直接执行（无需确认弹窗）。
  const handleTest = useCallback(
    (provider: Provider) => {
      checkProvider(provider.id, provider.name);
    },
    [checkProvider],
  );

  // 批量连通性检查：一次探测全部供应商并汇总结果，完成后刷新最近检测缓存。
  const handleCheckAll = useCallback(async () => {
    const results = await checkAllProviders(false);
    if (results) {
      void refetchLatestChecks();
    }
  }, [checkAllProviders, refetchLatestChecks]);

  // 进入多选模式时预取最近检测结果（用于"选中不可达"）
  const enterSelectMode = useCallback(() => {
    setSelectMode(true);
    setSelectedIds(new Set());
  }, []);

  const exitSelectMode = useCallback(() => {
    setSelectMode(false);
    setSelectedIds(new Set());
  }, []);

  const toggleSelected = useCallback((providerId: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(providerId)) {
        next.delete(providerId);
      } else {
        next.add(providerId);
      }
      return next;
    });
  }, []);

  // 选中全部不可达供应商（依赖最近检测结果）
  const selectUnreachable = useCallback(() => {
    const failedIds = new Set(
      latestChecks.filter((c) => !c.success).map((c) => c.providerId),
    );
    setSelectedIds(failedIds);
  }, [latestChecks]);

  // 批量删除：逐个调用现有 delete API，汇总成功/失败数
  const [deletingBatch, setDeletingBatch] = useState(false);
  const handleBatchDelete = useCallback(async () => {
    if (selectedIds.size === 0) return;
    const names = sortedProviders
      .filter((p) => selectedIds.has(p.id))
      .map((p) => p.name);
    const confirmed = window.confirm(
      t("provider.batchDeleteConfirm", {
        count: selectedIds.size,
        names: names.slice(0, 5).join(", "),
        defaultValue: `确定删除 ${selectedIds.size} 个供应商吗？\n${names.slice(0, 5).join(", ")}${names.length > 5 ? " 等" : ""}\n此操作不可撤销。`,
      }),
    );
    if (!confirmed) return;

    setDeletingBatch(true);
    let ok = 0;
    let fail = 0;
    const results = await Promise.allSettled(
      [...selectedIds].map((id) => providersApi.delete(id, appId)),
    );
    for (const r of results) {
      if (r.status === "fulfilled") ok++;
      else fail++;
    }
    setDeletingBatch(false);

    if (fail === 0) {
      toast.success(
        t("provider.batchDeleteDone", {
          count: ok,
          defaultValue: `已删除 ${ok} 个供应商`,
        }),
      );
    } else {
      toast.error(
        t("provider.batchDeletePartial", {
          ok,
          fail,
          defaultValue: `删除完成：成功 ${ok} 个，失败 ${fail} 个`,
        }),
      );
    }
    exitSelectMode();
  }, [selectedIds, sortedProviders, appId, t, exitSelectMode]);

  // Import current live config as default provider
  const queryClient = useQueryClient();
  const importMutation = useMutation({
    mutationFn: async (): Promise<boolean> => {
      if (appId === "opencode") {
        const count = await providersApi.importOpenCodeFromLive();
        return count > 0;
      }
      if (appId === "openclaw") {
        const count = await providersApi.importOpenClawFromLive();
        return count > 0;
      }
      if (appId === "hermes") {
        const count = await providersApi.importHermesFromLive();
        return count > 0;
      }
      if (appId === "claude-desktop") {
        const count = await providersApi.importClaudeDesktopFromClaude();
        return count > 0;
      }
      return providersApi.importDefault(appId);
    },
    onSuccess: (imported) => {
      if (imported) {
        queryClient.invalidateQueries({ queryKey: ["providers", appId] });
        if (appId === "claude-desktop") {
          queryClient.invalidateQueries({ queryKey: ["claudeDesktopStatus"] });
        }
        toast.success(t("provider.importCurrentDescription"));
      } else {
        toast.info(t("provider.noProviders"));
      }
    },
    onError: (error: unknown) => {
      // Tauri invoke 的 reject 值是后端序列化出的纯字符串而非 Error 对象，
      // 取 .message 只会得到 undefined（空 toast）。
      toast.error(extractErrorMessage(error) || t("settings.importFailed"));
      // 导入失败前也可能已产生需要上屏的副作用：GrokBuild 官方登录态下点
      // 导入，命令层会先补种官方条目、随后才因 live 不可导入而报错。
      queryClient.invalidateQueries({ queryKey: ["providers", appId] });
    },
  });

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;

      const key = event.key.toLowerCase();
      if ((event.metaKey || event.ctrlKey) && key === "f") {
        // 正在输入框/可编辑区域中时不抢占 Ctrl+F（例如添加供应商表单里
        // ProviderPresetSelector 的搜索框），避免与其同名快捷键冲突。
        if (isTextEditableTarget(document.activeElement)) return;
        event.preventDefault();
        setIsSearchOpen(true);
        return;
      }

      if (key === "escape") {
        setIsSearchOpen(false);
      }
    };

    globalThis.addEventListener("keydown", handleKeyDown);
    return () => globalThis.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (isSearchOpen) {
      const frame = requestAnimationFrame(() => {
        searchInputRef.current?.focus();
        searchInputRef.current?.select();
      });
      return () => cancelAnimationFrame(frame);
    }
  }, [isSearchOpen]);

  // 按模式排序的展示列表（manual = useDragSort 的原序）
  const displayProviders = useMemo(() => {
    if (sortMode === "manual") return sortedProviders;
    const list = [...sortedProviders];
    if (sortMode === "name") {
      return list.sort((a, b) => a.name.localeCompare(b.name));
    }
    const checkMap = new Map(latestChecks.map((c) => [c.providerId, c]));
    if (sortMode === "health") {
      // 不可达在前，其次未检测，可达在后；同组内保持原序
      const rank = (id: string) => {
        const c = checkMap.get(id);
        if (!c) return 1;
        return c.success ? 2 : 0;
      };
      return list.sort((a, b) => rank(a.id) - rank(b.id));
    }
    // latency：有延迟数据的按延迟升序，无数据的排最后
    return list.sort((a, b) => {
      const la = checkMap.get(a.id)?.responseTimeMs;
      const lb = checkMap.get(b.id)?.responseTimeMs;
      if (la === undefined && lb === undefined) return 0;
      if (la === undefined) return 1;
      if (lb === undefined) return -1;
      return la - lb;
    });
  }, [sortedProviders, sortMode, latestChecks]);

  const filteredProviders = useMemo(() => {
    const keyword = searchTerm.trim().toLowerCase();
    if (!keyword) return displayProviders;
    return displayProviders.filter((provider) => {
      const fields = [provider.name, provider.notes, provider.websiteUrl];
      return fields.some((field) =>
        field?.toString().toLowerCase().includes(keyword),
      );
    });
  }, [searchTerm, displayProviders]);

  const claudeDesktopStatusMessages = useMemo(() => {
    if (appId !== "claude-desktop" || !claudeDesktopStatus) return [];

    const messages: string[] = [];
    if (!claudeDesktopStatus.supported) {
      messages.push(
        t("claudeDesktop.statusUnsupported", {
          defaultValue: "当前平台暂不支持 Claude Desktop 3P 配置写入。",
        }),
      );
      return messages;
    }

    if (claudeDesktopStatus.staleRawModels) {
      messages.push(
        t("claudeDesktop.statusStaleRawModels", {
          defaultValue:
            "Claude Desktop profile 中存在非 claude-* 模型名，新版 Claude Desktop 可能拒绝加载；重新切换当前供应商可修复。",
        }),
      );
    }
    if (claudeDesktopStatus.missingRouteMappings) {
      messages.push(
        t("claudeDesktop.statusMissingRouteMappings", {
          defaultValue:
            "当前供应商启用了模型映射，但没有有效路由；请编辑供应商并补全至少一个模型映射。",
        }),
      );
    }
    if (
      claudeDesktopStatus.mode === "proxy" &&
      !claudeDesktopStatus.gatewayTokenConfigured
    ) {
      messages.push(
        t("claudeDesktop.statusGatewayTokenMissing", {
          defaultValue:
            "当前本地路由 token 尚未生成；重新切换该供应商会写入新的本地 token。",
        }),
      );
    }

    const expected = claudeDesktopStatus.expectedBaseUrl?.replace(/\/+$/, "");
    const actual = claudeDesktopStatus.actualBaseUrl?.replace(/\/+$/, "");
    if (expected && actual && expected !== actual) {
      messages.push(
        t("claudeDesktop.statusBaseUrlMismatch", {
          expected,
          actual,
          defaultValue:
            "Claude Desktop profile 指向的地址与当前供应商不一致；当前为 {{actual}}，应为 {{expected}}。重新切换当前供应商可修复。",
        }),
      );
    }

    return messages;
  }, [appId, claudeDesktopStatus, t]);

  const piStateErrorMessages = [
    isPiCurrentStateError ? extractErrorMessage(piCurrentStateError) : "",
  ].filter(Boolean);
  const piStateErrorNotice =
    appId === "pi" && piStateErrorMessages.length > 0 ? (
      <div
        role="alert"
        className="rounded-lg border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-sm text-amber-900 dark:text-amber-200"
      >
        <div className="flex items-center gap-2 font-medium">
          <AlertTriangle className="h-4 w-4 shrink-0" />
          {t("pi.current.readFailed", {
            defaultValue: "无法读取 Pi 当前配置",
          })}
        </div>
        <p className="mt-1 text-xs leading-relaxed">
          {t("pi.current.stateUnavailableHint")}
          {piStateErrorMessages.length > 0
            ? ` ${piStateErrorMessages.join(" · ")}`
            : ""}
        </p>
      </div>
    ) : null;

  if (isLoading) {
    return (
      <div className="space-y-3">
        {[0, 1, 2].map((index) => (
          <div
            key={index}
            className="w-full border border-dashed rounded-lg h-28 border-muted-foreground/40 bg-muted/40"
          />
        ))}
      </div>
    );
  }

  if (sortedProviders.length === 0) {
    return (
      <div className="mt-4 space-y-4">
        {piStateErrorNotice}
        <ProviderEmptyState
          appId={appId}
          onCreate={appId === "pi" ? undefined : onCreate}
          onImport={appId === "pi" ? undefined : () => importMutation.mutate()}
        />
      </div>
    );
  }

  const renderProviderList = () => (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={handleDragEnd}
    >
      <SortableContext
        items={filteredProviders.map((provider) => provider.id)}
        strategy={verticalListSortingStrategy}
      >
        <div className="space-y-3">
          {filteredProviders.map((provider) => {
            const isOmo = provider.category === "omo";
            const isOmoSlim = provider.category === "omo-slim";
            const isOmoCurrent = isOmo && provider.id === (currentOmoId || "");
            const isOmoSlimCurrent =
              isOmoSlim && provider.id === (currentOmoSlimId || "");
            const isHermesCurrent =
              appId === "hermes" && hermesCurrentProviderId === provider.id;
            const isCurrent =
              appId === "pi"
                ? false
                : isOmo
                  ? isOmoCurrent
                  : isOmoSlim
                    ? isOmoSlimCurrent
                    : appId === "hermes"
                      ? isHermesCurrent
                      : provider.id === currentProviderId;
            return (
              <SortableProviderCard
                key={provider.id}
                provider={provider}
                isCurrent={isCurrent}
                appId={appId}
                isInConfig={
                  appId === "pi"
                    ? isPiProviderInConfig(provider)
                    : isProviderInConfig(provider.id)
                }
                isOmo={isOmo}
                isOmoSlim={isOmoSlim}
                onSwitch={onSwitch}
                onEdit={onEdit}
                onDelete={onDelete}
                onRemoveFromConfig={onRemoveFromConfig}
                onDisableOmo={onDisableOmo}
                onDisableOmoSlim={onDisableOmoSlim}
                onDuplicate={onDuplicate}
                onConvertToCodex={
                  appId === "claude" ? onConvertToCodex : undefined
                }
                onConfigureUsage={onConfigureUsage}
                onOpenWebsite={onOpenWebsite}
                onOpenTerminal={onOpenTerminal}
                onTest={handleTest}
                isTesting={isChecking(provider.id)}
                isProxyRunning={supportsFailover && isProxyRunning}
                isProxyServiceRunning={isProxyServiceRunning}
                isProxyTakeover={supportsFailover && isProxyTakeover}
                isAutoFailoverEnabled={isFailoverModeActive}
                failoverPriority={getFailoverPriority(provider.id)}
                isInFailoverQueue={isInFailoverQueue(provider.id)}
                onToggleFailover={
                  supportsFailover
                    ? (enabled) => handleToggleFailover(provider.id, enabled)
                    : undefined
                }
                activeProviderId={
                  supportsFailover ? activeProviderId : undefined
                }
                isDefaultModel={
                  appId === "hermes"
                    ? isHermesCurrent
                    : isProviderDefaultModel(provider.id)
                }
                isRemovalProtected={
                  appId === "pi"
                    ? false
                    : appId === "hermes"
                      ? isHermesCurrent
                      : appId === "openclaw"
                        ? isProviderDefaultModel(provider.id)
                        : false
                }
                isStateChangeProtected={
                  appId === "pi" && !isPiAuthoritativeStateReady
                }
                onSetAsDefault={
                  onSetAsDefault
                    ? (modelId) => onSetAsDefault(provider, modelId)
                    : undefined
                }
                selectMode={selectMode}
                isSelected={selectedIds.has(provider.id)}
                onToggleSelect={toggleSelected}
                lastLatencyMs={
                  latestChecks.find((c) => c.providerId === provider.id)
                    ?.responseTimeMs
                }
              />
            );
          })}
        </div>
      </SortableContext>
    </DndContext>
  );

  return (
    <div className="mt-4 space-y-4">
      {piStateErrorNotice}
      {claudeDesktopStatusMessages.length > 0 && (
        <div className="rounded-lg border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-sm text-amber-900 dark:text-amber-200">
          <div className="flex items-center gap-2 font-medium">
            <AlertTriangle className="h-4 w-4 shrink-0" />
            {t("claudeDesktop.statusTitle", {
              defaultValue: "Claude Desktop 配置需要检查",
            })}
          </div>
          <ul className="mt-2 space-y-1 text-xs leading-relaxed">
            {claudeDesktopStatusMessages.map((message) => (
              <li key={message}>{message}</li>
            ))}
          </ul>
        </div>
      )}
      <div className="flex flex-wrap items-center justify-between gap-2">
        {/* 排序模式：manual=拖拽原序 */}
        <div className="relative">
          <select
            value={sortMode}
            onChange={(e) =>
              setSortMode(
                e.target.value as "manual" | "name" | "health" | "latency",
              )
            }
            className="h-8 appearance-none rounded-md border border-border bg-background pl-3 pr-8 text-xs text-foreground"
            aria-label={t("provider.sortBy", { defaultValue: "排序方式" })}
          >
            <option value="manual">
              {t("provider.sortManual", { defaultValue: "手动排序" })}
            </option>
            <option value="name">
              {t("provider.sortByName", { defaultValue: "按名称" })}
            </option>
            <option value="health">
              {t("provider.sortByHealth", { defaultValue: "按健康状态" })}
            </option>
            <option value="latency">
              {t("provider.sortByLatency", { defaultValue: "按延迟" })}
            </option>
          </select>
          <ChevronDown className="pointer-events-none absolute right-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            className="gap-1.5"
            disabled={checkingAll || sortedProviders.length === 0}
            onClick={handleCheckAll}
          >
            <Activity
              className={checkingAll ? "h-4 w-4 animate-pulse" : "h-4 w-4"}
            />
            {checkingAll
              ? t("streamCheck.batchRunning", { defaultValue: "检测中..." })
              : t("streamCheck.batchCheckAll", { defaultValue: "全部检测" })}
          </Button>

          {selectMode ? (
            <>
              <Button
                variant="outline"
                size="sm"
                className="gap-1.5"
                onClick={selectUnreachable}
                disabled={latestChecks.length === 0}
                title={t("provider.selectUnreachableHint", {
                  defaultValue: "选中最近一次检测中不可达的全部供应商",
                })}
              >
                <AlertTriangle className="h-4 w-4" />
                {t("provider.selectUnreachable", {
                  defaultValue: "选中不可达",
                })}
              </Button>
              <Button
                variant="destructive"
                size="sm"
                className="gap-1.5"
                disabled={selectedIds.size === 0 || deletingBatch}
                onClick={handleBatchDelete}
              >
                <Trash2 className="h-4 w-4" />
                {deletingBatch
                  ? t("provider.batchDeleting", { defaultValue: "删除中..." })
                  : t("provider.batchDelete", {
                      count: selectedIds.size,
                      defaultValue: `删除所选 (${selectedIds.size})`,
                    })}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={exitSelectMode}
                className="gap-1"
              >
                <X className="h-4 w-4" />
                {t("provider.cancelSelect", { defaultValue: "取消" })}
              </Button>
            </>
          ) : (
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5"
              disabled={sortedProviders.length === 0}
              onClick={enterSelectMode}
            >
              <CheckSquare className="h-4 w-4" />
              {t("provider.multiSelect", { defaultValue: "多选" })}
            </Button>
          )}
        </div>
      </div>

      <AnimatePresence>
        {isSearchOpen && (
          <motion.div
            key="provider-search"
            initial={{ opacity: 0, y: -8, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -8, scale: 0.98 }}
            transition={{ duration: 0.18, ease: "easeOut" }}
            className="fixed left-1/2 top-[6.5rem] z-40 w-[min(90vw,26rem)] -translate-x-1/2 sm:right-6 sm:left-auto sm:translate-x-0"
          >
            <div className="p-4 space-y-3 border shadow-md rounded-2xl border-white/10 bg-background/95 shadow-black/20 backdrop-blur-md">
              <div className="relative flex items-center gap-2">
                <Search className="absolute w-4 h-4 -translate-y-1/2 pointer-events-none left-3 top-1/2 text-muted-foreground" />
                <Input
                  ref={searchInputRef}
                  value={searchTerm}
                  onChange={(event) => setSearchTerm(event.target.value)}
                  placeholder={t("provider.searchPlaceholder", {
                    defaultValue: "Search name, notes, or URL...",
                  })}
                  aria-label={t("provider.searchAriaLabel", {
                    defaultValue: "Search providers",
                  })}
                  className="pr-16 pl-9"
                />
                {searchTerm && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="absolute text-xs -translate-y-1/2 right-11 top-1/2"
                    onClick={() => setSearchTerm("")}
                  >
                    {t("common.clear", { defaultValue: "Clear" })}
                  </Button>
                )}
                <Button
                  variant="ghost"
                  size="icon"
                  className="ml-auto"
                  onClick={() => setIsSearchOpen(false)}
                  aria-label={t("provider.searchCloseAriaLabel", {
                    defaultValue: "Close provider search",
                  })}
                >
                  <X className="w-4 h-4" />
                </Button>
              </div>
              <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] text-muted-foreground">
                <span>
                  {t("provider.searchScopeHint", {
                    defaultValue: "Matches provider name, notes, and URL.",
                  })}
                </span>
                <span>
                  {t("provider.searchCloseHint", {
                    defaultValue: "Press Esc to close",
                  })}
                </span>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {filteredProviders.length === 0 ? (
        <div className="px-6 py-8 text-sm text-center border border-dashed rounded-lg border-border text-muted-foreground">
          {t("provider.noSearchResults", {
            defaultValue: "No providers match your search.",
          })}
        </div>
      ) : (
        renderProviderList()
      )}
    </div>
  );
}

interface SortableProviderCardProps {
  provider: Provider;
  isCurrent: boolean;
  appId: AppId;
  isInConfig: boolean;
  isOmo: boolean;
  isOmoSlim: boolean;
  onSwitch: (provider: Provider) => void;
  onEdit: (provider: Provider) => void;
  onDelete: (provider: Provider) => void;
  onRemoveFromConfig?: (provider: Provider) => void;
  onDisableOmo?: () => void;
  onDisableOmoSlim?: () => void;
  onDuplicate: (provider: Provider) => void;
  /** Claude 卡片专属：把该供应商的 API 设置转换为 Codex 供应商 */
  onConvertToCodex?: (provider: Provider) => void;
  onConfigureUsage?: (provider: Provider) => void;
  onOpenWebsite: (url: string) => void;
  onOpenTerminal?: (provider: Provider) => void;
  onTest?: (provider: Provider) => void;
  isTesting: boolean;
  isProxyRunning: boolean;
  isProxyTakeover: boolean;
  /** 代理进程是否在运行（未经 supportsFailover 短路，供健康轮询判断） */
  isProxyServiceRunning?: boolean;
  isAutoFailoverEnabled: boolean;
  failoverPriority?: number;
  isInFailoverQueue: boolean;
  onToggleFailover?: (enabled: boolean) => void;
  activeProviderId?: string;
  // OpenClaw: default model
  isDefaultModel?: boolean;
  isRemovalProtected?: boolean;
  isStateChangeProtected?: boolean;
  onSetAsDefault?: (modelId?: string) => void;
  // 定制：多选模式
  selectMode?: boolean;
  isSelected?: boolean;
  onToggleSelect?: (providerId: string) => void;
  // 定制：最近检测延迟（毫秒），无数据为 undefined
  lastLatencyMs?: number;
}

function SortableProviderCard({
  provider,
  isCurrent,
  appId,
  isInConfig,
  isOmo,
  isOmoSlim,
  onSwitch,
  onEdit,
  onDelete,
  onRemoveFromConfig,
  onDisableOmo,
  onDisableOmoSlim,
  onDuplicate,
  onConvertToCodex,
  onConfigureUsage,
  onOpenWebsite,
  onOpenTerminal,
  onTest,
  isTesting,
  isProxyRunning,
  isProxyServiceRunning,
  isProxyTakeover,
  isAutoFailoverEnabled,
  failoverPriority,
  isInFailoverQueue,
  onToggleFailover,
  activeProviderId,
  isDefaultModel,
  isRemovalProtected,
  isStateChangeProtected,
  onSetAsDefault,
  selectMode,
  isSelected,
  onToggleSelect,
  lastLatencyMs,
}: SortableProviderCardProps) {
  const {
    setNodeRef,
    attributes,
    listeners,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: provider.id });

  const style: CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={style} className="relative">
      {selectMode && (
        <button
          type="button"
          className="absolute left-2 top-2 z-10 flex h-6 w-6 items-center justify-center rounded-md border bg-background/95 shadow-sm"
          aria-label={isSelected ? "deselect" : "select"}
          onClick={(event) => {
            event.stopPropagation();
            onToggleSelect?.(provider.id);
          }}
        >
          {isSelected ? (
            <CheckSquare className="h-4 w-4 text-primary" />
          ) : (
            <Square className="h-4 w-4 text-muted-foreground" />
          )}
        </button>
      )}
      <div
        className={selectMode ? "cursor-pointer" : undefined}
        onClick={
          selectMode
            ? (event) => {
                event.stopPropagation();
                onToggleSelect?.(provider.id);
              }
            : undefined
        }
      >
        <ProviderCard
          provider={provider}
          isCurrent={isCurrent}
          appId={appId}
          isInConfig={isInConfig}
          isOmo={isOmo}
          isOmoSlim={isOmoSlim}
          onSwitch={onSwitch}
          onEdit={onEdit}
          onDelete={onDelete}
          onRemoveFromConfig={onRemoveFromConfig}
          onDisableOmo={onDisableOmo}
          onDisableOmoSlim={onDisableOmoSlim}
          onDuplicate={onDuplicate}
          onConvertToCodex={onConvertToCodex}
          onConfigureUsage={
            onConfigureUsage
              ? (item) => onConfigureUsage(item)
              : () => undefined
          }
          onOpenWebsite={onOpenWebsite}
          onOpenTerminal={onOpenTerminal}
          onTest={onTest}
          isTesting={isTesting}
          isProxyRunning={isProxyRunning}
          isProxyServiceRunning={isProxyServiceRunning}
          isProxyTakeover={isProxyTakeover}
          dragHandleProps={{
            attributes,
            listeners,
            isDragging,
          }}
          isAutoFailoverEnabled={isAutoFailoverEnabled}
          failoverPriority={failoverPriority}
          isInFailoverQueue={isInFailoverQueue}
          onToggleFailover={onToggleFailover}
          activeProviderId={activeProviderId}
          // OpenClaw: default model
          isDefaultModel={isDefaultModel}
          isRemovalProtected={isRemovalProtected}
          isStateChangeProtected={isStateChangeProtected}
          onSetAsDefault={onSetAsDefault}
          lastLatencyMs={lastLatencyMs}
        />
      </div>
    </div>
  );
}
