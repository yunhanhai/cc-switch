import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RectifierConfigPanel } from "@/components/settings/RectifierConfigPanel";
import { settingsApi, type RectifierConfig } from "@/lib/api/settings";

vi.mock("@/lib/api/settings", () => ({
  settingsApi: {
    getRectifierConfig: vi.fn(),
    setRectifierConfig: vi.fn(),
    getOptimizerConfig: vi.fn(),
    setOptimizerConfig: vi.fn(),
  },
}));
vi.mock("sonner", () => ({ toast: { error: vi.fn() } }));

const saved: RectifierConfig = {
  enabled: true,
  requestThinkingSignature: false,
  requestThinkingBudget: true,
  requestMediaFallback: true,
  requestMediaHeuristic: false,
  codexResponsesCompatibility: false,
};

describe("Codex Responses compatibility settings", () => {
  beforeEach(() => {
    vi.mocked(settingsApi.getRectifierConfig).mockResolvedValue({ ...saved });
    vi.mocked(settingsApi.setRectifierConfig).mockResolvedValue(true);
    vi.mocked(settingsApi.getOptimizerConfig).mockResolvedValue({
      enabled: false,
      thinkingOptimizer: true,
      cacheInjection: true,
    });
  });

  const toggle = () =>
    screen.findByRole("switch", {
      name: "settings.advanced.rectifier.codexResponsesCompatibility",
    });

  it("is opt-in and saves both on and off without changing other rectifiers", async () => {
    const user = userEvent.setup();
    render(<RectifierConfigPanel />);
    const control = await toggle();
    expect(control).not.toBeChecked();
    await user.click(control);
    await waitFor(() =>
      expect(settingsApi.setRectifierConfig).toHaveBeenLastCalledWith({
        ...saved,
        codexResponsesCompatibility: true,
      }),
    );
    await user.click(control);
    await waitFor(() =>
      expect(settingsApi.setRectifierConfig).toHaveBeenLastCalledWith(saved),
    );
  });

  it("does not enable itself when an older settings record lacks the field", async () => {
    const { codexResponsesCompatibility: _unused, ...legacy } = saved;
    vi.mocked(settingsApi.getRectifierConfig).mockResolvedValue(
      legacy as RectifierConfig,
    );
    render(<RectifierConfigPanel />);
    expect(await toggle()).not.toBeChecked();
    expect(settingsApi.setRectifierConfig).not.toHaveBeenCalled();
  });

  it("respects the rectifier master switch", async () => {
    vi.mocked(settingsApi.getRectifierConfig).mockResolvedValue({
      ...saved,
      enabled: false,
    });
    render(<RectifierConfigPanel />);
    expect(await toggle()).toBeDisabled();
  });

  it("restores the saved value after persistence fails", async () => {
    vi.mocked(settingsApi.setRectifierConfig).mockRejectedValueOnce(
      new Error("save failed"),
    );
    render(<RectifierConfigPanel />);
    const control = await toggle();
    await userEvent.click(control);
    await waitFor(() => expect(control).not.toBeChecked());
  });
});
