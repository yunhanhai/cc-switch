import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ProviderActions } from "@/components/providers/ProviderActions";

describe("ProviderActions Claude → Codex conversion button", () => {
  const baseProps = {
    isCurrent: false,
    onSwitch: vi.fn(),
    onEdit: vi.fn(),
    onDelete: vi.fn(),
    onDuplicate: vi.fn(),
  };

  it("renders the convert action for Claude providers when wired", () => {
    const onConvertToCodex = vi.fn();
    render(
      <ProviderActions
        {...baseProps}
        appId="claude"
        onConvertToCodex={onConvertToCodex}
      />,
    );

    expect(screen.getByTitle("provider.convertToCodex")).toBeInTheDocument();
  });

  it("invokes the conversion callback on click", async () => {
    const user = userEvent.setup();
    const onConvertToCodex = vi.fn();
    render(
      <ProviderActions
        {...baseProps}
        appId="claude"
        onConvertToCodex={onConvertToCodex}
      />,
    );

    await user.click(screen.getByTitle("provider.convertToCodex"));

    expect(onConvertToCodex).toHaveBeenCalledTimes(1);
  });

  it("renders the convert action for any app when the caller wires it (app gating is the caller's job)", () => {
    render(
      <ProviderActions
        {...baseProps}
        appId="codex"
        onConvertToCodex={vi.fn()}
      />,
    );

    expect(screen.getByTitle("provider.convertToCodex")).toBeInTheDocument();
  });

  it("omits the convert action for Claude when the caller does not wire it", () => {
    render(<ProviderActions {...baseProps} appId="claude" />);

    expect(
      screen.queryByTitle("provider.convertToCodex"),
    ).not.toBeInTheDocument();
  });
});
