import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_SETTINGS, type AppSettings } from "../types";
import type { Lang } from "../i18n";

export function useSettings(setLang: (l: Lang) => void, addToast: (t: "success" | "error", m: string) => void, t: (k: string) => string) {
  const [settings, setSettings] = createSignal<AppSettings>(DEFAULT_SETTINGS);

  function applyLang(lang: string) {
    if (lang === "en" || lang === "pl") setLang(lang as Lang);
  }

  // Sets `data-theme` on the document root, which is all the CSS in
  // App.css needs to switch palettes (see the "Light theme" block there).
  // Applied both on load and right after every save, same as `applyLang`.
  function applyTheme(theme: string) {
    document.documentElement.setAttribute("data-theme", theme || "dark");
  }

  async function load(): Promise<AppSettings> {
    try {
      const s = await invoke<AppSettings>("get_settings");
      const merged = { ...DEFAULT_SETTINGS, ...s };
      setSettings(merged);
      applyLang(merged.language);
      applyTheme(merged.theme);
      return merged;
    } catch {
      applyTheme(DEFAULT_SETTINGS.theme);
      return DEFAULT_SETTINGS;
    }
  }

  async function save(next: AppSettings) {
    try {
      await invoke("save_settings", { settings: next });
      const merged = { ...DEFAULT_SETTINGS, ...next };
      setSettings(merged);
      applyLang(merged.language);
      applyTheme(merged.theme);
      addToast("success", t("toast.settingsSaved"));
    } catch (err) {
      addToast("error", String(err));
    }
  }

  async function reset() {
    if (!window.confirm(t("settings.resetConfirm"))) return;
    try {
      const next = await invoke<AppSettings>("reset_settings");
      const merged = { ...DEFAULT_SETTINGS, ...next };
      setSettings(merged);
      applyLang(merged.language);
      applyTheme(merged.theme);
      addToast("success", t("toast.settingsSaved"));
    } catch (err) {
      addToast("error", String(err));
    }
  }

  /** Quick theme change from the sidebar toggle — saves immediately
   * (same as any other settings change) rather than needing a trip
   * through the Settings screen's dirty/Save flow. */
  async function setTheme(theme: string) {
    await save({ ...settings(), theme });
  }

  /** Backup & Restore (Settings): writes the current settings to a
   * standalone JSON file. Returns the path actually written to on
   * success, or `null` on failure (after showing an error toast) — the
   * caller doesn't need its own try/catch around this. */
  async function exportSnapshot(path: string): Promise<string | null> {
    try {
      const written = await invoke<string>("export_settings_snapshot", { path: path || undefined });
      addToast("success", `${t("toast.settingsExported")} ${written}`);
      return written;
    } catch (err) {
      addToast("error", String(err));
      return null;
    }
  }

  /** Backup & Restore (Settings): reads a previously-exported settings
   * file and makes it active, same as a normal save. */
  async function importSnapshot(path: string) {
    try {
      const next = await invoke<AppSettings>("import_settings_snapshot", { path: path || undefined });
      const merged = { ...DEFAULT_SETTINGS, ...next };
      setSettings(merged);
      applyLang(merged.language);
      applyTheme(merged.theme);
      addToast("success", t("toast.settingsImported"));
    } catch (err) {
      addToast("error", String(err));
    }
  }

  return { settings, load, save, reset, setTheme, exportSnapshot, importSnapshot };
}
