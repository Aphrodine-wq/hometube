import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RemovalDialog } from "./RemovalDialog";

const preview = {
  token: "token",
  items: [{
    mediaId: "one",
    title: "First Light",
    creator: "Channel",
    path: "/tmp/one.mp4",
    sizeBytes: 1024,
    managedByHomeTube: true,
  }],
  totalBytes: 1024,
  eligibleCount: 1,
  excludedCount: 0,
  requiredPhrase: "MOVE ALL TO TRASH",
};

describe("RemovalDialog", () => {
  it("gates destructive confirmation behind the backend phrase", () => {
    const confirm = vi.fn();
    const change = vi.fn();
    const { rerender } = render(
      <RemovalDialog preview={preview} loading={false} phrase="" onPhraseChange={change} onOpenChange={vi.fn()} onConfirm={confirm} />,
    );
    expect(screen.getByRole("button", { name: "Move to system Trash" })).toBeDisabled();
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "MOVE ALL TO TRASH" } });
    expect(change).toHaveBeenCalledWith("MOVE ALL TO TRASH");
    rerender(
      <RemovalDialog preview={preview} loading={false} phrase="MOVE ALL TO TRASH" onPhraseChange={change} onOpenChange={vi.fn()} onConfirm={confirm} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Move to system Trash" }));
    expect(confirm).toHaveBeenCalled();
  });
});
