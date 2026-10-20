import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const PROVIDER_CARD_TSX = path.resolve(
  __dirname,
  "..",
  "..",
  "src",
  "components",
  "providers",
  "ProviderCard.tsx",
);
const PROVIDER_LIST_TSX = path.resolve(
  __dirname,
  "..",
  "..",
  "src",
  "components",
  "providers",
  "ProviderList.tsx",
);

describe("ProviderCard layout", () => {
  const source = fs.readFileSync(PROVIDER_CARD_TSX, "utf8");

  it("lets website links use available card width before truncating", () => {
    expect(source).not.toContain("max-w-[280px]");
    expect(source).toContain("flex min-w-0 flex-1 items-center gap-2");
    expect(source).toContain("min-w-0 flex-1 space-y-1");
    expect(source).toContain(
      "inline-flex max-w-full items-center overflow-hidden text-left text-sm",
    );
  });

  it("does not add a Pi-only model list to provider cards", () => {
    expect(source).not.toContain("ProviderModelSummary");
    expect(source).not.toContain("extractPiModelSummaryItems");
  });

  // 健康轮询的开关必须来自"代理进程是否在运行"，而不是 ProviderList 里
  // 那个被 `supportsFailover &&` 短路过的 isProxyRunning —— 后者对不支持
  // 故障转移的供应商恒为 false，会静默停掉这些卡片的健康刷新。
  it("drives health polling from the proxy process state, not the failover-gated flag", () => {
    expect(source).toContain("isProxyServiceRunning");
    expect(source).toContain("isProxyServiceRunning ?? true");
    // 不允许把 isProxyRunning 直接当作轮询开关
    expect(source).not.toMatch(
      /useProviderHealth\([\s\S]{0,200}isProxyRunning,/,
    );
  });

  it("keeps Pi provider cards independent from routing capability state", () => {
    const listSource = fs.readFileSync(PROVIDER_LIST_TSX, "utf8");

    expect(source).not.toContain("piCurrentRoute");
    expect(source).not.toContain("isFailoverEligible");
    expect(listSource).not.toContain("gatewayStatus");
    expect(listSource).not.toContain("activeRoute");
    expect(listSource).not.toContain("pi.gatewayReason");
    expect(listSource).not.toContain("pi.route");
  });
});
