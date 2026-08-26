import { FilmSlateIcon as Film } from "@phosphor-icons/react";
import { bridge } from "../lib/bridge";

interface ArtworkProps {
  path: string | null;
  title: string;
  className?: string;
}

export function Artwork({ path, title, className = "" }: ArtworkProps) {
  return (
    <div className={`artwork ${className}`} role="img" aria-label={`${title} artwork`}>
      {path ? <img src={bridge.mediaUrl(path)} alt="" loading={className.includes("hero-art") ? "eager" : "lazy"} /> : <Film aria-hidden="true" />}
    </div>
  );
}
