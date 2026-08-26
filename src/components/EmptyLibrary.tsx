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
    <div className="empty-state">
      <div className="empty-orbit"><FolderOpen size={38} /></div>
      <span className="eyebrow">Your screen, your files</span>
      <h1>Build your first row</h1>
      <p>Open Discover and paste a public YouTube video or playlist. Finished MP4s appear here automatically.</p>
      <div>
        <button className="primary-button" onClick={onDiscover}><Compass size={18} /> Open Discover</button>
        <button className="secondary-button" onClick={onRescan}><RefreshCw size={18} /> Scan library</button>
      </div>
    </div>
  );
}
