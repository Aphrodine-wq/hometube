import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Titlebar } from "./Titlebar";

const baseProps = { query: "", onQuery: () => undefined, scanning: false };

describe("Titlebar", () => {
  it("cycles the theme from the quick toggle", () => {
    const cycle = vi.fn();
    render(<Titlebar {...baseProps} theme="system" onCycleTheme={cycle} activeView="home" onNavigate={() => undefined} />);
    const toggle = screen.getByRole("button", { name: /System theme\. Switch to dark theme/ });
    fireEvent.click(toggle);
    expect(cycle).toHaveBeenCalledTimes(1);
  });

  it("labels the toggle with the current and next theme", () => {
    render(<Titlebar {...baseProps} theme="light" onCycleTheme={() => undefined} activeView="home" onNavigate={() => undefined} />);
    expect(screen.getByRole("button", { name: /Light theme\. Switch to system theme/ })).toBeInTheDocument();
  });

  it("hosts the utility views and highlights the active one", () => {
    const navigate = vi.fn();
    render(<Titlebar {...baseProps} theme="dark" onCycleTheme={() => undefined} activeView="settings" onNavigate={navigate} />);
    expect(screen.getByRole("button", { name: "Library" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Recently removed" })).not.toHaveAttribute("aria-current");
    const settings = screen.getByRole("button", { name: "Settings" });
    expect(settings).toHaveAttribute("aria-current", "page");
    fireEvent.click(screen.getByRole("button", { name: "Library" }));
    expect(navigate).toHaveBeenCalledWith("library");
  });
});
