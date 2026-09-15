import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Sidebar } from "./Sidebar";

describe("Sidebar", () => {
  it("marks the active view and navigates", () => {
    const navigate = vi.fn();
    render(<Sidebar active="home" onNavigate={navigate} />);
    expect(screen.getByRole("button", { name: "Home" })).toHaveAttribute("aria-current", "page");
    fireEvent.click(screen.getByRole("button", { name: "Discover" }));
    expect(navigate).toHaveBeenCalledWith("discover");
  });

  it("keeps only content destinations in the primary nav", () => {
    render(<Sidebar active="home" onNavigate={() => undefined} />);
    expect(screen.queryByRole("button", { name: "Library" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Recently Removed" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Settings" })).not.toBeInTheDocument();
  });
});
