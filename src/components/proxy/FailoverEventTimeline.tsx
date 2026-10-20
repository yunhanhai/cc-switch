/**
 * 故障转移事件时间线（定制）
 *
 * 展示最近的自动切换记录：从谁 → 到谁、原因（额度耗尽/限流/故障）、时间。
 * 数据来自后端 failover_events 表，15 秒自动刷新。
 */

import { useTranslation } from "react-i18next";
import { History, ArrowRight } from "lucide-react";
import { useFailoverEvents } from "@/lib/query/failover";
import type { ProxyAppId } from "@/config/appConfig";

interface FailoverEventTimelineProps {
  appType: ProxyAppId;
}

const REASON_STYLES: Record<string, string> = {
  billing_exhausted: "bg-red-500/15 text-red-700 dark:text-red-400",
  rate_limited: "bg-amber-500/15 text-amber-700 dark:text-amber-400",
  provider_failed: "bg-orange-500/15 text-orange-700 dark:text-orange-400",
};

export function FailoverEventTimeline({ appType }: FailoverEventTimelineProps) {
  const { t } = useTranslation();
  const { data: events = [], isLoading } = useFailoverEvents(appType, 15);

  const formatTime = (ts: number) => {
    const date = new Date(ts * 1000);
    const now = new Date();
    const sameDay = date.toDateString() === now.toDateString();
    const time = date.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });
    return sameDay ? time : `${date.toLocaleDateString()} ${time}`;
  };

  const reasonLabel = (reason?: string | null) => {
    if (!reason) return null;
    if (reason === "billing_exhausted")
      return t("proxy.failoverEvents.reasonBilling", {
        defaultValue: "额度耗尽",
      });
    if (reason === "rate_limited")
      return t("proxy.failoverEvents.reasonRateLimit", {
        defaultValue: "限流",
      });
    if (reason === "provider_failed")
      return t("proxy.failoverEvents.reasonFailed", {
        defaultValue: "故障",
      });
    return reason;
  };

  return (
    <div className="rounded-lg border border-border/50 bg-card/40 p-4">
      <div className="mb-3 flex items-center gap-2 text-sm font-medium">
        <History className="h-4 w-4 text-muted-foreground" />
        {t("proxy.failoverEvents.title", { defaultValue: "转移事件" })}
      </div>

      {isLoading ? (
        <div className="h-16 animate-pulse rounded bg-muted" />
      ) : events.length === 0 ? (
        <p className="text-xs text-muted-foreground">
          {t("proxy.failoverEvents.empty", {
            defaultValue: "暂无自动切换记录",
          })}
        </p>
      ) : (
        <ul className="space-y-2">
          {events.map((event) => (
            <li
              key={event.id}
              className="flex flex-wrap items-center gap-x-2 gap-y-1 rounded-md border border-border/40 bg-background/60 px-3 py-2 text-xs"
            >
              <span className="text-muted-foreground tabular-nums">
                {formatTime(event.createdAt)}
              </span>
              <span className="font-medium">
                {event.fromProviderName ??
                  event.fromProviderId ??
                  t("proxy.failoverEvents.unknownSource", {
                    defaultValue: "未知",
                  })}
              </span>
              <ArrowRight className="h-3 w-3 text-muted-foreground" />
              <span className="font-medium text-primary">
                {event.toProviderName}
              </span>
              {event.reason && (
                <span
                  className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${
                    REASON_STYLES[event.reason] ??
                    "bg-muted text-muted-foreground"
                  }`}
                >
                  {reasonLabel(event.reason)}
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
