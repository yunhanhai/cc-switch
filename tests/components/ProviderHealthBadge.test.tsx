import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ProviderHealthBadge } from "@/components/providers/ProviderHealthBadge";

// 保留 key 与插值，便于断言渲染出的是哪一条文案、带的是哪个数值。
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (!params) return key;
      const rendered = Object.entries(params)
        .filter(([name]) => name !== "defaultValue")
        .map(([name, value]) => `${name}=${String(value)}`)
        .join(",");
      return rendered ? `${key}(${rendered})` : key;
    },
  }),
}));

describe("ProviderHealthBadge", () => {
  it("shows the rate-limit cooldown before any health state", () => {
    render(
      <ProviderHealthBadge
        consecutiveFailures={0}
        isHealthy
        cooldownSecs={42}
      />,
    );

    // 冷却优先：健康站点被限流时也要显示冷却，而不是「正常」
    expect(
      screen.getByText("health.rateLimitedCooldown(seconds=42)"),
    ).toBeInTheDocument();
    expect(screen.queryByText("health.operational")).not.toBeInTheDocument();
  });

  it("appends the recovery countdown to an open circuit", () => {
    render(
      <ProviderHealthBadge
        consecutiveFailures={5}
        isHealthy={false}
        recoveryRemainingSecs={37}
      />,
    );

    expect(
      screen.getByText(
        "health.circuitOpenCountdown(label=health.circuitOpen,seconds=37)",
      ),
    ).toBeInTheDocument();
  });

  it("falls back to the plain circuit label once the window expired", () => {
    render(<ProviderHealthBadge consecutiveFailures={5} isHealthy={false} />);

    expect(screen.getByText("health.circuitOpen")).toBeInTheDocument();
    expect(
      screen.queryByText(/health\.circuitOpenCountdown/),
    ).not.toBeInTheDocument();
  });

  it("keeps the degraded state when failures exist but health is still true", () => {
    render(<ProviderHealthBadge consecutiveFailures={2} isHealthy />);

    expect(screen.getByText("health.degraded")).toBeInTheDocument();
  });

  it("explains a fatal (billing) circuit in the tooltip", () => {
    const { container } = render(
      <ProviderHealthBadge
        consecutiveFailures={4}
        isHealthy={false}
        recoveryRemainingSecs={60}
        fatal
      />,
    );

    const badge = container.firstElementChild as HTMLElement;
    expect(badge.title).toContain("health.fatalCircuitHint");
    expect(badge.title).toContain("health.recoveryCountdownHint(seconds=60)");
  });
});
