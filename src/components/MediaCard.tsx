import {
  CheckIcon as Check,
  CheckSquareIcon,
  HeartIcon as Heart,
  InfoIcon,
  PlayIcon as Play,
  SquareIcon,
} from "@phosphor-icons/react";
import type { MediaItem } from "../types";
import { Artwork } from "./Artwork";

interface MediaCardProps {
  item: MediaItem;
  onPlay: (item: MediaItem) => void;
  onFavorite: (item: MediaItem) => void;
  selectionMode?: boolean;
  selected?: boolean;
  onToggleSelection?: (item: MediaItem) => void;
  onDetails?: (item: MediaItem) => void;
}

function formatDuration(seconds: number) {
  const totalMinutes = Math.max(0, Math.round(seconds / 60));
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return hours ? `${hours}h ${minutes}m` : `${minutes}m`;
}

export function MediaCard({
  item,
  onPlay,
  onFavorite,
  selectionMode = false,
  selected = false,
  onToggleSelection,
  onDetails,
}: MediaCardProps) {
  const progress = item.durationSecs > 0 ? Math.min(100, (item.progressSecs / item.durationSecs) * 100) : 0;
  return (
    <article className={selected ? "media-card selected" : "media-card"}>
      <button
        className="media-open"
        onClick={() => selectionMode ? onToggleSelection?.(item) : onPlay(item)}
        aria-label={selectionMode ? `${selected ? "Deselect" : "Select"} ${item.title}` : `Play ${item.title}`}
        aria-pressed={selectionMode ? selected : undefined}
      >
        <Artwork path={item.artworkPath} title={item.title} />
        <span className="card-play"><Play size={20} weight="fill" /></span>
        {selectionMode ? <span className="card-selection">{selected ? <CheckSquareIcon weight="fill" /> : <SquareIcon />}</span> : null}
        {item.managedByHomeTube ? <span className="managed-badge">HomeTube</span> : null}
        {item.watched ? <span className="watched-badge"><Check size={12} /> Watched</span> : null}
        {progress > 0 ? <span className="card-progress"><span style={{ width: `${progress}%` }} /></span> : null}
      </button>
      <div className="card-copy">
        <div>
          <h3 title={item.title}>{item.title}</h3>
          <p><span>{item.creator}</span><span aria-hidden="true">·</span><span>{formatDuration(item.durationSecs)}</span></p>
        </div>
        <div className="card-actions">
          {onDetails ? <button className="favorite-button" onClick={() => onDetails(item)} aria-label={`Details for ${item.title}`}><InfoIcon size={18} /></button> : null}
          <button
            className={item.favorite ? "favorite-button active" : "favorite-button"}
            onClick={() => onFavorite(item)}
            aria-label={item.favorite ? `Remove ${item.title} from favorites` : `Add ${item.title} to favorites`}
          >
            <Heart size={17} weight={item.favorite ? "fill" : "regular"} />
          </button>
        </div>
      </div>
    </article>
  );
}
