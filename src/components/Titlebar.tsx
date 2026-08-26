import {
  ArrowsOutSimpleIcon as Maximize2,
  MagnifyingGlassIcon as Search,
  MinusIcon as Minus,
  XIcon as X,
} from "@phosphor-icons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { bridge } from "../lib/bridge";

interface TitlebarProps {
  query: string;
  onQuery: (value: string) => void;
  scanning: boolean;
}

export function Titlebar({ query, onQuery, scanning }: TitlebarProps) {
  const runWindowAction = (action: "minimize" | "toggleMaximize" | "close") => {
    if (bridge.isTauri) void getCurrentWindow()[action]();
  };

  return (
    <div className="titlebar">
      <div className="drag-region" data-tauri-drag-region role="status" aria-live="polite">
        <span className={scanning ? "scan-dot active" : "scan-dot"} />
        <span>{scanning ? "Indexing library…" : "Local library"}</span>
      </div>
      <label className="search-box">
        <Search size={17} weight="regular" aria-hidden="true" />
        <span className="sr-only">Search your library</span>
        <input value={query} onChange={(event) => onQuery(event.target.value)} placeholder="Search titles and creators" />
        <kbd>Ctrl K</kbd>
      </label>
      <div className="window-actions">
        <button onClick={() => runWindowAction("minimize")} aria-label="Minimize"><Minus size={16} weight="bold" /></button>
        <button onClick={() => runWindowAction("toggleMaximize")} aria-label="Maximize"><Maximize2 size={15} weight="bold" /></button>
        <button className="window-close" onClick={() => runWindowAction("close")} aria-label="Close"><X size={16} weight="bold" /></button>
      </div>
    </div>
  );
}
