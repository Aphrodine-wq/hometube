import { HardDriveIcon, TrashIcon, WarningIcon } from "@phosphor-icons/react";
import type { RemovalPreview } from "../../types";
import { ConfirmDialog } from "../ui/ConfirmDialog";

interface RemovalDialogProps {
  preview: RemovalPreview | null;
  loading: boolean;
  phrase: string;
  onPhraseChange: (phrase: string) => void;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}

function formatBytes(bytes: number) {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unit = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** unit).toFixed(unit > 2 ? 1 : 0)} ${units[unit]}`;
}

export function RemovalDialog({
  preview,
  loading,
  phrase,
  onPhraseChange,
  onOpenChange,
  onConfirm,
}: RemovalDialogProps) {
  return (
    <ConfirmDialog
      open={Boolean(preview)}
      onOpenChange={onOpenChange}
      title={`Move ${preview?.eligibleCount || 0} video${preview?.eligibleCount === 1 ? "" : "s"} to Trash?`}
      description="Files will move to your system Trash, not disappear permanently. Empty Trash later if you need to reclaim the disk space."
      confirmLabel="Move to system Trash"
      destructive
      busy={loading}
      requiredPhrase={preview?.requiredPhrase === "MOVE ALL TO TRASH" ? preview.requiredPhrase : null}
      phraseValue={phrase}
      onPhraseChange={onPhraseChange}
      onConfirm={onConfirm}
    >
      {preview ? (
        <div className="removal-summary">
          <div><TrashIcon /><span><strong>{preview.eligibleCount}</strong> verified HomeTube downloads</span></div>
          <div><HardDriveIcon /><span><strong>{formatBytes(preview.totalBytes)}</strong> moved to Trash</span></div>
          {preview.excludedCount ? <div className="warning"><WarningIcon /><span>{preview.excludedCount} unverified local files excluded for safety</span></div> : null}
          <ul>
            {preview.items.slice(0, 5).map((item) => <li key={item.mediaId}><span>{item.title}</span><small>{formatBytes(item.sizeBytes)}</small></li>)}
            {preview.items.length > 5 ? <li><span>and {preview.items.length - 5} more…</span></li> : null}
          </ul>
        </div>
      ) : null}
    </ConfirmDialog>
  );
}
