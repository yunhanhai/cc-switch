import { QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ProviderCard } from "@/components/providers/ProviderCard";
import type { Provider } from "@/types";
import { createTestQueryClient } from "../utils/testQueryClient";

const providerActionsProps = vi.hoisted(() => vi.fn());

vi.mock("@/components/providers/ProviderActions", () => ({
  ProviderActions: (props: unknown) => {
    providerActionsProps(props);
    return null;
  },
}));

vi.mock("@/components/ProviderIcon", () => ({ ProviderIcon: () => null }));
vi.mock("@/components/UsageFooter", () => ({ default: () => null }));
vi.mock("@/components/SubscriptionQuotaFooter", () => ({
  default: () => null,
}));
vi.mock("@/components/CopilotQuotaFooter", () => ({ default: () => null }));
vi.mock("@/components/CodexOauthQuotaFooter", () => ({ default: () => null }));
vi.mock("@/components/XaiOauthQuotaFooter", () => ({ default: () => null }));

vi.mock("@/lib/query/failover", () => ({
  useProviderHealth: () => ({ data: undefined }),
  useResetCircuitBreaker: () => ({ mutate: () => {}, isPending: false }),
}));

vi.mock("@/lib/query/queries", () => ({
  useUsageQuery: () => ({ data: undefined }),
}));

const thirdPartyClaudeProvider = (): Provider => ({
  id: "kimi-claude",
  name: "Kimi",
  settingsConfig: {
    env: {
      ANTHROPIC_BASE_URL: "https://api.kimi.com/anthropic",
      ANTHROPIC_AUTH_TOKEN: "sk-kimi",
    },
  },
});

const officialClaudeProvider = (): Provider => ({
  id: "anthropic-official",
  name: "Claude Official",
  category: "official",
  settingsConfig: { env: { ANTHROPIC_AUTH_TOKEN: "" } },
});

function renderCard(provider: Provider) {
  providerActionsProps.mockClear();
  const queryClient = createTestQueryClient();
  render(
    <QueryClientProvider client={queryClient}>
      <ProviderCard
        provider={provider}
        appId="claude"
        isCurrent={false}
        isProxyRunning={false}
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onConfigureUsage={vi.fn()}
        onOpenWebsite={vi.fn()}
        onDuplicate={vi.fn()}
        onConvertToCodex={vi.fn()}
      />
    </QueryClientProvider>,
  );

  const lastCall = providerActionsProps.mock.calls.at(-1)?.[0] as
    | { onConvertToCodex?: () => void }
    | undefined;
  return lastCall ?? {};
}

describe("ProviderCard Claude → Codex conversion gating", () => {
  it("wires the convert action for a third-party Claude provider", () => {
    const props = renderCard(thirdPartyClaudeProvider());

    expect(props.onConvertToCodex).toBeTypeOf("function");
  });

  it("hides the convert action for the official Claude provider", () => {
    const props = renderCard(officialClaudeProvider());

    expect(props.onConvertToCodex).toBeUndefined();
  });
});
