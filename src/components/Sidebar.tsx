import {
  CompassIcon as Compass,
  FilmSlateIcon as Film,
  GearSixIcon as Settings,
  HeartIcon as Heart,
  HouseIcon as Home,
  TrashIcon,
} from "@phosphor-icons/react";
import type { View } from "../types";

const navItems: Array<{ view: View; label: string; icon: typeof Home }> = [
  { view: "home", label: "Home", icon: Home },
  { view: "library", label: "Library", icon: Film },
  { view: "favorites", label: "Favorites", icon: Heart },
  { view: "discover", label: "Discover", icon: Compass },
  { view: "removed", label: "Recently Removed", icon: TrashIcon },
];

interface SidebarProps {
  active: View;
  onNavigate: (view: View) => void;
  count: number;
}

export function Sidebar({ active, onNavigate, count }: SidebarProps) {
  return (
    <div className="primary-nav">
      <button className="brand" onClick={() => onNavigate("home")} aria-label="HomeTube home">
        HOMETUBE
      </button>
      <nav aria-label="Primary">
        {navItems.map(({ view, label, icon: Icon }) => (
          <button
            key={view}
            className={active === view ? "nav-item active" : "nav-item"}
            onClick={() => onNavigate(view)}
            aria-current={active === view ? "page" : undefined}
          >
            <Icon size={17} weight={active === view ? "fill" : "regular"} aria-hidden="true" />
            <span>{label}</span>
            {view === "library" ? <span className="nav-count">{count}</span> : null}
          </button>
        ))}
        <button
          className={active === "settings" ? "nav-item active settings-link" : "nav-item settings-link"}
          onClick={() => onNavigate("settings")}
          aria-current={active === "settings" ? "page" : undefined}
        >
          <Settings size={17} weight={active === "settings" ? "fill" : "regular"} aria-hidden="true" />
          <span>Settings</span>
        </button>
      </nav>
    </div>
  );
}
