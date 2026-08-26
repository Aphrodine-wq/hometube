import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Sidebar } from "./Sidebar";

describe("Sidebar", () => {
  it("marks the active view and navigates", () => {
    const navigate = vi.fn();
    render(<Sidebar active="home" onNavigate={navigate} count={12} />);
    expect(screen.getByRole("button", { name: "Home" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByText("12")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Discover" }));
    expect(navigate).toHaveBeenCalledWith("discover");
    fireEvent.click(screen.getByRole("button", { name: "Recently Removed" }));
    expect(navigate).toHaveBeenCalledWith("removed");
  });
});
