import { useState, useCallback } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import {
  streamCheckAllProviders,
  streamCheckProvider,
  type StreamCheckResult,
} from "@/lib/api/connectivity-check";
import type { AppId } from "@/lib/api";

/**
 * 供应商连通性检查。
 *
 * 只探测 base_url 是否可达（任何 HTTP 响应都算可达），不发真实大模型请求。
 * 刻意 **不** 重置故障转移熔断器——可达 ≠ 配置正确，一个端口通但鉴权废的供应商
 * 不应被误判为"健康"而切回线上。熔断器只由真实转发流量驱动（见 proxy/forwarder.rs）。
 */
export function useStreamCheck(appId: AppId) {
  const { t } = useTranslation();
  const [checkingIds, setCheckingIds] = useState<Set<string>>(new Set());
  const [checkingAll, setCheckingAll] = useState(false);

  const checkProvider = useCallback(
    async (
      providerId: string,
      providerName: string,
    ): Promise<StreamCheckResult | null> => {
      setCheckingIds((prev) => new Set(prev).add(providerId));

      try {
        const result = await streamCheckProvider(appId, providerId);

        if (result.status === "operational") {
          toast.success(
            t("streamCheck.reachable", {
              providerName: providerName,
              responseTimeMs: result.responseTimeMs,
              defaultValue: `${providerName} 连通正常 (${result.responseTimeMs}ms)`,
            }),
            { closeButton: true },
          );
        } else if (result.status === "degraded") {
          toast.warning(
            t("streamCheck.reachableSlow", {
              providerName: providerName,
              responseTimeMs: result.responseTimeMs,
              defaultValue: `${providerName} 连通但较慢 (${result.responseTimeMs}ms)`,
            }),
          );
        } else {
          // 仅当无法建立连接（DNS / 连接被拒 / TLS / 超时）才会到这里
          toast.error(
            t("streamCheck.unreachable", {
              providerName: providerName,
              message: result.message,
              defaultValue: `${providerName} 无法连通: ${result.message}`,
            }),
            {
              description: t("streamCheck.unreachableHint", {
                defaultValue:
                  "无法建立连接（DNS / 连接 / TLS / 超时）。请检查 base_url 与网络。",
              }),
              duration: 8000,
              closeButton: true,
            },
          );
        }

        return result;
      } catch (e) {
        toast.error(
          t("streamCheck.error", {
            providerName: providerName,
            error: String(e),
            defaultValue: `${providerName} 检查出错: ${String(e)}`,
          }),
        );
        return null;
      } finally {
        setCheckingIds((prev) => {
          const next = new Set(prev);
          next.delete(providerId);
          return next;
        });
      }
    },
    [appId, t],
  );

  const isChecking = useCallback(
    (providerId: string) => checkingIds.has(providerId),
    [checkingIds],
  );

  /**
   * 批量连通性检查：一次探测当前应用下所有（或仅代理目标）供应商，
   * 汇总可达/缓慢/不可达数量，并把不可达的以摘要形式提示。
   */
  const checkAllProviders = useCallback(
    async (
      proxyTargetsOnly: boolean = false,
    ): Promise<Array<[string, StreamCheckResult]> | null> => {
      setCheckingAll(true);
      try {
        const results = await streamCheckAllProviders(appId, proxyTargetsOnly);
        const failed = results.filter(([, r]) => !r.success);
        const degraded = results.filter(
          ([, r]) => r.success && r.status === "degraded",
        );

        if (failed.length === 0 && degraded.length === 0) {
          toast.success(
            t("streamCheck.batchAllReachable", {
              count: results.length,
              defaultValue: `全部 ${results.length} 个供应商可达`,
            }),
          );
        } else if (failed.length === 0) {
          toast.warning(
            t("streamCheck.batchDegraded", {
              count: degraded.length,
              defaultValue: `${degraded.length} 个供应商响应缓慢`,
            }),
          );
        } else {
          toast.error(
            t("streamCheck.batchFailed", {
              failed: failed.length,
              total: results.length,
              defaultValue: `${failed.length}/${results.length} 个供应商不可达`,
            }),
            {
              description: failed
                .map(([, r]) => r.message)
                .filter(Boolean)
                .slice(0, 3)
                .join("\n"),
              duration: 10000,
              closeButton: true,
            },
          );
        }
        return results;
      } catch (e) {
        toast.error(
          t("streamCheck.batchError", {
            error: String(e),
            defaultValue: `批量检测失败: ${String(e)}`,
          }),
        );
        return null;
      } finally {
        setCheckingAll(false);
      }
    },
    [appId, t],
  );

  return { checkProvider, checkAllProviders, isChecking, checkingAll };
}
