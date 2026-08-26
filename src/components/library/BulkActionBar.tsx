import {
  CheckCircleIcon,
  HeartIcon,
  TrashIcon,
  XIcon,
} from "@phosphor-icons/react";

interface BulkActionBarProps {
  selectedCount: number;
  managedCount: number;
  onFavorite: (favorite: boolean) => void;
  onWatched: (watched: boolean) => void;
  onRemove: () => void;
  onClear: () => void;
}

export function BulkActionBar({
  selectedCount,
  managedCount,
  onFavorite,
  onWatched,
  onRemove,
  onClear,
}: BulkActionBarProps) {
  if (!selectedCount) return null;
  return (
    <div className="bulk-action-bar" role="region" aria-label={`${selectedCount} selected videos`}>
      <strong>{selectedCount} selected</strong>
      <span>{managedCount} eligible for Trash</span>
      <div>
        <button onClick={() => onFavorite(true)}><HeartIcon /> Favorite</button>
        <button onClick={() => onFavorite(false)}><HeartIcon /> Unfavorite</button>
        <button onClick={() => onWatched(true)}><CheckCircleIcon /> Watched</button>
        <button className="danger" onClick={onRemove} disabled={!managedCount}><TrashIcon /> Move to Trash</button>
        <button className="icon-button" onClick={onClear} aria-label="Clear selection"><XIcon /></button>
      </div>
    </div>
  );
}
