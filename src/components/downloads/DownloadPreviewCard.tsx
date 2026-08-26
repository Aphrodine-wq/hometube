import {
  ArrowClockwiseIcon,
  FilmSlateIcon,
  ListBulletsIcon,
  SpinnerGapIcon,
  WarningCircleIcon,
} from "@phosphor-icons/react";
import type { DownloadPreviewState } from "../../hooks/useDownloadPreview";
import type { DownloadQuality } from "../../types";

interface DownloadPreviewCardProps {
  state: DownloadPreviewState;
  quality: DownloadQuality;
  onRetry: () => void;
}

function duration(seconds: number | null) {
  if (!seconds) return null;
  const minutes = Math.round(seconds / 60);
  return minutes >= 60 ? `${Math.floor(minutes / 60)}h ${minutes % 60}m` : `${minutes}m`;
}

export function DownloadPreviewCard({ state, quality, onRetry }: DownloadPreviewCardProps) {
  const { status, preview, error } = state;
  if (status === "idle") {
    return (
      <div className="download-preview idle">
        <div className="preview-placeholder"><FilmSlateIcon weight="duotone" /></div>
        <div><strong>Paste a YouTube link</strong><span>A preview will appear here before anything is queued.</span></div>
      </div>
    );
  }

  if (status === "loading") {
    return (
      <div className="download-preview loading" aria-live="polite">
        <div className="preview-placeholder"><SpinnerGapIcon className="spin" /></div>
        <div><strong>Reading this link…</strong><span>Checking title, channel, artwork, and available quality.</span></div>
      </div>
    );
  }

  if (status === "error") {
    return (
      <div className="download-preview error" role="status">
        <div className="preview-placeholder"><WarningCircleIcon weight="duotone" /></div>
        <div><strong>Preview unavailable</strong><span>{error}</span></div>
        <button className="text-button" onClick={onRetry}><ArrowClockwiseIcon /> Retry preview</button>
      </div>
    );
  }

  const requestedQuality = quality === "best" ? "Best compatible" : quality;
  return (
    <article className="download-preview ready" aria-live="polite">
      <div className="preview-art">
        {preview.thumbnailUrl ? <img src={preview.thumbnailUrl} alt="" /> : <FilmSlateIcon weight="duotone" />}
        <span>{preview.kind === "playlist" ? <ListBulletsIcon /> : <FilmSlateIcon />}{preview.kind}</span>
      </div>
      <div className="preview-copy">
        <span className="eyebrow">Ready to add</span>
        <h2>{preview.title}</h2>
        <p>{preview.channel || "Unknown channel"}</p>
        <div className="preview-facts">
          {preview.itemCount ? <span>{preview.itemCount.toLocaleString()} videos</span> : null}
          {duration(preview.durationSecs) ? <span>{duration(preview.durationSecs)}</span> : null}
          <span>{requestedQuality}</span>
          {preview.sourceMaxHeight ? <span>Source up to {preview.sourceMaxHeight}p</span> : null}
        </div>
      </div>
    </article>
  );
}
