import { cn } from "@/lib/utils";
import { ProviderHealthStatus } from "@/types/proxy";
import { useTranslation } from "react-i18next";

interface ProviderHealthBadgeProps {
  consecutiveFailures: number;
  isHealthy?: boolean;
  /** 最近一次失败原因，悬停时展示，便于排查为何熔断 */
  lastError?: string | null;
  /** 熔断自动恢复探测的剩余秒数（>0 时显示倒计时） */
  recoveryRemainingSecs?: number;
  /** 429 限流冷却的剩余秒数（>0 时显示限流标记） */
  cooldownSecs?: number;
  /** 本次熔断是否由额度耗尽触发 */
  fatal?: boolean;
  /** 半开探测名额占用数（>0 且状态为 half_open 时说明探测名额可能泄漏） */
  halfOpenProbes?: number;
  /** 熔断器当前状态，用于判断探测名额是否「卡在」半开 */
  circuitState?: string | null;
  className?: string;
}

/**
 * 供应商健康状态徽章
 *
 * 优先级（高 → 低）：限流冷却 > 熔断（含倒计时）> 降级 > 正常。
 * 冷却排在最前是因为它解释的是「现在为什么变慢」，而不是「这个站点坏没坏」——
 * 一个冷却中的站点健康状态仍然是正常的。
 */
export function ProviderHealthBadge({
  consecutiveFailures,
  isHealthy,
  lastError,
  recoveryRemainingSecs = 0,
  cooldownSecs = 0,
  fatal = false,
  halfOpenProbes = 0,
  circuitState,
  className,
}: ProviderHealthBadgeProps) {
  const { t } = useTranslation();

  /**
   * 半开探测名额泄漏：长期停在 half_open 却已没有可用名额。
   *
   * 半开只发放 1 个名额（`allow_half_open_probe` 的 max=1）。正常流程里
   * 请求结束会把名额还回去，所以稳态下 half_open_probes 应为 0。若它持续
   * 停在 1（或更多）且熔断器仍是 half_open，说明某条请求路径漏了释放 ——
   * 该供应商会永久拒绝所有请求，而倒计时显示 0 秒。
   *
   * 这是需要用户手动「重置熔断」才能自愈的异常态，所以优先级最高：
   * 它比普通熔断更严重，且无法自动恢复。
   */
  const isProbeLeaked = circuitState === "half_open" && halfOpenProbes > 0;
  if (isProbeLeaked) {
    return (
      <div
        className={cn(
          "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium",
          "bg-red-500/15 text-red-700 dark:text-red-400",
          className,
        )}
        title={t("health.halfOpenProbeLeakedHint", {
          defaultValue:
            "半开探测名额未释放：该供应商目前会拒绝所有请求，且无法自动恢复。请手动「重置熔断」。",
        })}
      >
        <div className="w-2 h-2 rounded-full bg-red-500 animate-pulse" />
        <span>
          {t("health.halfOpenProbeLeaked", { defaultValue: "探测卡住" })}
        </span>
      </div>
    );
  }

  // 限流冷却：独立于健康度，说明「上游让我们等」
  if (cooldownSecs > 0) {
    const cooldownLabel = t("health.rateLimitedCooldown", {
      seconds: cooldownSecs,
      defaultValue: `限流 ${cooldownSecs}s`,
    });
    return (
      <div
        className={cn(
          "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium",
          "bg-amber-500/10 text-amber-600 dark:text-amber-400",
          className,
        )}
        title={t("health.rateLimitedCooldownHint", {
          defaultValue:
            "上游返回 429 并要求等待。该供应商仍会被尝试，但在路由选择时排到队列末尾。",
        })}
      >
        <div className="w-2 h-2 rounded-full bg-amber-500 animate-pulse" />
        <span className="tabular-nums">{cooldownLabel}</span>
      </div>
    );
  }

  // 根据失败次数计算状态
  const getStatus = () => {
    if (consecutiveFailures === 0) {
      return {
        labelKey: "health.operational",
        labelFallback: "正常",
        status: ProviderHealthStatus.Healthy,
        color: "bg-green-500",
        // 使用更深/柔和的背景色，去除可能的白色内容感
        bgColor: "bg-green-500/10",
        textColor: "text-green-600 dark:text-green-400",
      };
    } else if (isHealthy !== false) {
      return {
        labelKey: "health.degraded",
        labelFallback: "降级",
        status: ProviderHealthStatus.Degraded,
        color: "bg-yellow-500",
        bgColor: "bg-yellow-500/10",
        textColor: "text-yellow-600 dark:text-yellow-400",
      };
    } else {
      return {
        labelKey: "health.circuitOpen",
        labelFallback: "熔断",
        status: ProviderHealthStatus.Failed,
        color: "bg-red-500",
        bgColor: "bg-red-500/10",
        textColor: "text-red-600 dark:text-red-400",
      };
    }
  };

  const statusConfig = getStatus();
  // 熔断且还有恢复倒计时时，把剩余秒数并进标签，避免用户以为要手动干预
  const baseLabel = t(statusConfig.labelKey, {
    defaultValue: statusConfig.labelFallback,
  });
  const label =
    statusConfig.status === ProviderHealthStatus.Failed &&
    recoveryRemainingSecs > 0
      ? t("health.circuitOpenCountdown", {
          label: baseLabel,
          seconds: recoveryRemainingSecs,
          defaultValue: `${baseLabel} ${recoveryRemainingSecs}s`,
        })
      : baseLabel;

  const tooltipLines = [
    t("health.consecutiveFailures", {
      count: consecutiveFailures,
      defaultValue: `连续失败 ${consecutiveFailures} 次`,
    }),
    fatal
      ? t("health.fatalCircuitHint", {
          defaultValue: "由额度耗尽触发：恢复窗口内直接快速失败，不再打上游",
        })
      : null,
    recoveryRemainingSecs > 0
      ? t("health.recoveryCountdownHint", {
          seconds: recoveryRemainingSecs,
          defaultValue: `约 ${recoveryRemainingSecs} 秒后自动半开探测`,
        })
      : null,
    lastError?.trim() || null,
  ]
    .filter(Boolean)
    .join("\n");

  return (
    <div
      className={cn(
        "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium",
        statusConfig.bgColor,
        statusConfig.textColor,
        className,
      )}
      title={tooltipLines}
    >
      <div className={cn("w-2 h-2 rounded-full", statusConfig.color)} />
      <span className="tabular-nums">{label}</span>
    </div>
  );
}
