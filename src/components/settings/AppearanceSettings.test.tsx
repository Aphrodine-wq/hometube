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
  });
});
