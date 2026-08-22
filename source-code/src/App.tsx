import { createSignal, createMemo, createEffect, onMount, onCleanup, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import {
  ShoppingBag, Gamepad2, Shield, Cpu, RefreshCw, Settings as SettingsIcon, History as HistoryIcon,
  Snowflake, Blocks, SquareTerminal,
} from "lucide-solid";
import {
  ALL_PACKAGES, GAME_LAUNCHERS, DRIVERS, type Package, type Category,
} from "./data/packages";
import type { AppInfo, DiscoverItem, DiscoverResponse, DiscoverResult, QueueJob } from "./types";
import "./App.css";

import { useI18n } from "./hooks/useI18n";
import { useToasts } from "./hooks/useToasts";
import { useSettings } from "./hooks/useSettings";
import { useInstalledState } from "./hooks/useInstalledState";
import { useOperationRunner } from "./hooks/useOperationRunner";
import { useQueue } from "./hooks/useQueue";
import { useOnlineStatus } from "./hooks/useOnlineStatus";

import { Sidebar, type NavItem } from "./components/Sidebar";
import { TerminalLog } from "./components/TerminalLog";
import { Toasts } from "./components/Toasts";
import { OfflineBanner } from "./components/OfflineBanner";
import { AppDetailModal } from "./components/AppDetailModal";
import { DiscoverView } from "./components/DiscoverView";
import { PackageList } from "./components/PackageList";
import { PentestView } from "./components/PentestView";
import { EcosystemView } from "./components/EcosystemView";
import { DevToolsView } from "./components/DevToolsView";
import { UpdateView } from "./components/UpdateView";
import { SettingsView } from "./components/SettingsView";
import { HistoryView } from "./components/HistoryView";
import { NixView } from "./components/NixView";
import { SearchResults } from "./components/SearchResults";

const VALID_SECTIONS: Category[] = ["discover", "game_launchers", "pentest_tools", "drivers", "hackeros_ecosystem", "dev_tools", "update", "settings", "history", "nix"];

interface CuratedPayload { name: string; category: string; }
interface DiscoverPayload {
  packageId: string; source: string; name: string;
  /** Flatpak only. */
  remote?: string; branch?: string;
  /** Snap only. */
  channel?: string;
}

export default function App() {
  const { setLang, t } = useI18n();
  const { toasts, addToast } = useToasts();
  const settingsApi = useSettings(setLang, addToast, t);
  const installedApi = useInstalledState();
  const runner = useOperationRunner();
  const online = useOnlineStatus();

  const [active, setActive] = createSignal<Category>("discover");
  const [search, setSearch] = createSignal("");
  const [installing, setInstalling] = createSignal<Record<string, boolean>>({});
  const [uninstalling, setUninstalling] = createSignal<Record<string, boolean>>({});
  const [pentestTag, setPentestTag] = createSignal("all");
  const [ecosystemTag, setEcosystemTag] = createSignal("all");
  const [devToolsLangTag, setDevToolsLangTag] = createSignal("all");
  const [updating, setUpdating] = createSignal(false);
  const [appInfo, setAppInfo] = createSignal<AppInfo | null>(null);
  const [updatesAvailable, setUpdatesAvailable] = createSignal<number | null>(null);
  const [selected, setSelected] = createSignal<DiscoverItem | null>(null);
  const [discoverBusyKey, setDiscoverBusyKey] = createSignal<string | null>(null);
  const [nixAvailable, setNixAvailable] = createSignal(false);
  const [hackerAvailable, setHackerAvailable] = createSignal<boolean | null>(null);
  const [podmanAvailable, setPodmanAvailable] = createSignal<boolean | null>(null);
  const [discoverSearchResults, setDiscoverSearchResults] = createSignal<DiscoverResult[]>([]);
  const [discoverSearchLoading, setDiscoverSearchLoading] = createSignal(false);

  let searchInput: HTMLInputElement | undefined;
  let appliedDefault = false;

  // ── Marks/clears the "busy" (spinner) UI state for whatever a queued job
  // refers to. Used both when a job is first enqueued and when a persisted
  // queue is reloaded on startup, so a row shows "installing…" the moment
  // it's queued (not only once it actually starts running). ──────────────
  function setBusyForJob(kind: QueueJob["kind"], payload: unknown, value: boolean) {
    if (kind === "curated-install" || kind === "curated-uninstall") {
      const { name, category } = payload as CuratedPayload;
      const key = `${category}::${name}`;
      if (kind === "curated-install") setInstalling(m => ({ ...m, [key]: value }));
      else setUninstalling(m => ({ ...m, [key]: value }));
    } else if (kind === "discover-install" || kind === "discover-uninstall") {
      const { packageId, source } = payload as DiscoverPayload;
      setDiscoverBusyKey(value ? `${source}::${packageId}` : null);
    } else if (kind === "update") {
      setUpdating(value);
    }
  }

  // ── The one place that actually knows how to run each kind of queued
  // job. `useQueue` only ever calls this for whichever job is at the front
  // of the line — see hooks/useQueue.ts for why that's sequential rather
  // than concurrent. ──────────────────────────────────────────────────────
  async function executeJob(kind: QueueJob["kind"], payload: unknown): Promise<void> {
    switch (kind) {
      case "curated-install": {
        const { name, category } = payload as CuratedPayload;
        try {
          await runner.run(`${t("btn.installing")} ${name}`, () => invoke("install_package", { name, category }));
          await installedApi.loadCurated(); // refetch, so the version number shows up immediately
          addToast("success", `${name} ${t("toast.installOk")}`);
        } catch (err) {
          addToast("error", `${t("toast.installFail")} ${err}`);
          throw err;
        } finally {
          setBusyForJob(kind, payload, false);
        }
        return;
      }
      case "curated-uninstall": {
        const { name, category } = payload as CuratedPayload;
        try {
          await runner.run(`${t("btn.uninstalling")} ${name}`, () => invoke("uninstall_package", { name, category }));
          await installedApi.loadCurated();
          addToast("success", `${name} ${t("toast.uninstallOk")}`);
        } catch (err) {
          addToast("error", `${t("toast.uninstallFail")} ${err}`);
          throw err;
        } finally {
          setBusyForJob(kind, payload, false);
        }
        return;
      }
      case "discover-install": {
        const { packageId, source, name, remote, branch, channel } = payload as DiscoverPayload;
        try {
          await runner.run(`${t("btn.installing")} ${name}`, () =>
            invoke("discover_install", { packageId, source, name, remote, branch, channel }));
          addToast("success", `${name} ${t("toast.installOk")}`);
          await installedApi.refreshSets();
        } catch (err) {
          addToast("error", `${t("toast.installFail")} ${err}`);
          throw err;
        } finally {
          setBusyForJob(kind, payload, false);
        }
        return;
      }
      case "discover-uninstall": {
        const { packageId, source, name } = payload as DiscoverPayload;
        try {
          await runner.run(`${t("btn.uninstalling")} ${name}`, () => invoke("discover_uninstall", { packageId, source }));
          addToast("success", `${name} ${t("toast.uninstallOk")}`);
          await installedApi.refreshSets();
        } catch (err) {
          addToast("error", `${t("toast.uninstallFail")} ${err}`);
          throw err;
        } finally {
          setBusyForJob(kind, payload, false);
        }
        return;
      }
      case "update": {
        try {
          await runner.run(t("update.title"), () => invoke("update_system"));
          addToast("success", t("toast.updateOk"));
          setUpdatesAvailable(0);
        } catch (err) {
          addToast("error", `${t("toast.updateFail")} ${err}`);
          throw err;
        } finally {
          setBusyForJob(kind, payload, false);
        }
        return;
      }
    }
  }

  const queue = useQueue(executeJob);

  onMount(async () => {
    installedApi.loadCurated();
    installedApi.refreshSets();

    const merged = await settingsApi.load();
    if (!appliedDefault) {
      appliedDefault = true;
      if (VALID_SECTIONS.includes(merged.default_section as Category)) {
        setActive(merged.default_section as Category);
      }
    }
    if (merged.check_updates_on_startup) {
      invoke<number>("check_updates_available").then(setUpdatesAvailable).catch(() => {});
    }

    invoke<AppInfo>("get_app_info").then(setAppInfo).catch(() => {});
    invoke<boolean>("is_nix_available").then(setNixAvailable).catch(() => setNixAvailable(false));
    invoke<boolean>("is_hacker_available").then(setHackerAvailable).catch(() => setHackerAvailable(false));
    invoke<boolean>("is_podman_available").then(setPodmanAvailable).catch(() => setPodmanAvailable(false));

    // Resume anything still queued from before the app was last closed —
    // and restore the "installing…"/"removing…" spinner on whichever rows
    // those jobs refer to, so the UI looks exactly like it would if the
    // person had just queued them again.
    const restored = await queue.hydrate();
    for (const job of restored) setBusyForJob(job.kind, job.payload, true);

    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey && e.key === "f") || (e.key === "/" && (document.activeElement as HTMLElement)?.tagName !== "INPUT")) {
        e.preventDefault();
        searchInput?.focus();
      }
      if (e.key === "Escape") { setSearch(""); runner.setShowLog(false); setSelected(null); }
    };
    window.addEventListener("keydown", onKey);
    onCleanup(() => window.removeEventListener("keydown", onKey));
  });

  const NAV = createMemo<NavItem[]>(() => [
    { id: "discover", label: t("discover.title"), icon: ShoppingBag },
    { id: "game_launchers", label: t("nav.game_launchers"), icon: Gamepad2 },
    { id: "pentest_tools", label: t("nav.pentest_tools"), icon: Shield },
    { id: "drivers", label: t("nav.drivers"), icon: Cpu },
    { id: "hackeros_ecosystem", label: t("nav.hackeros_ecosystem"), icon: Blocks },
    { id: "dev_tools", label: t("nav.dev_tools"), icon: SquareTerminal },
    { id: "nix", label: t("nav.nix"), icon: Snowflake },
    { id: "update", label: t("nav.update"), icon: RefreshCw, badge: updatesAvailable() ?? undefined },
    { id: "history", label: t("nav.history"), icon: HistoryIcon },
    { id: "settings", label: t("nav.settings"), icon: SettingsIcon },
  ]);

  // ── Curated-section install/uninstall (game launchers, pentest, drivers) ──
  function handleInstall(pkg: Package) {
    const payload: CuratedPayload = { name: pkg.name, category: pkg.category };
    setBusyForJob("curated-install", payload, true);
    if (runner.busy()) addToast("info", `${pkg.name} ${t("toast.queued")}`);
    queue.enqueue("curated-install", `${t("btn.install")}: ${pkg.name}`, payload);
  }

  function handleUninstall(pkg: Package) {
    if (!window.confirm(t("confirm.removePkg", { name: pkg.name }))) return;
    const payload: CuratedPayload = { name: pkg.name, category: pkg.category };
    setBusyForJob("curated-uninstall", payload, true);
    if (runner.busy()) addToast("info", `${pkg.name} ${t("toast.queued")}`);
    queue.enqueue("curated-uninstall", `${t("btn.uninstall")}: ${pkg.name}`, payload);
  }

  // ── Discover install/uninstall (any of apt/flatpak/snap/brew/hpm/nix/appimage) ──
  function handleDiscoverInstall(item: DiscoverItem, opts?: { remote?: string; branch?: string; channel?: string }) {
    const payload: DiscoverPayload = {
      packageId: item.package_id, source: item.source, name: item.name,
      remote: opts?.remote, branch: opts?.branch, channel: opts?.channel,
    };
    setBusyForJob("discover-install", payload, true);
    if (runner.busy()) addToast("info", `${item.name} ${t("toast.queued")}`);
    queue.enqueue("discover-install", `${t("btn.install")}: ${item.name}`, payload);
  }

  function handleDiscoverUninstall(item: DiscoverItem) {
    if (!window.confirm(t("confirm.removePkg", { name: item.name }))) return;
    const payload: DiscoverPayload = { packageId: item.package_id, source: item.source, name: item.name };
    setBusyForJob("discover-uninstall", payload, true);
    if (runner.busy()) addToast("info", `${item.name} ${t("toast.queued")}`);
    queue.enqueue("discover-uninstall", `${t("btn.uninstall")}: ${item.name}`, payload);
  }

  async function handleCancel() {
    await runner.cancel(() => addToast("info", t("toast.cancelled")));
  }

  function handleUpdate() {
    setBusyForJob("update", {}, true);
    if (runner.busy()) addToast("info", `${t("nav.update")} ${t("toast.queued")}`);
    queue.enqueue("update", t("update.title"), {});
  }

  async function handleClearCache() {
    try {
      await runner.run(t("btn.clearCache"), () => invoke("clear_cache"));
      addToast("success", t("toast.cacheCleared"));
    } catch (err) {
      addToast("error", String(err));
    }
  }

  // ── "Build Nix index" (Settings quick action + the Nix panel's own
  // button call the same backend command, `nix_update_index`) — this can
  // take 1-2 minutes, so unlike the AppImage feed refresh it goes through
  // the shared runner/TerminalLog rather than a silent fire-and-forget. ──
  async function handleBuildNixIndex() {
    try {
      await runner.run(t("settings.buildNixIndex"), () => invoke<string>("nix_update_index"));
      addToast("success", t("settings.buildNixIndex"));
    } catch (err) {
      addToast("error", String(err));
    }
  }

  const isInstalling   = (pkg: Package) => installing()[`${pkg.category}::${pkg.name}`] ?? false;
  const isUninstalling = (pkg: Package) => uninstalling()[`${pkg.category}::${pkg.name}`] ?? false;
  const isInstalled    = (pkg: Package) => installedApi.installed()[`${pkg.category}::${pkg.name}`]?.installed ?? false;
  const getVersion     = (pkg: Package) => installedApi.installed()[`${pkg.category}::${pkg.name}`]?.version;

  const searchResults = createMemo(() => {
    const q = search().trim().toLowerCase();
    if (q.length <= 1) return [];
    return ALL_PACKAGES.filter(p =>
      p.name.toLowerCase().includes(q) ||
      p.desc.toLowerCase().includes(q) ||
      (p.tags ?? []).some(tag => tag.includes(q)),
    );
  });

  // ── Global search also covers Discover (apt/flatpak/snap/brew/hpm/nix/
  // appimage) ──────────────────────────────────────────────────────────
  // The curated `searchResults` memo above is instant/offline (it just
  // filters an in-memory array), but it only ever covered the small
  // curated catalogs. Discover's much bigger catalog lives behind a live
  // backend query, so it needs its own debounced effect — same 420ms
  // debounce DiscoverView's own search box uses, so a fast typist isn't
  // firing a query per keystroke. Skipped while offline, since
  // `discover_search` would just fail for every source anyway (the
  // OfflineBanner already explains why).
  let discoverSearchDebounce: ReturnType<typeof setTimeout> | undefined;
  createEffect(() => {
    const q = search().trim();
    if (discoverSearchDebounce) clearTimeout(discoverSearchDebounce);
    if (q.length < 2 || !online()) {
      setDiscoverSearchResults([]);
      setDiscoverSearchLoading(false);
      return;
    }
    setDiscoverSearchLoading(true);
    discoverSearchDebounce = setTimeout(async () => {
      try {
        const resp = await invoke<DiscoverResponse>("discover_search", { query: q });
        // Stale-response guard: if the search box has moved on to a
        // different query by the time this resolves, don't clobber
        // newer/empty results with an outdated answer.
        if (search().trim() === q) setDiscoverSearchResults(resp.results);
      } catch {
        if (search().trim() === q) setDiscoverSearchResults([]);
      } finally {
        if (search().trim() === q) setDiscoverSearchLoading(false);
      }
    }, 420);
  });
  onCleanup(() => { if (discoverSearchDebounce) clearTimeout(discoverSearchDebounce); });

  return (
    <div class="app">
      <Sidebar
        navItems={NAV()}
        active={active()}
        search={search()}
        onSearch={setSearch}
        onNav={setActive}
        logCount={runner.logLines().length}
        logActive={runner.logActive()}
        showLog={runner.showLog()}
        onToggleLog={() => runner.setShowLog(v => !v)}
        appInfo={appInfo()}
        queueJobs={queue.jobs()}
        onDequeue={queue.dequeue}
        onReorderQueue={queue.reorder}
        searchInputRef={el => (searchInput = el)}
        theme={settingsApi.settings().theme}
        onSetTheme={settingsApi.setTheme}
      />

      <main class="main">
        <Show when={!online()}>
          <OfflineBanner />
        </Show>

        <Show
          when={search().trim().length <= 1}
          fallback={
            <SearchResults results={searchResults()} query={search()}
              isInstalling={isInstalling} isUninstalling={isUninstalling} isInstalled={isInstalled}
              getVersion={getVersion} onInstall={handleInstall} onUninstall={handleUninstall}
              discoverResults={discoverSearchResults()} discoverLoading={discoverSearchLoading()}
              isDiscoverInstalled={installedApi.isDiscoverInstalled} discoverBusyKey={discoverBusyKey()}
              onDiscoverInstall={handleDiscoverInstall} onDiscoverUninstall={handleDiscoverUninstall}
              onDiscoverOpen={setSelected} />
          }
        >
          <Show when={active() === "discover"}>
            <DiscoverView
              settings={settingsApi.settings()}
              isDiscoverInstalled={installedApi.isDiscoverInstalled}
              discoverBusyKey={discoverBusyKey()}
              onInstall={handleDiscoverInstall}
              onUninstall={handleDiscoverUninstall}
              onOpen={setSelected}
            />
          </Show>
          <Show when={active() === "game_launchers"}>
            <PackageList title={t("nav.game_launchers")} packages={GAME_LAUNCHERS}
              isInstalling={isInstalling} isUninstalling={isUninstalling} isInstalled={isInstalled}
              getVersion={getVersion} onInstall={handleInstall} onUninstall={handleUninstall} />
          </Show>
          <Show when={active() === "pentest_tools"}>
            <PentestView tag={pentestTag()} onTag={setPentestTag}
              isInstalling={isInstalling} isUninstalling={isUninstalling} isInstalled={isInstalled}
              getVersion={getVersion} onInstall={handleInstall} onUninstall={handleUninstall} />
          </Show>
          <Show when={active() === "drivers"}>
            <PackageList title={t("nav.drivers")} packages={DRIVERS}
              isInstalling={isInstalling} isUninstalling={isUninstalling} isInstalled={isInstalled}
              getVersion={getVersion} onInstall={handleInstall} onUninstall={handleUninstall} />
          </Show>
          <Show when={active() === "hackeros_ecosystem"}>
            <EcosystemView tag={ecosystemTag()} onTag={setEcosystemTag} hackerAvailable={hackerAvailable()}
              isInstalling={isInstalling} isUninstalling={isUninstalling} isInstalled={isInstalled}
              getVersion={getVersion} onInstall={handleInstall} onUninstall={handleUninstall} />
          </Show>
          <Show when={active() === "dev_tools"}>
            <DevToolsView langTag={devToolsLangTag()} onLangTag={setDevToolsLangTag}
              defaultMode={settingsApi.settings().dev_tools_default_mode} podmanAvailable={podmanAvailable()}
              isInstalling={isInstalling} isUninstalling={isUninstalling} isInstalled={isInstalled}
              getVersion={getVersion} onInstall={handleInstall} onUninstall={handleUninstall} />
          </Show>
          <Show when={active() === "update"}>
            <UpdateView updating={updating()} onUpdate={handleUpdate}
              progress={runner.progress()} onShowLog={() => runner.setShowLog(true)}
              updatesAvailable={updatesAvailable()} />
          </Show>
          <Show when={active() === "history"}>
            <HistoryView addToast={addToast} />
          </Show>
          <Show when={active() === "nix"}>
            <NixView available={nixAvailable()} run={runner.run} busy={runner.busy()}
              addToast={addToast} onBuildIndex={handleBuildNixIndex} />
          </Show>
          <Show when={active() === "settings"}>
            <SettingsView settings={settingsApi.settings()} onSave={settingsApi.save}
              onClearCache={handleClearCache} onReset={settingsApi.reset} onBuildNixIndex={handleBuildNixIndex}
              onExportSnapshot={settingsApi.exportSnapshot} onImportSnapshot={settingsApi.importSnapshot}
              busy={runner.busy()} appInfo={appInfo()} />
          </Show>
        </Show>
      </main>

      <Show when={selected()}>
        {(item) => (
          <AppDetailModal item={item()} onClose={() => setSelected(null)}
            settings={settingsApi.settings()}
            isInstalled={installedApi.isDiscoverInstalled(item().source, item().package_id)}
            busy={discoverBusyKey() === `${item().source}::${item().package_id}`}
            onInstall={(opts) => handleDiscoverInstall(item(), opts)}
            onUninstall={() => handleDiscoverUninstall(item())}
            addToast={addToast}
          />
        )}
      </Show>

      <Show when={runner.progress()}>
        <div class="progress-bar-global">
          <div class="progress-bar-fill" style={{ width: `${Math.round((runner.progress()?.progress ?? 0) * 100)}%` }} />
        </div>
      </Show>

      <Show when={runner.showLog()}>
        <TerminalLog lines={runner.logLines()} onClose={() => runner.setShowLog(false)}
          title={runner.logTitle()} active={runner.logActive()}
          onCancel={runner.busy() ? handleCancel : undefined} cancelling={runner.cancelling()} />
      </Show>

      <Toasts toasts={toasts()} />
    </div>
  );
}
