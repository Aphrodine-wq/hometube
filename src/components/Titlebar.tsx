import {
  ArrowsOutSimpleIcon as Maximize2,
  DesktopIcon,
  MagnifyingGlassIcon as Search,
  MinusIcon as Minus,
  MoonIcon,
  SunIcon,
  XIcon as X,
} from "@phosphor-icons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { bridge } from "../lib/bridge";
import type { AppSettings } from "../types";

type ThemePreference = AppSettings["themePreference"];

const THEME_LABELS: Record<ThemePreference, string> = {
  system: "System theme",
  dark: "Dark theme",
  light: "Light theme",
};
const THEME_ORDER: ThemePreference[] = ["system", "dark", "light"];

interface TitlebarProps {
  query: string;
  onQuery: (value: string) => void;
  scanning: boolean;
  theme: ThemePreference;
  onCycleTheme: () => void;
}

export function Titlebar({ query, onQuery, scanning, theme, onCycleTheme }: TitlebarProps) {
  const nextTheme = THEME_ORDER[(THEME_ORDER.indexOf(theme) + 1) % THEME_ORDER.length];
  const themeLabel = `${THEME_LABELS[theme]}. Switch to ${THEME_LABELS[nextTheme].toLowerCase()}`;
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
      <button className="theme-cycle" onClick={onCycleTheme} aria-label={themeLabel} title={`${themeLabel} (Ctrl+Shift+D)`}>
        {theme === "system" ? <DesktopIcon size={17} /> : theme === "dark" ? <MoonIcon size={17} /> : <SunIcon size={17} />}
      </button>
      <div className="window-actions">
        <button onClick={() => runWindowAction("minimize")} aria-label="Minimize"><Minus size={16} weight="bold" /></button>
        <button onClick={() => runWindowAction("toggleMaximize")} aria-label="Maximize"><Maximize2 size={15} weight="bold" /></button>
        <button className="window-close" onClick={() => runWindowAction("close")} aria-label="Close"><X size={16} weight="bold" /></button>
      </div>
    </div>
  );
}
