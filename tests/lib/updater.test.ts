import { beforeEach, describe, expect, it, vi } from "vitest";
import { checkForUpdate } from "@/lib/updater";

const invokeMock = vi.hoisted(() => vi.fn());
const checkMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/app", () => ({
  getVersion: async () => "3.20.1-yunswitch.1",
}));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: checkMock }));

describe("independent updater configuration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("does not contact an updater or report current when its channel is disabled", async () => {
    invokeMock.mockResolvedValue(false);
    expect(await checkForUpdate()).toEqual({ status: "disabled" });
    expect(invokeMock).toHaveBeenCalledWith("is_app_updater_configured");
    expect(checkMock).not.toHaveBeenCalled();
  });

  it("uses the signed updater after the running app enables its channel", async () => {
    invokeMock.mockResolvedValue(true);
    checkMock.mockResolvedValue({
      version: "3.20.1-yunswitch.2",
      notes: "Changes",
    });
    expect(await checkForUpdate({ timeout: 1234 })).toEqual({
      status: "available",
      info: {
        currentVersion: "3.20.1-yunswitch.1",
        availableVersion: "3.20.1-yunswitch.2",
        notes: "Changes",
        pubDate: undefined,
      },
    });
    expect(checkMock).toHaveBeenCalledWith({ timeout: 1234 });
  });
});
