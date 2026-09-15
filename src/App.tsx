import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  SpinnerGapIcon as LoaderCircle,
  WarningCircleIcon as AlertCircle,
} from "@phosphor-icons/react";
import { bridge } from "./lib/bridge";
import { applyAppearance, appearanceFromSettings, DEFAULT_APPEARANCE } from "./lib/appearance";
import { loadAppearanceMirror } from "./lib/uiPreferences";
import type { AppearancePreferences, AppSettings, BootstrapStatus, LibrarySnapshot, MediaItem, View } from "./types";
import { EmptyLibrary } from "./components/EmptyLibrary";
import { Hero } from "./components/Hero";
import { LibraryPage } from "./components/library/LibraryPage";
import { MediaCard } from "./components/MediaCard";
import { MediaRow } from "./components/MediaRow";
import { PlayerOverlay } from "./components/PlayerOverlay";
import { RecentlyRemovedPage } from "./components/library/RecentlyRemovedPage";
import { SettingsPanel } from "./components/SettingsPanel";
import { Sidebar } from "./components/Sidebar";
import { Titlebar } from "./components/Titlebar";

const DownloadPanel = lazy(() => import("./components/DownloadPanel"));
const EMPTY_LIBRARY: LibrarySnapshot = { media: [], libraryPath: "" };
const EMPTY_BOOTSTRAP: BootstrapStatus = {
  ready: false,
  dependencies: [],
  settings: {
    libraryPath: "",
    ffmpegPath: "ffmpeg",
    ffprobePath: "ffprobe",
    ytDlpPath: "yt-dlp",
    jsRuntimePath: "node",
    conversionQuality: "balanced",
    // Seed from the localStorage mirror so the pre-bootstrap render already
    // matches the saved appearance instead of flashing the defaults.
    ...DEFAULT_APPEARANCE,
    ...(loadAppearanceMirror() ?? {}),
    libraryVolumeId: null,
    youtubeCookiesBrowser: null,
    youtubeCookiesFile: null,
  },
  storage: {
    path: "", available: false, totalBytes: 0, availableBytes: 0, usedBytes: 0,
    freePercent: 0, libraryBytes: 0, mediaCount: 0, estimatedAdditionalItems: null,
    warning: false, protectionActive: false, volumeId: null, message: null,
  },
};

interface PlaybackQueue { ids: string[]; index: number }

function matches(item: MediaItem, query: string) {
  const needle = query.trim().toLocaleLowerCase();
  return !needle || [item.title, item.creator, item.description || "", item.container].some((value) => value.toLocaleLowerCase().includes(needle));
}

