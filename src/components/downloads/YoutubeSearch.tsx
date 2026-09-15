import { MagnifyingGlassIcon as Search } from "@phosphor-icons/react";
import { useState, type FormEvent } from "react";
import { bridge } from "../../lib/bridge";
import type { YoutubeSearchItem } from "../../types";

function formatDuration(secs: number | null) {
  if (!secs) return null;
  const total = Math.round(secs);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  return hours
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`
    : `${minutes}:${String(seconds).padStart(2, "0")}`;
}

interface YoutubeSearchProps {
  queuedUrls: string[];
  onDownload: (item: YoutubeSearchItem) => Promise<void>;
}

export function YoutubeSearch({ queuedUrls, onDownload }: YoutubeSearchProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<YoutubeSearchItem[] | null>(null);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async (event: FormEvent) => {
    event.preventDefault();
    const needle = query.trim();
    if (!needle || searching) return;
    setSearching(true);
    setError(null);
    try {
      setResults(await bridge.youtubeSearch(needle));
    } catch (searchError) {
      setError(String(searchError));
    } finally {
      setSearching(false);
    }
  };

  return (
    <section className="yt-search" aria-label="YouTube search">
      <div className="yt-search-head">
        <h2>Search YouTube</h2>
        <p>Find videos without leaving the app and queue them straight to downloads.</p>
      </div>
      <form className="yt-search-bar" onSubmit={run}>
        <label>
          <span className="sr-only">Search YouTube</span>
          <Search size={16} aria-hidden="true" />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Try “lofi hip hop radio”…"
          />
        </label>
        <button type="submit" className="secondary-button" disabled={searching || !query.trim()}>
          {searching ? "Searching…" : "Search"}
        </button>
      </form>
      {error ? (
        <p className="yt-search-error" role="alert">{error}</p>
      ) : null}
      {results?.length ? (
        <ul className="yt-results">
          {results.map((item) => {
            const queued = queuedUrls.includes(item.url);
            return (
              <li className="yt-card" key={item.videoId}>
                <div className="yt-thumb">
                  {item.thumbnailUrl ? <img src={item.thumbnailUrl} alt="" loading="lazy" /> : null}
                  {formatDuration(item.durationSecs) ? <span>{formatDuration(item.durationSecs)}</span> : null}
                </div>
                <div className="yt-copy">
                  <strong title={item.title}>{item.title}</strong>
                  <small>{item.channel}</small>
                </div>
                <button
                  type="button"
                  className="toolbar-button"
                  disabled={queued}
                  onClick={() => void onDownload(item)}
                >
                  {queued ? "In queue" : "Download"}
                </button>
              </li>
            );
          })}
        </ul>
      ) : results && !results.length ? (
        <p className="yt-search-empty">No videos matched that search.</p>
      ) : null}
    </section>
  );
}
