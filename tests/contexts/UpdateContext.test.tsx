import React from "react";
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { UpdateProvider, useUpdate } from "@/contexts/UpdateContext";

const configuredMock = vi.hoisted(() => vi.fn());
const checkMock = vi.hoisted(() => vi.fn());
vi.mock("@/lib/updater", () => ({
  isAutoUpdateConfigured: configuredMock,
  checkForUpdate: checkMock,
}));

describe("UpdateProvider startup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });
  afterEach(() => vi.useRealTimers());
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <UpdateProvider>{children}</UpdateProvider>
  );

  it("skips automatic checks when the app has no configured update channel", async () => {
    configuredMock.mockResolvedValue(false);
    const { result } = renderHook(useUpdate, { wrapper });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(checkMock).not.toHaveBeenCalled();
    expect(result.current.autoUpdateEnabled).toBe(false);
    expect(result.current.hasUpdate).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("checks enabled channels and preserves the available update", async () => {
    configuredMock.mockResolvedValue(true);
    checkMock.mockResolvedValue({
      status: "available",
      info: { currentVersion: "1.0.0", availableVersion: "1.0.1" },
    });
    const { result } = renderHook(useUpdate, { wrapper });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(checkMock).toHaveBeenCalledTimes(1);
    expect(result.current.autoUpdateEnabled).toBe(true);
    expect(result.current.updateInfo?.availableVersion).toBe("1.0.1");
  });
});
