import {
  CheckCircleIcon,
  HeartIcon,
  TrashIcon,
  XIcon,
} from "@phosphor-icons/react";

interface BulkActionBarProps {
  selectedCount: number;
  onFavorite: (favorite: boolean) => void;
  onWatched: (watched: boolean) => void;
  onRemove: () => void;
  onClear: () => void;
}

export function BulkActionBar({
  selectedCount,
  onFavorite,
  onWatched,
  onRemove,
  onClear,
}: BulkActionBarProps) {
  if (!selectedCount) return null;
  return (
    <div className="bulk-action-bar" role="region" aria-label={`${selectedCount} selected videos`}>
      <strong>{selectedCount} selected</strong>
      <div>
        <button onClick={() => onFavorite(true)}><HeartIcon /> Favorite</button>
        <button onClick={() => onFavorite(false)}><HeartIcon /> Unfavorite</button>
        <button onClick={() => onWatched(true)}><CheckCircleIcon /> Watched</button>
        <button className="danger" onClick={onRemove}><TrashIcon /> Move to Trash</button>
        <button className="icon-button" onClick={onClear} aria-label="Clear selection"><XIcon /></button>
      </div>
    </div>
  );
}
