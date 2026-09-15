import {
  CheckCircleIcon,
  FolderOpenIcon,
  HeartIcon,
  PlayIcon,
  TrashIcon,
} from "@phosphor-icons/react";
import type { MediaItem } from "../../types";
import { Artwork } from "../Artwork";
import { Sheet } from "../ui/Sheet";

interface VideoDetailsPanelProps {
  item: MediaItem | null;
  onOpenChange: (open: boolean) => void;
  onPlay: (item: MediaItem) => void;
  onFavorite: (item: MediaItem) => void;
  onWatched: (item: MediaItem, watched: boolean) => void;
  onOpenFolder: (item: MediaItem) => void;
  onRemove: (item: MediaItem) => void;
}

function formatDuration(seconds: number) {
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  return hours ? `${hours}h ${minutes}m` : `${minutes}m`;
}

function formatBytes(bytes: number) {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unit = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** unit).toFixed(unit > 2 ? 1 : 0)} ${units[unit]}`;
}

export function VideoDetailsPanel({
  item,
  onOpenChange,
  onPlay,
  onFavorite,
  onWatched,
  onOpenFolder,
  onRemove,
}: VideoDetailsPanelProps) {
  return (
    <Sheet
      open={Boolean(item)}
      onOpenChange={onOpenChange}
      title={item?.title || "Video details"}
      description={item?.creator}
      footer={item ? (
        <div className="details-footer">
          <button className="primary-button" onClick={() => onPlay(item)}><PlayIcon weight="fill" />{item.progressSecs > 0 ? "Resume" : "Play"}</button>
          <button className="danger-button" onClick={() => onRemove(item)}><TrashIcon /> Move to Trash</button>
        </div>
      ) : null}
    >
      {item ? (
        <div className="video-details">
          <Artwork path={item.artworkPath} title={item.title} className="details-artwork" />
          {item.description ? <p className="details-description">{item.description}</p> : null}
          <div className="details-actions">
            <button onClick={() => onFavorite(item)}><HeartIcon weight={item.favorite ? "fill" : "regular"} />{item.favorite ? "Favorited" : "Favorite"}</button>
            <button onClick={() => onWatched(item, !item.watched)}><CheckCircleIcon weight={item.watched ? "fill" : "regular"} />{item.watched ? "Mark unwatched" : "Mark watched"}</button>
            <button onClick={() => onOpenFolder(item)}><FolderOpenIcon /> Open folder</button>
          </div>
          <dl className="details-grid">
            <div><dt>Duration</dt><dd>{formatDuration(item.durationSecs)}</dd></div>
            <div><dt>File size</dt><dd>{formatBytes(item.sizeBytes)}</dd></div>
            <div><dt>Format</dt><dd>{item.container.toUpperCase()}</dd></div>
            <div><dt>Video</dt><dd>{item.videoCodec || "Unknown"}</dd></div>
            <div><dt>Audio</dt><dd>{item.audioCodec || "None"}</dd></div>
            <div><dt>Added</dt><dd>{new Date(item.addedAt * 1000).toLocaleDateString()}</dd></div>
          </dl>
          <div className="details-path">
            <span>Local file</span>
            <code>{item.path}</code>
          </div>
          {item.sourceUrl ? <div className="details-path"><span>Source</span><code>{item.sourceUrl}</code></div> : null}
          <div className={item.managedByHomeTube ? "managed-note verified" : "managed-note"}>
            {item.managedByHomeTube
              ? "Verified HomeTube download · eligible for safe bulk Trash actions"
              : "Local library file · HomeTube will not include it in Delete All"}
          </div>
        </div>
      ) : null}
    </Sheet>
  );
}
