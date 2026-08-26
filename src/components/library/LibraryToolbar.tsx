import {
  ArrowsClockwiseIcon,
  CheckSquareIcon,
  FunnelIcon,
  SlidersHorizontalIcon,
  TrashIcon,
  XIcon,
} from "@phosphor-icons/react";
import { activeLibraryFilterCount } from "../../lib/libraryView";
import { DEFAULT_LIBRARY_VIEW, type LibraryViewPreferences } from "../../lib/uiPreferences";

interface LibraryToolbarProps {
  preferences: LibraryViewPreferences;
  onPreferencesChange: (preferences: LibraryViewPreferences) => void;
  creators: string[];
  formats: string[];
  selectionMode: boolean;
  selectedCount: number;
  visibleCount: number;
  managedCount: number;
  onToggleSelectionMode: () => void;
  onSelectVisible: () => void;
  onRescan: () => void;
  onRemoveAll: () => void;
}

export function LibraryToolbar({
  preferences,
  onPreferencesChange,
  creators,
  formats,
  selectionMode,
  selectedCount,
  visibleCount,
  managedCount,
  onToggleSelectionMode,
  onSelectVisible,
  onRescan,
  onRemoveAll,
}: LibraryToolbarProps) {
  const filters = activeLibraryFilterCount(preferences);
  const update = <K extends keyof LibraryViewPreferences>(key: K, value: LibraryViewPreferences[K]) => {
    onPreferencesChange({ ...preferences, [key]: value });
  };

  return (
    <div className="library-toolbar">
      <div className="library-filters">
        <label><span className="sr-only">Sort library</span><SlidersHorizontalIcon />
          <select value={preferences.sort} onChange={(event) => update("sort", event.target.value as LibraryViewPreferences["sort"])}>
            <option value="recent">Recently added</option>
            <option value="title">Title</option>
            <option value="creator">Creator</option>
            <option value="duration">Longest</option>
            <option value="size">Largest files</option>
          </select>
        </label>
        <label><span className="sr-only">Viewing status</span><FunnelIcon />
          <select value={preferences.viewing} onChange={(event) => update("viewing", event.target.value as LibraryViewPreferences["viewing"])}>
            <option value="all">All viewing states</option>
            <option value="unwatched">Unwatched</option>
            <option value="watching">In progress</option>
            <option value="watched">Watched</option>
          </select>
        </label>
        <label><span className="sr-only">Creator</span>
          <select value={preferences.creator} onChange={(event) => update("creator", event.target.value)}>
            <option value="">All creators</option>
            {creators.map((creator) => <option key={creator}>{creator}</option>)}
          </select>
        </label>
        <label><span className="sr-only">File format</span>
          <select value={preferences.format} onChange={(event) => update("format", event.target.value)}>
            <option value="">All formats</option>
            {formats.map((format) => <option key={format}>{format.toUpperCase()}</option>)}
          </select>
        </label>
        <label><span className="sr-only">File origin</span>
          <select value={preferences.managed} onChange={(event) => update("managed", event.target.value as LibraryViewPreferences["managed"])}>
            <option value="all">All files</option>
            <option value="hometube">HomeTube downloads</option>
            <option value="local">Other local files</option>
          </select>
        </label>
        {filters ? <button className="text-button filter-reset" onClick={() => onPreferencesChange(DEFAULT_LIBRARY_VIEW)}><XIcon /> Reset {filters}</button> : null}
      </div>
      <div className="library-tools">
        <button className={selectionMode ? "toolbar-button active" : "toolbar-button"} onClick={onToggleSelectionMode}>
          <CheckSquareIcon /> {selectionMode ? `${selectedCount} selected` : "Select"}
        </button>
        {selectionMode ? <button className="toolbar-button" onClick={onSelectVisible}>Select visible ({visibleCount})</button> : null}
        <button className="toolbar-button" onClick={onRescan}><ArrowsClockwiseIcon /> Rescan</button>
        <button className="toolbar-button danger" onClick={onRemoveAll} disabled={!managedCount}><TrashIcon /> Move all ({managedCount}) to Trash</button>
      </div>
    </div>
  );
}
