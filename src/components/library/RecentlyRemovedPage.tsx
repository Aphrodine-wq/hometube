import {
  ArrowClockwiseIcon,
  CheckCircleIcon,
  FolderOpenIcon,
  ShieldCheckIcon,
  TrashIcon,
} from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { bridge } from "../../lib/bridge";
import type { LegacyCandidate, RemovedItem } from "../../types";

interface RecentlyRemovedPageProps {
  onLibraryChanged: () => void;
  onError: (error: string) => void;
}

function formatBytes(bytes: number) {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unit = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** unit).toFixed(unit > 2 ? 1 : 0)} ${units[unit]}`;
}

export function RecentlyRemovedPage({ onLibraryChanged, onError }: RecentlyRemovedPageProps) {
  const [items, setItems] = useState<RemovedItem[]>([]);
  const [candidates, setCandidates] = useState<LegacyCandidate[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [adopting, setAdopting] = useState(false);

  const load = () => {
    setLoading(true);
    void Promise.all([bridge.recentlyRemoved(), bridge.listLegacyCandidates()])
      .then(([removed, legacy]) => {
        setItems(removed);
        setCandidates(legacy);
        setSelected(new Set());
      })
      .catch((error) => onError(String(error)))
      .finally(() => setLoading(false));
  };

  useEffect(load, []);

  const adopt = async () => {
    setAdopting(true);
    try {
      const result = await bridge.adoptLegacyDownloads([...selected]);
      if (result.failures.length) onError(`${result.failures.length} files could not be adopted.`);
      await onLibraryChanged();
      load();
    } catch (error) {
      onError(String(error));
    } finally {
      setAdopting(false);
    }
  };

  return (
    <section className="removed-page">
      <div className="page-heading split">
        <div><span className="eyebrow">System Trash</span><h1>Recently removed</h1><p>HomeTube keeps history here. Restore files through your desktop Trash.</p></div>
        <div className="heading-actions">
          <button className="secondary-button" onClick={() => void bridge.openSystemTrash()}><FolderOpenIcon /> Open system Trash</button>
          <button className="toolbar-button" onClick={load}><ArrowClockwiseIcon /> Refresh</button>
        </div>
      </div>

      {loading ? <div className="page-loader">Loading removal history…</div> : items.length ? (
        <div className="removed-list">
          {items.map((item) => (
            <article key={item.id}>
              <div className={`removed-status ${item.status}`}><TrashIcon /></div>
              <div><strong>{item.title}</strong><span>{item.creator} · {formatBytes(item.sizeBytes)}</span><code>{item.originalPath}</code></div>
              <div><span>{new Date(item.removedAt * 1000).toLocaleString()}</span><small>{item.status}</small></div>
            </article>
          ))}
        </div>
      ) : <div className="removed-empty"><TrashIcon weight="duotone" /><h2>Trash history is empty</h2><p>Verified HomeTube downloads moved to system Trash will be listed here.</p></div>}

      {candidates.length ? (
        <section className="legacy-adoption">
          <div className="legacy-heading">
            <div><ShieldCheckIcon /><span><strong>Review older downloads</strong><small>HomeTube found likely older downloads, but will not claim or remove them without your approval.</small></span></div>
            <button className="primary-button" onClick={() => void adopt()} disabled={!selected.size || adopting}><CheckCircleIcon />{adopting ? "Verifying…" : `Adopt ${selected.size || ""}`}</button>
          </div>
          <div className="legacy-list">
            {candidates.map((candidate) => (
              <label key={candidate.mediaId}>
                <input
                  type="checkbox"
                  checked={selected.has(candidate.mediaId)}
                  onChange={() => setSelected((current) => {
                    const next = new Set(current);
                    if (next.has(candidate.mediaId)) next.delete(candidate.mediaId);
                    else next.add(candidate.mediaId);
                    return next;
                  })}
                />
                <span><strong>{candidate.title}</strong><small>{candidate.creator} · {candidate.reason}</small><code>{candidate.path}</code></span>
              </label>
            ))}
          </div>
        </section>
      ) : null}
    </section>
  );
}
