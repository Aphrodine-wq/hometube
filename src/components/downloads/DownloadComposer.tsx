import { DownloadSimpleIcon, SpinnerGapIcon } from "@phosphor-icons/react";
import type { FormEvent } from "react";
import type { DownloadQuality } from "../../types";

interface DownloadComposerProps {
  url: string;
  onUrlChange: (url: string) => void;
  quality: DownloadQuality;
  onQualityChange: (quality: DownloadQuality) => void;
  onSubmit: (event: FormEvent) => void;
  submitting: boolean;
}

export function DownloadComposer({
  url,
  onUrlChange,
  quality,
  onQualityChange,
  onSubmit,
  submitting,
}: DownloadComposerProps) {
  return (
    <form className="download-composer" onSubmit={onSubmit}>
      <label className="download-url">
        <span>YouTube video or playlist</span>
        <input
          type="url"
          value={url}
          onChange={(event) => onUrlChange(event.target.value)}
          placeholder="Paste a youtube.com or youtu.be link"
          autoComplete="off"
          spellCheck={false}
          aria-label="YouTube URL"
          aria-describedby="download-help"
        />
      </label>
      <label className="download-quality">
        <span>Download quality</span>
        <select aria-label="Quality" value={quality} onChange={(event) => onQualityChange(event.target.value as DownloadQuality)}>
          <option value="720p">720p · smaller</option>
          <option value="1080p">1080p · recommended</option>
          <option value="best">Best compatible</option>
        </select>
      </label>
      <button className="primary-button download-submit" type="submit" disabled={submitting || !url.trim()} aria-label="Download">
        {submitting ? <SpinnerGapIcon className="spin" /> : <DownloadSimpleIcon weight="bold" />}
        {submitting ? "Adding…" : "Add to queue"}
      </button>
    </form>
  );
}
