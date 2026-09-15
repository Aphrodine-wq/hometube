import {
  ArrowsClockwiseIcon as RefreshCw,
  CompassIcon as Compass,
  FolderOpenIcon as FolderOpen,
} from "@phosphor-icons/react";

interface EmptyLibraryProps {
  query?: string;
  onDiscover: () => void;
  onRescan: () => void;
}

const SKELETON_CARDS = 10;

function SkeletonCard({ index }: { index: number }) {
  return (
    <div className="skel-card">
      <div className="skel-artwork" />
      <div className="skel-copy">
        <span className="skel-line title" style={{ width: `${88 - (index % 4) * 11}%` }} />
        <span className="skel-line meta" style={{ width: `${48 + (index % 3) * 12}%` }} />
      </div>
    </div>
  );
}

export function EmptyLibrary({ query, onDiscover, onRescan }: EmptyLibraryProps) {
  if (query) {
    return (
      <div className="empty-state compact">
        <FolderOpen size={36} />
        <h2>No matches for “{query}”</h2>
        <p>Try another title, creator, or file format.</p>
      </div>
    );
  }
  return (
    <div className="empty-stage">
      <div className="empty-skeleton" aria-hidden="true">
        {Array.from({ length: SKELETON_CARDS }, (_, index) => (
          <SkeletonCard key={index} index={index} />
        ))}
      </div>
      <div className="empty-panel">
        <span className="eyebrow">Getting started</span>
        <h1>Your library is empty</h1>
        <p>Every download lands here as a plain MP4 file you keep.</p>
        <ol className="empty-steps">
          <li>
            <span>1</span>
            <div><strong>Open Discover</strong><small>HomeTube’s built-in downloader.</small></div>
          </li>
          <li>
            <span>2</span>
            <div><strong>Paste a public YouTube link</strong><small>Single videos or entire playlists both work.</small></div>
          </li>
          <li>
            <span>3</span>
            <div><strong>Watch the grid fill in</strong><small>Finished MP4s appear right here, no extra steps.</small></div>
          </li>
        </ol>
        <div className="empty-actions">
          <button className="primary-button" onClick={onDiscover}><Compass size={18} /> Open Discover</button>
          <button className="secondary-button" onClick={onRescan}><RefreshCw size={18} /> Scan library folder</button>
        </div>
      </div>
    </div>
  );
}
