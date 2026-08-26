import {
  CaretLeftIcon as ChevronLeft,
  CaretRightIcon as ChevronRight,
} from "@phosphor-icons/react";
import { useRef } from "react";
import type { MediaItem } from "../types";
import { MediaCard } from "./MediaCard";

interface MediaRowProps {
  title: string;
  eyebrow?: string;
  items: MediaItem[];
  onPlay: (item: MediaItem) => void;
  onFavorite: (item: MediaItem) => void;
}

export function MediaRow({ title, eyebrow, items, onPlay, onFavorite }: MediaRowProps) {
  const rail = useRef<HTMLDivElement>(null);
  const scroll = (direction: number) => rail.current?.scrollBy({
    left: direction * 720,
    behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth",
  });
  if (items.length === 0) return null;
  return (
    <section className="media-section">
      <div className="section-heading">
        <div>{eyebrow ? <span>{eyebrow}</span> : null}<h2>{title}</h2></div>
        <div className="rail-actions">
          <button onClick={() => scroll(-1)} aria-label={`Scroll ${title} left`}><ChevronLeft /></button>
          <button onClick={() => scroll(1)} aria-label={`Scroll ${title} right`}><ChevronRight /></button>
        </div>
      </div>
      <div className="media-rail" ref={rail}>
        {items.map((item) => <MediaCard key={item.id} item={item} onPlay={onPlay} onFavorite={onFavorite} />)}
      </div>
    </section>
  );
}
