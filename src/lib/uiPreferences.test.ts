import { beforeEach, describe, expect, it } from "vitest";
import type { AppearancePreferences } from "../types";
import { loadAppearanceMirror, saveAppearanceMirror } from "./uiPreferences";

const preferences: AppearancePreferences = {
  themePreference: "light",
  accentPreference: "violet",
  densityPreference: "compact",
  textSizePreference: "large",
  reducedMotion: true,
};

describe("appearance mirror", () => {
  beforeEach(() => window.localStorage.clear());

  it("round-trips saved preferences", () => {
    saveAppearanceMirror(preferences);
    expect(loadAppearanceMirror()).toEqual(preferences);
  });

  it("returns null when nothing is stored", () => {
    expect(loadAppearanceMirror()).toBeNull();
  });

  it("rejects tampered or outdated values instead of applying them", () => {
    window.localStorage.setItem("hometube:appearance:v1", JSON.stringify({ ...preferences, accentPreference: "chartreuse" }));
    expect(loadAppearanceMirror()).toBeNull();
    window.localStorage.setItem("hometube:appearance:v1", "not json");
    expect(loadAppearanceMirror()).toBeNull();
  });
});
