import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ClaudeDesktopRouteToggle } from "@/components/proxy/ClaudeDesktopRouteToggle";

const { useProxyStatusMock, useGlobalProxyConfigMock } = vi.hoisted(() => ({
  useProxyStatusMock: vi.fn(),
  useGlobalProxyConfigMock: vi.fn(),
}));

vi.mock("@/hooks/useProxyStatus", () => ({
  useProxyStatus: useProxyStatusMock,
}));
vi.mock("@/lib/query/proxy", () => ({
  useGlobalProxyConfig: useGlobalProxyConfigMock,
}));

describe("ClaudeDesktopRouteToggle endpoint", () => {
  beforeEach(() => {
    useProxyStatusMock.mockReturnValue({
      isRunning: false,
      status: undefined,
      takeoverStatus: {},
      startProxyServer: vi.fn(),
      stopProxyServer: vi.fn(),
      isStarting: false,
      isStoppingServer: false,
    });
    useGlobalProxyConfigMock.mockReturnValue({ data: undefined });
  });

  it("does not invent an endpoint while configuration is loading", () => {
    const { container } = render(<ClaudeDesktopRouteToggle />);
    expect(container.firstElementChild?.getAttribute("title")).not.toMatch(
      /1572[12]/,
    );
    expect(screen.getByRole("switch")).toBeEnabled();
  });

  it("shows the saved endpoint when the server is stopped", () => {
    useGlobalProxyConfigMock.mockReturnValue({
      data: { listenAddress: "127.0.0.1", listenPort: 25001 },
    });
    const { container } = render(<ClaudeDesktopRouteToggle />);
    expect(container.firstElementChild?.getAttribute("title")).toContain(
      "127.0.0.1:25001",
    );
  });

  it("shows the actual running endpoint rather than the stored configuration", () => {
    useGlobalProxyConfigMock.mockReturnValue({
      data: { listenAddress: "127.0.0.1", listenPort: 15732 },
    });
    useProxyStatusMock.mockReturnValue({
      isRunning: true,
      status: { address: "127.0.0.1", port: 25002 },
      takeoverStatus: {},
      startProxyServer: vi.fn(),
      stopProxyServer: vi.fn(),
      isStarting: false,
      isStoppingServer: false,
    });
    const { container } = render(<ClaudeDesktopRouteToggle />);
    expect(container.firstElementChild?.getAttribute("title")).toContain(
      "127.0.0.1:25002",
    );
  });
});
