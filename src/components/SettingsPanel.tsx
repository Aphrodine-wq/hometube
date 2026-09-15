import {
  CheckCircleIcon as CheckCircle2,
  FloppyDiskIcon as Save,
  FolderOpenIcon as FolderOpen,
  HardDriveIcon as HardDrive,
  ShieldCheckIcon as ShieldCheck,
  XCircleIcon as XCircle,
} from "@phosphor-icons/react";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import type { AppearancePreferences, AppSettings, BootstrapStatus } from "../types";
import { AppearanceSettings } from "./settings/AppearanceSettings";

interface SettingsPanelProps {
  bootstrap: BootstrapStatus;
  onSave: (settings: AppSettings) => Promise<void>;
  onAppearanceChange: (preferences: AppearancePreferences) => Promise<AppSettings>;
}

export function SettingsPanel({ bootstrap, onSave, onAppearanceChange }: SettingsPanelProps) {
  const [settings, setSettings] = useState(bootstrap.settings);
  const [saving, setSaving] = useState(false);
  const [savingAppearance, setSavingAppearance] = useState(false);
  const [appearanceError, setAppearanceError] = useState<string | null>(null);
  useEffect(() => setSettings(bootstrap.settings), [bootstrap.settings]);

  const chooseLibrary = async () => {
    const selected = await open({ directory: true, multiple: false, title: "Choose the HomeTube library" });
    if (typeof selected === "string") setSettings((current) => ({ ...current, libraryPath: selected }));
  };
  const save = async () => {
    setSaving(true);
    try { await onSave(settings); } finally { setSaving(false); }
  };
  const changeAppearance = async (preferences: AppearancePreferences) => {
    setSettings((current) => ({ ...current, ...preferences }));
    setSavingAppearance(true);
    setAppearanceError(null);
    try {
      setSettings(await onAppearanceChange(preferences));
    } catch (error) {
      setAppearanceError(String(error));
    } finally {
      setSavingAppearance(false);
    }
  };

  return (
    <section className="settings-page">
      <div className="page-heading"><span className="eyebrow">Local by design</span><h1>Settings</h1><p>HomeTube keeps media and viewing state on this machine.</p></div>
      <div className="settings-grid">
        <div className="settings-card wide">
          <div className="settings-card-title"><div><h2>Library location</h2><p>Finished MP4 downloads are organized here by channel.</p></div><FolderOpen /></div>
          <div className="path-picker"><input aria-label="Library location" value={settings.libraryPath} onChange={(event) => setSettings({ ...settings, libraryPath: event.target.value })} /><button onClick={chooseLibrary}>Browse</button></div>
        </div>
        <div className="settings-card">
          <div className="settings-card-title"><div><h2>Storage readiness</h2><p>Capacity and safeguards for your library volume.</p></div><HardDrive /></div>
          <div className="storage-summary">
            <div><strong>{bootstrap.storage.available ? `${bootstrap.storage.freePercent.toFixed(1)}% free` : "Storage unavailable"}</strong><span>{bootstrap.storage.available ? `${formatBytes(bootstrap.storage.availableBytes)} of ${formatBytes(bootstrap.storage.totalBytes)} available` : bootstrap.storage.path}</span></div>
            <div className="storage-meter" role="progressbar" aria-label="Storage space free" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(bootstrap.storage.freePercent)}><span style={{ width: `${Math.max(0, Math.min(100, bootstrap.storage.freePercent))}%` }} /></div>
            <dl><div><dt>Library</dt><dd>{formatBytes(bootstrap.storage.libraryBytes)}</dd></div><div><dt>Videos</dt><dd>{bootstrap.storage.mediaCount}</dd></div><div><dt>Room for</dt><dd>{bootstrap.storage.estimatedAdditionalItems === null ? "—" : `~${bootstrap.storage.estimatedAdditionalItems.toLocaleString()} more`}</dd></div></dl>
            {bootstrap.storage.message ? <p className={bootstrap.storage.protectionActive ? "storage-message danger" : "storage-message"}>{bootstrap.storage.message}</p> : <p className="storage-message ok">Storage is healthy. HomeTube never deletes media automatically.</p>}
          </div>
        </div>
        <div className="settings-card wide">
          <AppearanceSettings
            value={{
              themePreference: settings.themePreference,
              accentPreference: settings.accentPreference,
              densityPreference: settings.densityPreference,
              textSizePreference: settings.textSizePreference,
              reducedMotion: settings.reducedMotion,
            }}
            onChange={(preferences) => void changeAppearance(preferences)}
            saving={savingAppearance}
            error={appearanceError}
          />
        </div>
        <div className="settings-card">
          <div className="settings-card-title"><div><h2>System readiness</h2><p>Required tools are checked directly.</p></div><ShieldCheck /></div>
          <div className="dependency-list">
            {bootstrap.dependencies.length ? bootstrap.dependencies.map((dependency) => (
              <div key={dependency.key} className="dependency-row">
                {dependency.available ? <CheckCircle2 className="ok" /> : <XCircle className={dependency.required ? "bad" : "warn"} />}
                <div><strong>{dependency.label}</strong><span>{dependency.available ? dependency.version || dependency.path : dependency.hint}</span></div>
                <small>{dependency.required ? "Required" : "Recommended"}</small>
              </div>
            )) : <p className="browser-note">Dependency diagnostics are available inside the Tauri app.</p>}
          </div>
        </div>
        <div className="settings-card">
          <div className="settings-card-title"><div><h2>YouTube access</h2><p>Use your signed-in session when YouTube asks for verification.</p></div></div>
          <label className="select-field">Cookie source<select
            value={settings.youtubeCookiesBrowser ?? ""}
            onChange={(event) => setSettings({ ...settings, youtubeCookiesBrowser: event.target.value || null })}
          >
            <option value="">No cookies</option>
            {["firefox", "chrome", "chromium", "brave", "edge", "opera", "safari", "vivaldi", "whale", "librewolf"].map((browser) => (
              <option key={browser} value={browser}>{browser[0].toUpperCase() + browser.slice(1)}</option>
            ))}
          </select></label>
          <div className="path-picker">
            <input
              aria-label="Cookies file"
              placeholder="…or point at an exported cookies.txt"
              value={settings.youtubeCookiesFile ?? ""}
              onChange={(event) => setSettings({ ...settings, youtubeCookiesFile: event.target.value })}
            />
          </div>
          <p className="browser-note">Needed when downloads fail with “Sign in to confirm you’re not a bot”. Log into YouTube in the chosen browser, or export cookies.txt with a browser extension and paste its path here.</p>
        </div>
        <div className="settings-card">
          <div className="settings-card-title"><div><h2>Tool paths</h2><p>Override commands only when they are not on PATH.</p></div></div>
          <div className="form-grid">
            <label>FFmpeg<input value={settings.ffmpegPath} onChange={(event) => setSettings({ ...settings, ffmpegPath: event.target.value })} /></label>
            <label>FFprobe<input value={settings.ffprobePath} onChange={(event) => setSettings({ ...settings, ffprobePath: event.target.value })} /></label>
            <label>yt-dlp<input value={settings.ytDlpPath} onChange={(event) => setSettings({ ...settings, ytDlpPath: event.target.value })} /></label>
            <label>JS runtime<input value={settings.jsRuntimePath} onChange={(event) => setSettings({ ...settings, jsRuntimePath: event.target.value })} /></label>
          </div>
          <label className="select-field">Conversion quality<select value={settings.conversionQuality} onChange={(event) => setSettings({ ...settings, conversionQuality: event.target.value as AppSettings["conversionQuality"] })}><option value="compact">Compact</option><option value="balanced">Balanced</option><option value="quality">High quality</option></select></label>
        </div>
      </div>
      <div className="settings-save"><span>New downloads use this location immediately.</span><button className="primary-button" disabled={saving} onClick={save}><Save size={17} weight="bold" />{saving ? "Saving…" : "Save settings"}</button></div>
    </section>
  );
}

function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unit = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** unit).toFixed(unit > 2 ? 1 : 0)} ${units[unit]}`;
}
