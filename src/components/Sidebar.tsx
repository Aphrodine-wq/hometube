import {
  CompassIcon as Compass,
  HeartIcon as Heart,
  HouseIcon as Home,
} from "@phosphor-icons/react";
import type { View } from "../types";

const navItems: Array<{ view: View; label: string; icon: typeof Home }> = [
  { view: "home", label: "Home", icon: Home },
  { view: "favorites", label: "Favorites", icon: Heart },
  { view: "discover", label: "Discover", icon: Compass },
];

interface SidebarProps {
  active: View;
  onNavigate: (view: View) => void;
}

export function Sidebar({ active, onNavigate }: SidebarProps) {
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
          </button>
        ))}
      </nav>
    </div>
  );
}