export default function App() {
  const [view, setView] = useState<View>("home");
  const [query, setQuery] = useState("");
  const [library, setLibrary] = useState(EMPTY_LIBRARY);
  const [bootstrap, setBootstrap] = useState(EMPTY_BOOTSTRAP);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [queue, setQueue] = useState<PlaybackQueue | null>(null);
  const [error, setError] = useState<string | null>(null);
  const scanTimer = useRef<number | null>(null);
  const appearanceRequest = useRef(0);

  const rescan = useCallback(async () => {
    setScanning(true);
    try {
      setLibrary(await bridge.rescan());
      const storage = await bridge.storageStatus();
      setBootstrap((current) => ({ ...current, storage }));
    } catch (scanError) {
      setError(String(scanError));
    } finally {
      setScanning(false);
    }
  }, []);

  useEffect(() => {
    let disposed = false;
    Promise.all([bridge.bootstrap(), bridge.library()])
      .then(([status, snapshot]) => {
        if (disposed) return;
        setBootstrap(status);
        setLibrary(snapshot);
        if (status.ready) void rescan();
      })
      .catch((loadError) => { if (!disposed) setError(String(loadError)); })
      .finally(() => { if (!disposed) setLoading(false); });
    let cleanup: () => void = () => undefined;
    void bridge.listen("library-dirty", () => {
      if (scanTimer.current !== null) window.clearTimeout(scanTimer.current);
      scanTimer.current = window.setTimeout(() => void rescan(), 1200);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else cleanup = unlisten;
    });
    return () => {
      disposed = true;
      cleanup();
      if (scanTimer.current !== null) window.clearTimeout(scanTimer.current);
    };
  }, [rescan]);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: light)");
    const preferences = appearanceFromSettings(bootstrap.settings);
    const apply = () => applyAppearance(preferences);
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [
    bootstrap.settings.themePreference,
    bootstrap.settings.accentPreference,
    bootstrap.settings.densityPreference,
    bootstrap.settings.textSizePreference,
    bootstrap.settings.reducedMotion,
  ]);

  const cycleTheme = useCallback(() => {
    const preferences = appearanceFromSettings(bootstrap.settings);
    const order: AppSettings["themePreference"][] = ["system", "dark", "light"];
    const next = order[(order.indexOf(preferences.themePreference) + 1) % order.length];
    void saveAppearance({ ...preferences, themePreference: next }).catch(() => undefined);
  }, [bootstrap.settings]);

  useEffect(() => {
    const shortcut = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        document.querySelector<HTMLInputElement>(".search-box input")?.focus();
      }
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === "d") {
        event.preventDefault();
        cycleTheme();
      }
    };
    window.addEventListener("keydown", shortcut);
    return () => window.removeEventListener("keydown", shortcut);
  }, [cycleTheme]);

  const filtered = useMemo(() => library.media.filter((item) => matches(item, query)), [library.media, query]);
  const selected = useMemo(() => queue ? library.media.find((item) => item.id === queue.ids[queue.index]) || null : null, [library.media, queue]);
  const nextItem = useMemo(() => queue && queue.index + 1 < queue.ids.length ? library.media.find((item) => item.id === queue.ids[queue.index + 1]) || null : null, [library.media, queue]);
  const previousItem = useMemo(() => queue && queue.index > 0 ? library.media.find((item) => item.id === queue.ids[queue.index - 1]) || null : null, [library.media, queue]);
  const creators = useMemo(() => {
    const grouped = new Map<string, MediaItem[]>();
    for (const item of library.media) grouped.set(item.creator, [...(grouped.get(item.creator) || []), item]);
    return [...grouped.entries()].sort((a, b) => b[1].length - a[1].length || a[0].localeCompare(b[0]));
  }, [library.media]);
  const continueWatching = useMemo(() => library.media.filter((item) => item.progressSecs > 0 && !item.watched), [library.media]);
  const favorites = useMemo(() => library.media.filter((item) => item.favorite), [library.media]);
  const recent = library.media.slice(0, 16);

  const openPlayer = useCallback((item: MediaItem, context: MediaItem[]) => {
    const ids = context.map((entry) => entry.id);
    const index = ids.indexOf(item.id);
    setQueue({ ids: index >= 0 ? ids : [item.id], index: index >= 0 ? index : 0 });
  }, []);

  const toggleFavorite = useCallback((item: MediaItem) => {
    const favorite = !item.favorite;
    setLibrary((current) => ({ ...current, media: current.media.map((entry) => entry.id === item.id ? { ...entry, favorite } : entry) }));
    void bridge.setFavorite(item.id, favorite).catch((favoriteError) => {
      setError(String(favoriteError));
      setLibrary((current) => ({ ...current, media: current.media.map((entry) => entry.id === item.id ? { ...entry, favorite: !favorite } : entry) }));
    });
  }, []);

  const saveSettings = async (settings: AppSettings) => {
    try {
      const next = await bridge.saveSettings(settings);
      setBootstrap(next);
      await rescan();
    } catch (settingsError) {
      setError(String(settingsError));
    }
  };

  const saveAppearance = async (preferences: AppearancePreferences) => {
    const request = ++appearanceRequest.current;
    const previous = bootstrap.settings;
    const optimistic = { ...previous, ...preferences };
    setBootstrap((current) => ({ ...current, settings: optimistic }));
    try {
      const saved = await bridge.saveAppearance(preferences);
      if (request === appearanceRequest.current) {
        setBootstrap((current) => ({ ...current, settings: saved }));
      }
      return saved;
    } catch (appearanceError) {
      if (request === appearanceRequest.current) {
        setBootstrap((current) => ({ ...current, settings: previous }));
        setError(String(appearanceError));
      }
      throw appearanceError;
    }
  };

  const onConversionComplete = useCallback((mediaId: string, outputPath: string) => {
    setLibrary((current) => ({
      ...current,
      media: current.media.map((item) => item.id === mediaId ? { ...item, conversionPath: outputPath, playable: true } : item),
    }));
  }, []);

  const content = () => {
    if (query) {
      return <GridPage eyebrow="Search" title={`${filtered.length} result${filtered.length === 1 ? "" : "s"}`} items={filtered} query={query} onPlay={(item) => openPlayer(item, filtered)} onFavorite={toggleFavorite} onDiscover={() => setView("discover")} onRescan={rescan} />;
    }
    if (view === "discover") return <Suspense fallback={<PageLoader label="Opening downloads…" />}><DownloadPanel onOpenLibrary={() => setView("library")} /></Suspense>;
    if (view === "removed") return <RecentlyRemovedPage onLibraryChanged={rescan} onError={setError} />;
    if (view === "settings") return <SettingsPanel bootstrap={bootstrap} onSave={saveSettings} onAppearanceChange={saveAppearance} />;
    if (view === "library") return <LibraryPage items={library.media} onPlay={openPlayer} onFavorite={toggleFavorite} onDiscover={() => setView("discover")} onRescan={rescan} onError={setError} />;
    if (view === "favorites") return <GridPage eyebrow="Hand-picked" title="Favorites" items={favorites} onPlay={(item) => openPlayer(item, favorites)} onFavorite={toggleFavorite} onDiscover={() => setView("discover")} onRescan={rescan} />;
    if (library.media.length === 0) return <EmptyLibrary onDiscover={() => setView("discover")} onRescan={rescan} />;
    return (
      <div className="home-page">
        <Hero item={continueWatching[0] || recent[0]} onPlay={(item) => openPlayer(item, continueWatching.includes(item) ? continueWatching : recent)} onFavorite={toggleFavorite} />
        <div className="home-rows">
          <MediaRow title="Continue watching" eyebrow="Pick up where you left off" items={continueWatching} onPlay={(item) => openPlayer(item, continueWatching)} onFavorite={toggleFavorite} />
          <MediaRow title="Recently added" items={recent} onPlay={(item) => openPlayer(item, recent)} onFavorite={toggleFavorite} />
          <MediaRow title="Favorites" items={favorites} onPlay={(item) => openPlayer(item, favorites)} onFavorite={toggleFavorite} />
          {creators.slice(0, 8).map(([creator, items]) => <MediaRow key={creator} title={creator} eyebrow="From your channels" items={items} onPlay={(item) => openPlayer(item, items)} onFavorite={toggleFavorite} />)}
        </div>
      </div>
    );
  };

  if (loading) return <div className="boot-screen"><div className="boot-logo">H</div><span>HOMETUBE</span><LoaderCircle className="spin" /></div>;

  return (
    <div className="app-shell">
      <header className="app-header">
        <Sidebar active={view} onNavigate={(next) => { setView(next); setQuery(""); }} />
        <Titlebar
          query={query}
          onQuery={setQuery}
          scanning={scanning}
          theme={bootstrap.settings.themePreference}
          onCycleTheme={cycleTheme}
          activeView={view}
          onNavigate={(next) => { setView(next); setQuery(""); }}
        />
      </header>
      <div className="workspace">
        <main>{content()}</main>
      </div>
      {selected ? <PlayerOverlay
        item={selected}
        nextItem={nextItem}
        previousItem={previousItem}
        onNext={() => setQueue((current) => current && current.index + 1 < current.ids.length ? { ...current, index: current.index + 1 } : current)}
        onPrevious={() => setQueue((current) => current && current.index > 0 ? { ...current, index: current.index - 1 } : current)}
        onClose={() => setQueue(null)}
        onConversionComplete={onConversionComplete}
        onError={setError}
      /> : null}
      {error ? <div className="toast" role="alert"><AlertCircle /><span>{error}</span><button onClick={() => setError(null)}>Dismiss</button></div> : null}
    </div>
  );
}

interface GridPageProps {
  eyebrow: string;
  title: string;
  items: MediaItem[];
  query?: string;
  onPlay: (item: MediaItem) => void;
  onFavorite: (item: MediaItem) => void;
  onDiscover: () => void;
  onRescan: () => void;
}

function GridPage({ eyebrow, title, items, query, onPlay, onFavorite, onDiscover, onRescan }: GridPageProps) {
  return (
    <section className="grid-page">
      <div className="page-heading"><span className="eyebrow">{eyebrow}</span><h1>{title}</h1></div>
      {items.length ? <div className="media-grid">{items.map((item) => <MediaCard key={item.id} item={item} onPlay={onPlay} onFavorite={onFavorite} />)}</div> : <EmptyLibrary query={query} onDiscover={onDiscover} onRescan={onRescan} />}
    </section>
  );
}

function PageLoader({ label }: { label: string }) {
  return <div className="page-loader"><LoaderCircle className="spin" /><span>{label}</span></div>;
}
