import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AppearanceSettings } from "./AppearanceSettings";

const value = {
  themePreference: "system" as const,
  accentPreference: "cinema" as const,
  densityPreference: "comfortable" as const,
  textSizePreference: "standard" as const,
  reducedMotion: false,
};

describe("AppearanceSettings", () => {
  it("emits immediately previewable preferences", () => {
    const change = vi.fn();
    render(<AppearanceSettings value={value} onChange={change} saving={false} />);
    fireEvent.click(screen.getByRole("radio", { name: "Light" }));
    expect(change).toHaveBeenCalledWith({ ...value, themePreference: "light" });
    fireEvent.click(screen.getByRole("radio", { name: "Deep teal" }));
    expect(change).toHaveBeenCalledWith({ ...value, accentPreference: "teal" });
    fireEvent.click(screen.getByRole("radio", { name: "Compact" }));
    expect(change).toHaveBeenCalledWith({ ...value, densityPreference: "compact" });
    fireEvent.click(screen.getByRole("radio", { name: "Large" }));
    expect(change).toHaveBeenCalledWith({ ...value, textSizePreference: "large" });
    fireEvent.click(screen.getByRole("switch", { name: /Reduce motion/ }));
    expect(change).toHaveBeenCalledWith({ ...value, reducedMotion: true });
  });

  it("resets to defaults and disables the reset button when already default", () => {
    const change = vi.fn();
    const { rerender } = render(
      <AppearanceSettings value={{ ...value, accentPreference: "violet" }} onChange={change} saving={false} />,
    );
    const reset = screen.getByRole("button", { name: /Reset to defaults/ });
    expect(reset).toBeEnabled();
    fireEvent.click(reset);
    expect(change).toHaveBeenCalledWith(value);
    rerender(<AppearanceSettings value={value} onChange={change} saving={false} />);
    expect(screen.getByRole("button", { name: /Reset to defaults/ })).toBeDisabled();
  });

  it("shows a transient saved state and an inline error", () => {
    const change = vi.fn();
    const { rerender } = render(<AppearanceSettings value={value} onChange={change} saving />);
    expect(screen.getByRole("status")).toHaveTextContent("Saving…");
    rerender(<AppearanceSettings value={value} onChange={change} saving={false} />);
    expect(screen.getByRole("status")).toHaveTextContent("Saved");
    rerender(<AppearanceSettings value={value} onChange={change} saving={false} error="Disk on fire" />);
    expect(screen.getByRole("alert")).toHaveTextContent("Disk on fire");
    expect(screen.getByRole("status")).toHaveTextContent("Couldn't save");
  });
});
