import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Titlebar } from "./Titlebar";

describe("Titlebar", () => {
  it("cycles the theme from the quick toggle", () => {
    const cycle = vi.fn();
    render(<Titlebar query="" onQuery={() => undefined} scanning={false} theme="system" onCycleTheme={cycle} />);
    const toggle = screen.getByRole("button", { name: /System theme\. Switch to dark theme/ });
    fireEvent.click(toggle);
    expect(cycle).toHaveBeenCalledTimes(1);
  });

  it("labels the toggle with the current and next theme", () => {
    render(<Titlebar query="" onQuery={() => undefined} scanning={false} theme="light" onCycleTheme={() => undefined} />);
    expect(screen.getByRole("button", { name: /Light theme\. Switch to system theme/ })).toBeInTheDocument();
  });
});
