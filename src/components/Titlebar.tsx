import {
  ArrowsOutSimpleIcon as Maximize2,
  DesktopIcon,
  FilmSlateIcon as LibraryIcon,
  GearSixIcon as Settings,
  MagnifyingGlassIcon as Search,
  MinusIcon as Minus,
  MoonIcon,
  SunIcon,
  TrashIcon,
  XIcon as X,
} from "@phosphor-icons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { bridge } from "../lib/bridge";
import type { AppSettings, View } from "../types";

type ThemePreference = AppSettings["themePreference"];

const THEME_LABELS: Record<ThemePreference, string> = {
  system: "System theme",
  dark: "Dark theme",
  light: "Light theme",
};
const THEME_ORDER: ThemePreference[] = ["system", "dark", "light"];

const TOOL_VIEWS: Array<{ view: View; label: string; icon: typeof Settings }> = [
  { view: "library", label: "Library", icon: LibraryIcon },
  { view: "removed", label: "Recently removed", icon: TrashIcon },
  { view: "settings", label: "Settings", icon: Settings },
];

interface TitlebarProps {
  query: string;
  onQuery: (value: string) => void;
  scanning: boolean;
  theme: ThemePreference;
  onCycleTheme: () => void;
  activeView: View;
  onNavigate: (view: View) => void;
}

export function Titlebar({ query, onQuery, scanning, theme, onCycleTheme, activeView, onNavigate }: TitlebarProps) {
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
      <div className="titlebar-tools">
        {TOOL_VIEWS.map(({ view, label, icon: Icon }) => (
          <button
            key={view}
            className={activeView === view ? "tool-button active" : "tool-button"}
            onClick={() => onNavigate(view)}
            aria-label={label}
            aria-current={activeView === view ? "page" : undefined}
            title={label}
          >
            <Icon size={17} weight={activeView === view ? "fill" : "regular"} />
          </button>
        ))}
      </div>
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
