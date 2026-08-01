import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_SETTINGS, type AppSettings } from "../types";
import type { Lang } from "../i18n";

export function useSettings(setLang: (l: Lang) => void, addToast: (t: "success" | "error", m: string) => void, t: (k: string) => string) {
  const [settings, setSettings] = createSignal<AppSettings>(DEFAULT_SETTINGS);

  function applyLang(lang: string) {
    if (lang === "en" || lang === "pl") setLang(lang as Lang);
  }

  async function load(): Promise<AppSettings> {
    try {
      const s = await invoke<AppSettings>("get_settings");
      const merged = { ...DEFAULT_SETTINGS, ...s };
      setSettings(merged);
      applyLang(merged.language);
      return merged;
    } catch {
      return DEFAULT_SETTINGS;
    }
  }

  async function save(next: AppSettings) {
    try {
      await invoke("save_settings", { settings: next });
      const merged = { ...DEFAULT_SETTINGS, ...next };
      setSettings(merged);
      applyLang(merged.language);
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
      addToast("success", t("toast.settingsSaved"));
    } catch (err) {
      addToast("error", String(err));
    }
  }

  return { settings, load, save, reset };
}
