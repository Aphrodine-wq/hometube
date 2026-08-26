import {
  ArrowCounterClockwiseIcon,
  DesktopIcon,
  MoonIcon,
  SunIcon,
  TextAaIcon,
} from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { DEFAULT_APPEARANCE } from "../../lib/appearance";
import type { AppearancePreferences } from "../../types";
import { SwitchControl, ToggleControl } from "../ui/Controls";

interface AppearanceSettingsProps {
  value: AppearancePreferences;
  onChange: (value: AppearancePreferences) => void;
  saving: boolean;
  error?: string | null;
}

const accents: Array<{ value: AppearancePreferences["accentPreference"]; label: string }> = [
  { value: "cinema", label: "Cinema red" },
  { value: "amber", label: "Warm amber" },
  { value: "teal", label: "Deep teal" },
  { value: "blue", label: "Screen blue" },
  { value: "violet", label: "Soft violet" },
];

function isDefault(value: AppearancePreferences) {
  return (Object.keys(DEFAULT_APPEARANCE) as Array<keyof AppearancePreferences>)
    .every((key) => value[key] === DEFAULT_APPEARANCE[key]);
}

export function AppearanceSettings({ value, onChange, saving, error = null }: AppearanceSettingsProps) {
  const [showSaved, setShowSaved] = useState(false);
  const wasSaving = useRef(false);
  useEffect(() => {
    if (saving) {
      wasSaving.current = true;
      setShowSaved(false);
      return;
    }
    if (!wasSaving.current || error) return;
    wasSaving.current = false;
    setShowSaved(true);
    const timer = window.setTimeout(() => setShowSaved(false), 2000);
    return () => window.clearTimeout(timer);
  }, [saving, error]);

  const update = <K extends keyof AppearancePreferences>(key: K, next: AppearancePreferences[K]) => {
    onChange({ ...value, [key]: next });
  };

  const status = saving ? "Saving…" : error ? "Couldn't save" : showSaved ? "Saved" : "";
  const statusClass = saving
    ? "appearance-saving visible saving"
    : error
      ? "appearance-saving visible error"
      : showSaved
        ? "appearance-saving visible"
        : "appearance-saving";

  return (
    <div className="appearance-settings">
      <div className="appearance-header">
        <div><h2>Appearance</h2><p>Changes preview and save immediately.</p></div>
        <div className="appearance-header-actions">
          <span className={statusClass} role="status">{status}</span>
          <button
            className="appearance-reset"
            onClick={() => onChange({ ...DEFAULT_APPEARANCE })}
            disabled={isDefault(value)}
          >
            <ArrowCounterClockwiseIcon size={14} />
            Reset to defaults
          </button>
        </div>
      </div>
      {error ? <p className="appearance-error" role="alert">{error}</p> : null}
      <div className="appearance-preview" aria-hidden="true">
        <div className="demo-hero">
          <span className="demo-kicker">Continue watching</span>
          <strong>Saturday matinee</strong>
          <p>How this room looks with your theme, accent, and text size.</p>
          <div className="demo-buttons"><span className="demo-primary">Play</span><span className="demo-secondary">Details</span></div>
        </div>
        <div className="demo-card">
          <div className="demo-art"><i /></div>
          <strong>Field recording №4</strong>
          <small>Local library · 24 min</small>
        </div>
      </div>
      <div className="appearance-group">
        <span>Theme</span>
        <ToggleControl
          value={value.themePreference}
          onValueChange={(next) => update("themePreference", next)}
          label="Color theme"
          options={[
            { value: "system", label: "System", icon: <DesktopIcon /> },
            { value: "dark", label: "Dark", icon: <MoonIcon /> },
            { value: "light", label: "Light", icon: <SunIcon /> },
          ]}
        />
      </div>
      <div className="appearance-group">
        <span>Accent</span>
        <div className="accent-options" role="radiogroup" aria-label="Accent color">
          {accents.map((accent) => (
            <button
              key={accent.value}
              className={value.accentPreference === accent.value ? "active" : ""}
              data-accent-preview={accent.value}
              role="radio"
              aria-checked={value.accentPreference === accent.value}
              onClick={() => update("accentPreference", accent.value)}
            >
              <i />
              <span>{accent.label}</span>
            </button>
          ))}
        </div>
      </div>
      <div className="appearance-columns">
        <div className="appearance-group">
          <span>Content density</span>
          <ToggleControl
            value={value.densityPreference}
            onValueChange={(next) => update("densityPreference", next)}
            label="Content density"
            options={[
              { value: "comfortable", label: "Comfortable" },
              { value: "compact", label: "Compact" },
            ]}
          />
        </div>
        <div className="appearance-group">
          <span>Text size</span>
          <ToggleControl
            value={value.textSizePreference}
            onValueChange={(next) => update("textSizePreference", next)}
            label="Text size"
            options={[
              { value: "small", label: "Small", icon: <TextAaIcon size={12} /> },
              { value: "standard", label: "Standard", icon: <TextAaIcon size={15} /> },
              { value: "large", label: "Large", icon: <TextAaIcon size={18} /> },
            ]}
          />
        </div>
      </div>
      <div className="appearance-group motion-setting">
        <SwitchControl
          checked={value.reducedMotion}
          onCheckedChange={(next) => update("reducedMotion", next)}
          label="Reduce motion"
          description="Minimizes card movement, smooth scrolling, pulses, and panel animations."
        />
      </div>
    </div>
  );
}
