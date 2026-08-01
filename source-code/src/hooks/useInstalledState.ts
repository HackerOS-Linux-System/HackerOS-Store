import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type { InstalledMap, InstalledSets, InstalledState } from "../types";

const EMPTY_SETS: InstalledSets = { apt: [], flatpak: [], snap: [], brew: [], hpm: [], nix: [], appimage: [] };

export function useInstalledState() {
  const [installed, setInstalled] = createSignal<InstalledMap>({});
  const [installedSets, setInstalledSets] = createSignal<InstalledSets>(EMPTY_SETS);

  async function loadCurated() {
    try {
      const states = await invoke<InstalledState[]>("check_all_installed");
      const map: InstalledMap = {};
      states.forEach(s => { map[s.key] = { installed: s.installed, version: s.version }; });
      setInstalled(map);
    } catch { /* best-effort */ }
  }

  async function refreshSets() {
    try {
      setInstalledSets(await invoke<InstalledSets>("get_installed_sets"));
    } catch { /* best-effort */ }
  }

  function isDiscoverInstalled(source: string, packageId: string): boolean {
    const sets = installedSets();
    const set = sets[source as keyof InstalledSets] as string[] | undefined;
    return set ? set.includes(packageId) : false;
  }

  function markCuratedInstalled(key: string, isInstalled: boolean) {
    setInstalled(m => ({ ...m, [key]: { installed: isInstalled } }));
  }

  return { installed, installedSets, loadCurated, refreshSets, isDiscoverInstalled, markCuratedInstalled };
}
