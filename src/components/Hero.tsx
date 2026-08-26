import {
  ClockIcon as Clock3,
  HeartIcon as Heart,
  PlayIcon as Play,
} from "@phosphor-icons/react";
import type { MediaItem } from "../types";
import { Artwork } from "./Artwork";

interface HeroProps {
  item: MediaItem;
  onPlay: (item: MediaItem) => void;
  onFavorite: (item: MediaItem) => void;
}

const duration = (seconds: number) => {
  const minutes = Math.round(seconds / 60);
  return minutes >= 60 ? `${Math.floor(minutes / 60)}h ${minutes % 60}m` : `${minutes}m`;
};

export function Hero({ item, onPlay, onFavorite }: HeroProps) {
  const progress = item.durationSecs > 0
    ? Math.min(100, Math.round((item.progressSecs / item.durationSecs) * 100))
    : 0;

  return (
    <section className="hero">
      <Artwork path={item.artworkPath} title={item.title} className="hero-art" />
      <div className="hero-copy">
        <div className="hero-kicker">Featured in your library</div>
        <h1>{item.title}</h1>
        <div className="hero-meta"><strong>{item.creator}</strong><span><Clock3 size={14} /> {duration(item.durationSecs)}</span><span>{item.container.toUpperCase()}</span></div>
        <p>{item.description || `A recent addition from ${item.creator}, ready to watch from your local HomeTube library.`}</p>
        {progress > 0 && !item.watched ? (
          <div className="hero-resume" aria-label={`${progress} percent watched`}>
            <span><span style={{ width: `${progress}%` }} /></span>
            <small>{progress}% watched</small>
          </div>
        ) : null}
        <div className="hero-actions">
          <button className="primary-button" onClick={() => onPlay(item)}><Play size={18} weight="fill" />{item.progressSecs > 0 ? "Resume" : "Play"}</button>
          <button className="secondary-button" onClick={() => onFavorite(item)}><Heart size={18} weight={item.favorite ? "fill" : "regular"} />{item.favorite ? "Favorited" : "Favorite"}</button>
        </div>
      </div>
    </section>
  );
}
