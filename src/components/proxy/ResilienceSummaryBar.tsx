/**
 * 韧性健康摘要条（定制）
 *
 * 代理页顶部一屏总览：今日转移次数、429 等待统计、平均延迟、最慢站点、不可达数。
 * 数据来自 get_resilience_summary 命令，15 秒刷新。
 */

import { useTranslation } from "react-i18next";
import {
  ArrowLeftRight,
  Gauge,
  Hourglass,
  Timer,
  TrendingUp,
  WifiOff,
} from "lucide-react";
import { useResilienceSummary } from "@/lib/query/failover";

interface ResilienceSummaryBarProps {
  appType: string;
}

export function ResilienceSummaryBar({ appType }: ResilienceSummaryBarProps) {
  const { t } = useTranslation();
  const { data: summary } = useResilienceSummary(appType);

  if (!summary) return null;

  const items = [
    {
      icon: ArrowLeftRight,
      label: t("proxy.resilience.todayFailovers", {
        defaultValue: "今日转移",
      }),
      value: `${summary.todayFailoverCount}`,
      tone:
        summary.todayFailoverCount > 0
          ? "text-amber-600 dark:text-amber-400"
          : "text-foreground",
    },
    {
      icon: Timer,
      label: t("proxy.resilience.rateLimitWaits", {
        defaultValue: "429 等待",
      }),
      value:
        summary.rateLimitWaitCount > 0
          ? `${summary.rateLimitWaitCount} (${summary.rateLimitWaitTotalSecs}s)`
          : "0",
      tone:
        summary.rateLimitWaitCount > 0
          ? "text-teal-600 dark:text-teal-400"
          : "text-foreground",
    },
    {
      icon: Gauge,
      label: t("proxy.resilience.avgLatency", { defaultValue: "平均延迟" }),
      value: summary.avgLatencyMs
        ? `${summary.avgLatencyMs}ms`
        : t("proxy.resilience.noData", { defaultValue: "—" }),
      tone: "text-foreground",
    },
    {
      icon: TrendingUp,
      label: t("proxy.resilience.slowest", { defaultValue: "最慢站点" }),
      value: summary.slowestProvider
        ? `${summary.slowestProvider[0]} ${summary.slowestProvider[1]}ms`
        : t("proxy.resilience.noData", { defaultValue: "—" }),
      tone: "text-foreground",
    },
    {
      icon: WifiOff,
      label: t("proxy.resilience.unreachable", { defaultValue: "不可达" }),
      value: `${summary.unreachableCount}`,
      tone:
        summary.unreachableCount > 0
          ? "text-red-600 dark:text-red-400"
          : "text-emerald-600 dark:text-emerald-400",
    },
    {
      icon: Hourglass,
      label: t("proxy.resilience.coolingDown", { defaultValue: "冷却中" }),
      value:
        summary.coolingDownCount > 0
          ? `${summary.coolingDownCount} (${summary.longestCooldownSecs}s)`
          : "0",
      tone:
        summary.coolingDownCount > 0
          ? "text-amber-600 dark:text-amber-400"
          : "text-emerald-600 dark:text-emerald-400",
    },
  ];

  return (
    <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6">
      {items.map((item) => (
        <div
          key={item.label}
          className="flex flex-col gap-1 rounded-xl border border-border/60 bg-card/50 px-3 py-2.5"
          title={
            item.label ===
            t("proxy.resilience.coolingDown", { defaultValue: "冷却中" })
              ? t("proxy.resilience.coolingDownHint", {
                  defaultValue:
                    "上游返回 429 并要求等待的站点数。它们仍可被尝试，但在选路时排到队列末尾。",
                })
              : undefined
          }
        >
          <span className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
            <item.icon className="h-3 w-3" />
            {item.label}
          </span>
          <span
            className={`truncate text-sm font-semibold tabular-nums ${item.tone}`}
            title={item.value}
          >
            {item.value}
          </span>
        </div>
      ))}
    </div>
  );
}
