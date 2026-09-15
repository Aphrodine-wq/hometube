import { useEffect, useMemo, useState } from "react";
import { bridge } from "../../lib/bridge";
import { applyLibraryView } from "../../lib/libraryView";
import { loadLibraryView, saveLibraryView, type LibraryViewPreferences } from "../../lib/uiPreferences";
import type { MediaItem, RemovalPreview } from "../../types";
import { EmptyLibrary } from "../EmptyLibrary";
import { MediaCard } from "../MediaCard";
import { BulkActionBar } from "./BulkActionBar";
import { LibraryToolbar } from "./LibraryToolbar";
import { RemovalDialog } from "./RemovalDialog";
import { VideoDetailsPanel } from "./VideoDetailsPanel";

interface LibraryPageProps {
  items: MediaItem[];
  eyebrow?: string;
  title?: string;
  onPlay: (item: MediaItem, context: MediaItem[]) => void;
  onFavorite: (item: MediaItem) => void;
  onDiscover: () => void;
  onRescan: () => Promise<void> | void;
  onError: (error: string) => void;
}

export function LibraryPage({
  items,
  eyebrow = "Everything, offline",
  title = "Your library",
  onPlay,
  onFavorite,
  onDiscover,
  onRescan,
  onError,
}: LibraryPageProps) {
  const [preferences, setPreferences] = useState<LibraryViewPreferences>(() => loadLibraryView());
  const [selectionMode, setSelectionMode] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [details, setDetails] = useState<MediaItem | null>(null);
  const [removalPreview, setRemovalPreview] = useState<RemovalPreview | null>(null);
  const [phrase, setPhrase] = useState("");
  const [removing, setRemoving] = useState(false);

  const visible = useMemo(() => applyLibraryView(items, preferences), [items, preferences]);
  const selectedItems = useMemo(() => items.filter((item) => selected.has(item.id)), [items, selected]);
  const creators = useMemo(() => [...new Set(items.map((item) => item.creator))].sort(), [items]);
  const formats = useMemo(() => [...new Set(items.map((item) => item.container))].sort(), [items]);
  const managedCount = items.filter((item) => item.managedByHomeTube).length;

  useEffect(() => saveLibraryView(preferences), [preferences]);
  useEffect(() => {
    setSelected((current) => new Set([...current].filter((id) => items.some((item) => item.id === id))));
    if (details) {
      const refreshed = items.find((item) => item.id === details.id);
      if (!refreshed) setDetails(null);
      else if (refreshed !== details) setDetails(refreshed);
    }
  }, [items, details]);

  const toggleSelection = (item: MediaItem) => setSelected((current) => {
    const next = new Set(current);
    if (next.has(item.id)) next.delete(item.id);
    else next.add(item.id);
    return next;
  });

  const bulk = async (action: "favorite" | "watched", value: boolean) => {
    try {
      const ids = [...selected];
      const result = action === "favorite"
        ? await bridge.bulkSetFavorite(ids, value)
        : await bridge.bulkSetWatched(ids, value);
      if (result.failures.length) onError(`${result.failures.length} selected videos could not be updated.`);
      await onRescan();
    } catch (error) {
      onError(String(error));
    }
  };

  const openRemoval = async (mode: "selected" | "all", item?: MediaItem) => {
    try {
      const preview = mode === "all"
        ? await bridge.previewRemoveAllHomeTube()
        : await bridge.previewRemoveMedia(item ? [item.id] : [...selected]);
      if (!preview.eligibleCount) {
        onError("No verified HomeTube downloads are eligible for this Trash action.");
        return;
      }
      setDetails(null);
      setPhrase(preview.requiredPhrase === "MOVE ALL TO TRASH" ? "" : preview.requiredPhrase);
      setRemovalPreview(preview);
    } catch (error) {
      onError(String(error));
    }
  };

  const executeRemoval = async () => {
    if (!removalPreview) return;
    setRemoving(true);
    try {
      const result = await bridge.executeRemoval(removalPreview.token, phrase);
      if (result.failures.length) onError(`${result.failures.length} files could not be moved to Trash.`);
      setRemovalPreview(null);
      setPhrase("");
      setSelected(new Set());
      await onRescan();
    } catch (error) {
      onError(String(error));
    } finally {
      setRemoving(false);
    }
  };

  return (
    <section className="grid-page library-page">
      <div className="page-heading split">
        <div><span className="eyebrow">{eyebrow}</span><h1>{title}</h1><p>{visible.length} of {items.length} videos</p></div>
      </div>
      <LibraryToolbar
        preferences={preferences}
        onPreferencesChange={setPreferences}
        creators={creators}
        formats={formats}
        selectionMode={selectionMode}
        selectedCount={selected.size}
        visibleCount={visible.length}
        managedCount={managedCount}
        onToggleSelectionMode={() => { setSelectionMode((value) => !value); setSelected(new Set()); }}
        onSelectVisible={() => setSelected(new Set(visible.map((item) => item.id)))}
        onRescan={() => void onRescan()}
        onRemoveAll={() => void openRemoval("all")}
      />

      {visible.length ? (
        <div className="media-grid">
          {visible.map((item) => (
            <MediaCard
              key={item.id}
              item={item}
              onPlay={(entry) => onPlay(entry, visible)}
              onFavorite={onFavorite}
              selectionMode={selectionMode}
              selected={selected.has(item.id)}
              onToggleSelection={toggleSelection}
              onDetails={setDetails}
            />
          ))}
        </div>
      ) : <EmptyLibrary query={items.length ? "these filters" : undefined} onDiscover={onDiscover} onRescan={onRescan} />}

      <BulkActionBar
        selectedCount={selected.size}
        onFavorite={(value) => void bulk("favorite", value)}
        onWatched={(value) => void bulk("watched", value)}
        onRemove={() => void openRemoval("selected")}
        onClear={() => setSelected(new Set())}
      />
      <VideoDetailsPanel
        item={details}
        onOpenChange={(open) => { if (!open) setDetails(null); }}
        onPlay={(item) => onPlay(item, visible)}
        onFavorite={onFavorite}
        onWatched={(item, watched) => {
          void bridge.bulkSetWatched([item.id], watched).then(() => onRescan()).catch((error) => onError(String(error)));
        }}
        onOpenFolder={(item) => void bridge.openMediaFolder(item.id).catch((error) => onError(String(error)))}
        onRemove={(item) => void openRemoval("selected", item)}
      />
      <RemovalDialog
        preview={removalPreview}
        loading={removing}
        phrase={phrase}
        onPhraseChange={setPhrase}
        onOpenChange={(open) => { if (!open) setRemovalPreview(null); }}
        onConfirm={() => void executeRemoval()}
      />
    </section>
  );
}
