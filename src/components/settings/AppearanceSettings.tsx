import {
  DesktopIcon,
  MoonIcon,
  SunIcon,
  TextAaIcon,
} from "@phosphor-icons/react";
import type { AppearancePreferences } from "../../types";
import { SwitchControl, ToggleControl } from "../ui/Controls";

interface AppearanceSettingsProps {
  value: AppearancePreferences;
  onChange: (value: AppearancePreferences) => void;
  saving: boolean;
}

const accents: Array<{ value: AppearancePreferences["accentPreference"]; label: string }> = [
  { value: "cinema", label: "Cinema red" },
  { value: "amber", label: "Warm amber" },
  { value: "teal", label: "Deep teal" },
  { value: "blue", label: "Screen blue" },
  { value: "violet", label: "Soft violet" },
];

export function AppearanceSettings({ value, onChange, saving }: AppearanceSettingsProps) {
  const update = <K extends keyof AppearancePreferences>(key: K, next: AppearancePreferences[K]) => {
    onChange({ ...value, [key]: next });
  };

  return (
    <div className="appearance-settings">
      <div className="appearance-header">
        <div><h2>Appearance</h2><p>Changes preview and save immediately.</p></div>
        <span className={saving ? "appearance-saving active" : "appearance-saving"}>{saving ? "Saving…" : "Saved"}</span>
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
              { value: "small", label: "Small", icon: <TextAaIcon /> },
              { value: "standard", label: "Standard", icon: <TextAaIcon /> },
              { value: "large", label: "Large", icon: <TextAaIcon /> },
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
