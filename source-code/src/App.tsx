import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Search, Gamepad2, Shield, ShoppingBag, Cpu, RefreshCw,
  X, CheckCircle, AlertCircle, Loader2, ChevronRight, ChevronLeft, Star,
  Terminal, Info, Settings as SettingsIcon, Trash2, XCircle,
  ExternalLink, ImageOff, Layers,
} from "lucide-react";
import {
  ALL_PACKAGES, GAME_LAUNCHERS, PENTEST_TOOLS, DRIVERS, Package, Category
} from "./data/packages";
import { PkgIcon, SourceIcon, ICONS } from "./iconMap";
import { useTranslation, LANGUAGES, Lang } from "./i18n";
import "./App.css";

// ─── Types ────────────────────────────────────────────────────────────────────

interface Progress { step: string; message: string; progress: number; }
interface LogLine  { stream: "stdout"|"stderr"|"info"|"error"|"success"; line: string; }
interface ToastItem { id: number; type: "success"|"error"|"info"; message: string; }
interface InstalledState { key: string; installed: boolean; version?: string; }
interface DiscoverResult {
  name: string; version: string; desc: string;
  source: "apt"|"flatpak"|"snap"|"brew"; package_id: string;
  size?: string; icon?: string|null;
}
interface CategoryDef { id: string; label: string; icon: string; }
interface RatingInfo { average: number; count: number; }
interface AppDetails {
  id: string; name: string; source: string; package_id: string;
  summary: string; description: string; icon?: string|null;
  screenshots: string[]; version?: string; license?: string; homepage?: string;
  categories: string[]; size?: string; rating?: RatingInfo|null;
}
interface InstalledSets { apt: string[]; flatpak: string[]; snap: string[]; brew: string[]; }
interface AppSettings {
  language: string;
  flatpak_remote_url: string;
  apt_mirror: string;
  check_updates_on_startup: boolean;
  enabled_sources: string[];
  ratings_enabled: boolean;
  default_section: string;
}
interface AppInfo { version: string; name: string; target_release: string; }

type InstalledMap    = Record<string, { installed: boolean; version?: string }>;
type InstallingMap   = Record<string, boolean>;
type UninstallingMap = Record<string, boolean>;

const DEFAULT_SETTINGS: AppSettings = {
  language: "en",
  flatpak_remote_url: "https://dl.flathub.org/repo/flathub.flatpakrepo",
  apt_mirror: "",
  check_updates_on_startup: true,
  enabled_sources: ["apt","flatpak","snap","brew"],
  ratings_enabled: true,
  default_section: "discover",
};

const SOURCES: { id: "apt"|"flatpak"|"snap"|"brew"; label: string; color: string }[] = [
  { id:"apt",     label:"APT",     color:"#f97316" },
  { id:"flatpak", label:"Flatpak", color:"#3b82f6" },
  { id:"snap",    label:"Snap",    color:"#e11d48" },
  { id:"brew",    label:"Homebrew",color:"#84cc16" },
];

function sourceColor(s: string) { return SOURCES.find(x=>x.id===s)?.color ?? "#8e8e93"; }

// ─── Terminal log panel ───────────────────────────────────────────────────────

function TerminalLog({ lines, onClose, title, active, onCancel, cancelling }: {
  lines: LogLine[]; onClose: () => void; title: string; active: boolean;
  onCancel?: () => void; cancelling?: boolean;
}) {
  const bottomRef = useRef<HTMLDivElement>(null);
  useEffect(() => { bottomRef.current?.scrollIntoView({ behavior: "smooth" }); }, [lines]);
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="terminal-modal" onClick={e => e.stopPropagation()}>
        <div className="terminal-header">
          <div className="terminal-dots">
            <span className="dot dot-red"/><span className="dot dot-yellow"/><span className="dot dot-green"/>
          </div>
          <span className="terminal-title">{title}</span>
          {active && <Loader2 size={13} className="spin terminal-spinner"/>}
          {active && onCancel && (
            <button className="terminal-cancel" onClick={onCancel} disabled={cancelling} title="Cancel">
              <XCircle size={13}/> {cancelling ? "Cancelling…" : "Cancel"}
            </button>
          )}
          <button className="terminal-close" onClick={onClose}><X size={14}/></button>
        </div>
        <div className="terminal-body">
          {lines.map((l, i) => (
            <div key={i} className={`log-line log-${l.stream}`}>
              <span className="log-prefix">
                {l.stream==="stdout"?">" : l.stream==="stderr"?"!" :
                 l.stream==="info"?"•" : l.stream==="success"?"✓" : "✗"}
              </span>
              <span className="log-text">{l.line}</span>
            </div>
          ))}
          <div ref={bottomRef}/>
        </div>
      </div>
    </div>
  );
}

// ─── Star rating ──────────────────────────────────────────────────────────────

function StarRating({ rating, size=14 }: { rating: RatingInfo; size?: number }) {
  const full = Math.round(rating.average);
  return (
    <div className="star-rating">
      {[1,2,3,4,5].map(i => (
        <Star key={i} size={size} fill={i<=full ? "currentColor" : "none"}
          className={i<=full ? "star-filled" : "star-empty"}/>
      ))}
      <span className="star-rating-text">{rating.average.toFixed(1)} ({rating.count})</span>
    </div>
  );
}

// ─── App ──────────────────────────────────────────────────────────────────────

export default function App() {
  const [active, setActive]             = useState<Category>("discover");
  const [search, setSearch]             = useState("");
  const [installing, setInstalling]     = useState<InstallingMap>({});
  const [uninstalling, setUninstalling] = useState<UninstallingMap>({});
  const [installed, setInstalled]       = useState<InstalledMap>({});
  const [toasts, setToasts]             = useState<ToastItem[]>([]);
  const [pentestTag, setPentestTag]     = useState("all");
  const [updating, setUpdating]         = useState(false);
  const [logLines, setLogLines]         = useState<LogLine[]>([]);
  const [logTitle, setLogTitle]         = useState("");
  const [showLog, setShowLog]           = useState(false);
  const [logActive, setLogActive]       = useState(false);
  const [progress, setProgress]         = useState<Progress|null>(null);
  const [cancelling, setCancelling]     = useState(false);
  const [busy, setBusy]                 = useState(false);
  const [settings, setSettings]         = useState<AppSettings>(DEFAULT_SETTINGS);
  const [appInfo, setAppInfo]           = useState<AppInfo|null>(null);
  const [updatesAvailable, setUpdatesAvailable] = useState<number|null>(null);
  const [installedSets, setInstalledSets] = useState<InstalledSets>({apt:[],flatpak:[],snap:[],brew:[]});
  const [selected, setSelected]         = useState<{package_id:string; source:string; name:string}|null>(null);
  const toastId  = useRef(0);
  const searchRef = useRef<HTMLInputElement>(null);
  const appliedDefault = useRef(false);
  const { lang, setLang, t } = useTranslation("en");

  const NAV: { id: Category; label: string; icon: typeof Star; badge?: number }[] = [
    { id:"discover",       label:t("discover.title"),     icon:ShoppingBag },
    { id:"game_launchers", label:t("nav.game_launchers"), icon:Gamepad2    },
    { id:"pentest_tools",  label:t("nav.pentest_tools"),  icon:Shield      },
    { id:"drivers",        label:t("nav.drivers"),        icon:Cpu         },
    { id:"update",         label:t("nav.update"),         icon:RefreshCw, badge: updatesAvailable ?? undefined },
    { id:"settings",       label:t("nav.settings"),       icon:SettingsIcon},
  ];

  // ── Load installed state + settings + app info + installed sets on startup ──
  useEffect(() => {
    invoke<InstalledState[]>("check_all_installed").then(states => {
      const map: InstalledMap = {};
      states.forEach(s => { map[s.key] = { installed: s.installed, version: s.version }; });
      setInstalled(map);
    }).catch(() => {});

    invoke<InstalledSets>("get_installed_sets").then(setInstalledSets).catch(() => {});

    invoke<AppSettings>("get_settings").then(s => {
      const merged = { ...DEFAULT_SETTINGS, ...s };
      setSettings(merged);
      if (merged.language === "en" || merged.language === "pl") setLang(merged.language as Lang);
      if (!appliedDefault.current) {
        appliedDefault.current = true;
        const valid: Category[] = ["discover","game_launchers","pentest_tools","drivers","update","settings"];
        if (valid.includes(merged.default_section as Category)) setActive(merged.default_section as Category);
      }
      if (merged.check_updates_on_startup) {
        invoke<number>("check_updates_available").then(setUpdatesAvailable).catch(() => {});
      }
    }).catch(() => {});

    invoke<AppInfo>("get_app_info").then(setAppInfo).catch(() => {});
  }, [setLang]);

  // ── Listen to backend events ──────────────────────────────────────────────
  useEffect(() => {
    const u1 = listen<Progress>("install_progress", e => setProgress(e.payload));
    const u2 = listen<LogLine>("install_log", e => setLogLines(prev => [...prev, e.payload]));
    return () => { u1.then(f=>f()); u2.then(f=>f()); };
  }, []);

  // ── Keyboard shortcuts ────────────────────────────────────────────────────
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if ((e.ctrlKey && e.key==="f") || (e.key==="/" && document.activeElement?.tagName!=="INPUT")) {
        e.preventDefault(); searchRef.current?.focus();
      }
      if (e.key==="Escape") { setSearch(""); setShowLog(false); setSelected(null); }
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, []);

  const addToast = useCallback((type: ToastItem["type"], message: string) => {
    const id = ++toastId.current;
    setToasts(t => [...t, { id, type, message }]);
    setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 5000);
  }, []);

  const refreshInstalledSets = useCallback(() => {
    invoke<InstalledSets>("get_installed_sets").then(setInstalledSets).catch(() => {});
  }, []);

  // ── Curated-section install/uninstall (game launchers, pentest, drivers) ──
  const handleInstall = useCallback(async (pkg: Package) => {
    const key = `${pkg.category}::${pkg.name}`;
    setInstalling(m => ({...m, [key]: true}));
    setBusy(true);
    setLogLines([]);
    setLogTitle(`${t("btn.installing")} ${pkg.name}`);
    setShowLog(true);
    setLogActive(true);
    setCancelling(false);
    setProgress({step:"start", message:"Starting…", progress:0});
    try {
      await invoke("install_package", { name: pkg.name, category: pkg.category });
      setInstalled(m => ({...m, [key]: {installed:true}}));
      addToast("success", `${pkg.name} ${t("toast.installOk")}`);
    } catch (err) {
      addToast("error", `${t("toast.installFail")} ${err}`);
    } finally {
      setInstalling(m => ({...m, [key]: false}));
      setLogActive(false);
      setBusy(false);
      setCancelling(false);
      setTimeout(() => setProgress(null), 1200);
    }
  }, [addToast, t]);

  const handleUninstall = useCallback(async (pkg: Package) => {
    const key = `${pkg.category}::${pkg.name}`;
    const confirmed = window.confirm(lang==="pl" ? `Na pewno usunąć ${pkg.name}?` : `Remove ${pkg.name}?`);
    if (!confirmed) return;
    setUninstalling(m => ({...m, [key]: true}));
    setBusy(true);
    setLogLines([]);
    setLogTitle(`${t("btn.uninstalling")} ${pkg.name}`);
    setShowLog(true);
    setLogActive(true);
    setCancelling(false);
    setProgress({step:"start", message:"Starting…", progress:0});
    try {
      await invoke("uninstall_package", { name: pkg.name, category: pkg.category });
      setInstalled(m => ({...m, [key]: {installed:false}}));
      addToast("success", `${pkg.name} ${t("toast.uninstallOk")}`);
    } catch (err) {
      addToast("error", `${t("toast.uninstallFail")} ${err}`);
    } finally {
      setUninstalling(m => ({...m, [key]: false}));
      setLogActive(false);
      setBusy(false);
      setCancelling(false);
      setTimeout(() => setProgress(null), 1200);
    }
  }, [addToast, t, lang]);

  // ── Discover install/uninstall (any of apt/flatpak/snap/brew) ────────────
  const [discoverBusyKey, setDiscoverBusyKey] = useState<string|null>(null);

  const handleDiscoverInstall = useCallback(async (item: {package_id:string; source:string; name:string}) => {
    const key = `${item.source}::${item.package_id}`;
    setDiscoverBusyKey(key);
    setBusy(true);
    setLogLines([]);
    setLogTitle(`${t("btn.installing")} ${item.name}`);
    setShowLog(true);
    setLogActive(true);
    setCancelling(false);
    setProgress({step:"start", message:"Starting…", progress:0});
    try {
      await invoke("discover_install", { packageId: item.package_id, source: item.source });
      addToast("success", `${item.name} ${t("toast.installOk")}`);
      refreshInstalledSets();
    } catch (err) {
      addToast("error", `${t("toast.installFail")} ${err}`);
    } finally {
      setDiscoverBusyKey(null);
      setLogActive(false);
      setBusy(false);
      setCancelling(false);
      setTimeout(() => setProgress(null), 1200);
    }
  }, [addToast, t, refreshInstalledSets]);

  const handleDiscoverUninstall = useCallback(async (item: {package_id:string; source:string; name:string}) => {
    const confirmed = window.confirm(lang==="pl" ? `Na pewno usunąć ${item.name}?` : `Remove ${item.name}?`);
    if (!confirmed) return;
    const key = `${item.source}::${item.package_id}`;
    setDiscoverBusyKey(key);
    setBusy(true);
    setLogLines([]);
    setLogTitle(`${t("btn.uninstalling")} ${item.name}`);
    setShowLog(true);
    setLogActive(true);
    setCancelling(false);
    setProgress({step:"start", message:"Starting…", progress:0});
    try {
      await invoke("discover_uninstall", { packageId: item.package_id, source: item.source });
      addToast("success", `${item.name} ${t("toast.uninstallOk")}`);
      refreshInstalledSets();
    } catch (err) {
      addToast("error", `${t("toast.uninstallFail")} ${err}`);
    } finally {
      setDiscoverBusyKey(null);
      setLogActive(false);
      setBusy(false);
      setCancelling(false);
      setTimeout(() => setProgress(null), 1200);
    }
  }, [addToast, t, lang, refreshInstalledSets]);

  const handleCancel = useCallback(async () => {
    setCancelling(true);
    try { await invoke("cancel_install"); addToast("info", t("toast.cancelled")); }
    catch { /* best-effort */ }
  }, [addToast, t]);

  const handleUpdate = useCallback(async () => {
    setUpdating(true);
    setBusy(true);
    setLogLines([]);
    setLogTitle(t("update.title"));
    setShowLog(true);
    setLogActive(true);
    setCancelling(false);
    setProgress({step:"update", message:"Updating…", progress:0.05});
    try {
      await invoke("update_system");
      addToast("success", t("toast.updateOk"));
      setUpdatesAvailable(0);
    } catch (err) {
      addToast("error", `${t("toast.updateFail")} ${err}`);
    } finally {
      setUpdating(false);
      setLogActive(false);
      setBusy(false);
      setCancelling(false);
      setTimeout(() => setProgress(null), 1200);
    }
  }, [addToast, t]);

  const handleSaveSettings = useCallback(async (next: AppSettings) => {
    try {
      await invoke("save_settings", { settings: next });
      setSettings({ ...DEFAULT_SETTINGS, ...next });
      if (next.language === "en" || next.language === "pl") setLang(next.language as Lang);
      addToast("success", t("toast.settingsSaved"));
    } catch (err) {
      addToast("error", String(err));
    }
  }, [addToast, t, setLang]);

  const handleResetSettings = useCallback(async () => {
    const confirmed = window.confirm(t("settings.resetConfirm"));
    if (!confirmed) return;
    try {
      const next = await invoke<AppSettings>("reset_settings");
      const merged = { ...DEFAULT_SETTINGS, ...next };
      setSettings(merged);
      if (merged.language === "en" || merged.language === "pl") setLang(merged.language as Lang);
      addToast("success", t("toast.settingsSaved"));
    } catch (err) {
      addToast("error", String(err));
    }
  }, [addToast, t, setLang]);

  const handleClearCache = useCallback(async () => {
    setBusy(true);
    setLogLines([]);
    setLogTitle(t("btn.clearCache"));
    setShowLog(true);
    setLogActive(true);
    setProgress({step:"cache", message:"Clearing…", progress:0.1});
    try {
      await invoke("clear_cache");
      addToast("success", t("toast.cacheCleared"));
    } catch (err) {
      addToast("error", String(err));
    } finally {
      setLogActive(false);
      setBusy(false);
      setTimeout(() => setProgress(null), 1200);
    }
  }, [addToast, t]);

  const isInstalling   = (pkg: Package) => installing[`${pkg.category}::${pkg.name}`] ?? false;
  const isUninstalling = (pkg: Package) => uninstalling[`${pkg.category}::${pkg.name}`] ?? false;
  const isInstalled    = (pkg: Package) => installed[`${pkg.category}::${pkg.name}`]?.installed ?? false;
  const getVersion      = (pkg: Package) => installed[`${pkg.category}::${pkg.name}`]?.version;

  const isDiscoverInstalled = useCallback((source: string, packageId: string) => {
    const set = (installedSets as any)[source] as string[] | undefined;
    return set ? set.includes(packageId) : false;
  }, [installedSets]);

  const searchResults = search.trim().length > 1
    ? ALL_PACKAGES.filter(p =>
        p.name.toLowerCase().includes(search.toLowerCase()) ||
        p.desc.toLowerCase().includes(search.toLowerCase()) ||
        (p.tags??[]).some(t => t.includes(search.toLowerCase()))
      ) : [];

  const sharedProps = {
    isInstalling, isUninstalling, isInstalled, getVersion,
    onInstall: handleInstall, onUninstall: handleUninstall, t,
  };

  return (
    <div className="app">
      {/* Sidebar */}
      <aside className="sidebar">
        <div className="sidebar-logo">
          <Terminal size={20} className="logo-icon"/>
          <span className="logo-text">HackerOS Store</span>
        </div>

        <div className="search-wrap">
          <Search size={14} className="search-icon"/>
          <input ref={searchRef} className="search-input" placeholder={t("search.placeholder")}
            value={search} onChange={e => setSearch(e.target.value)}/>
          {search && <button className="search-clear" onClick={() => setSearch("")}><X size={12}/></button>}
        </div>

        <nav className="nav">
          {NAV.map(({id, label, icon:Icon, badge}) => (
            <button key={id} className={`nav-item ${active===id && !search ? "active" : ""}`}
              onClick={() => { setActive(id); setSearch(""); }}>
              <Icon size={16} className="nav-icon"/>
              <span>{label}</span>
              {!!badge && <span className="nav-badge">{badge}</span>}
              {active===id && !search && <ChevronRight size={12} className="nav-arrow"/>}
            </button>
          ))}
        </nav>

        {logLines.length > 0 && (
          <button className="log-toggle" onClick={() => setShowLog(v=>!v)}>
            <Terminal size={13}/>
            <span>{logActive ? "…" : t("btn.viewLog")}</span>
            {logActive && <Loader2 size={11} className="spin"/>}
          </button>
        )}

        <div className="sidebar-footer">
          v{appInfo?.version ?? "0.9.0"} · {appInfo?.target_release?.split(" ")[0] ?? "Debian"}
        </div>
      </aside>

      {/* Main */}
      <main className="main">
        {search.trim().length > 1 ? (
          <SearchResults results={searchResults} query={search} {...sharedProps}/>
        ) : active==="discover" ? (
          <DiscoverView t={t}
            settings={settings}
            isDiscoverInstalled={isDiscoverInstalled}
            discoverBusyKey={discoverBusyKey}
            onInstall={handleDiscoverInstall}
            onUninstall={handleDiscoverUninstall}
            onOpen={(item) => setSelected(item)}/>
        ) : active==="game_launchers" ? (
          <PackageList title={t("nav.game_launchers")} packages={GAME_LAUNCHERS} {...sharedProps}/>
        ) : active==="pentest_tools" ? (
          <PentestView tag={pentestTag} onTag={setPentestTag} {...sharedProps} t={t}/>
        ) : active==="drivers" ? (
          <PackageList title={t("nav.drivers")} packages={DRIVERS} {...sharedProps}/>
        ) : active==="update" ? (
          <UpdateView updating={updating} onUpdate={handleUpdate}
            progress={progress} onShowLog={() => setShowLog(true)}
            updatesAvailable={updatesAvailable} t={t}/>
        ) : active==="settings" ? (
          <SettingsView settings={settings} onSave={handleSaveSettings}
            onClearCache={handleClearCache} onReset={handleResetSettings}
            busy={busy} appInfo={appInfo} t={t}/>
        ) : null}
      </main>

      {/* App detail modal */}
      {selected && (
        <AppDetailModal item={selected} onClose={() => setSelected(null)}
          settings={settings}
          isInstalled={isDiscoverInstalled(selected.source, selected.package_id)}
          busy={discoverBusyKey === `${selected.source}::${selected.package_id}`}
          onInstall={() => handleDiscoverInstall(selected)}
          onUninstall={() => handleDiscoverUninstall(selected)}
          t={t}/>
      )}

      {/* Global progress bar */}
      {progress && (
        <div className="progress-bar-global">
          <div className="progress-bar-fill" style={{width:`${Math.round(progress.progress*100)}%`}}/>
        </div>
      )}

      {/* Terminal modal */}
      {showLog && (
        <TerminalLog lines={logLines} onClose={() => setShowLog(false)}
          title={logTitle} active={logActive}
          onCancel={busy ? handleCancel : undefined} cancelling={cancelling}/>
      )}

      {/* Toasts */}
      <div className="toast-stack">
        {toasts.map(ts => (
          <div key={ts.id} className={`toast toast-${ts.type}`}>
            {ts.type==="success" && <CheckCircle size={15}/>}
            {ts.type==="error"   && <AlertCircle size={15}/>}
            {ts.type==="info"    && <Info size={15}/>}
            <span>{ts.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ─── Install Button (curated sections) ────────────────────────────────────────

function InstallBtn({ pkg, installing, uninstalling, installed, version, onInstall, onUninstall, t }: {
  pkg: Package; installing: boolean; uninstalling: boolean; installed: boolean;
  version?: string; onInstall: (p: Package) => void; onUninstall: (p: Package) => void;
  t: (k: string) => string;
}) {
  if (uninstalling) return (
    <button className="btn btn-installing" disabled>
      <Loader2 size={13} className="spin"/> {t("btn.uninstalling")}
    </button>
  );
  if (installed) return (
    <div className="install-done-wrap">
      <button className="btn btn-installed" disabled>
        <CheckCircle size={13}/> {t("btn.installed")}
      </button>
      {version && <span className="installed-version">{version}</span>}
      <button className="btn-uninstall" onClick={() => onUninstall(pkg)} title={t("btn.uninstall")}>
        <Trash2 size={13}/>
      </button>
    </div>
  );
  if (installing) return (
    <button className="btn btn-installing" disabled>
      <Loader2 size={13} className="spin"/> {t("btn.installing")}
    </button>
  );
  return <button className="btn btn-install" onClick={() => onInstall(pkg)}>{t("btn.install")}</button>;
}

// ─── Package Row (curated sections) ───────────────────────────────────────────

function PkgRow({ pkg, isInstalling, isUninstalling, isInstalled, getVersion, onInstall, onUninstall, t }: {
  pkg: Package;
  isInstalling: (p: Package) => boolean;
  isUninstalling: (p: Package) => boolean;
  isInstalled:  (p: Package) => boolean;
  getVersion:   (p: Package) => string|undefined;
  onInstall:    (p: Package) => void;
  onUninstall:  (p: Package) => void;
  t: (k: string) => string;
}) {
  const [info, setInfo]     = useState<{size?:string;version?:string;note?:string}|null>(null);
  const [showInfo, setShowInfo] = useState(false);

  const loadInfo = async () => {
    if (info) { setShowInfo(v=>!v); return; }
    try {
      const r = await invoke<{size:string|null;version:string|null;note?:string}>(
        "get_package_info", { name: pkg.name, category: pkg.category }
      );
      setInfo({ size: r.size??undefined, version: r.version??undefined, note: r.note });
      setShowInfo(true);
    } catch { setInfo({}); setShowInfo(true); }
  };

  return (
    <div className={`pkg-row ${isInstalled(pkg) ? "pkg-row--installed" : ""}`}>
      <div className="pkg-row-icon"><PkgIcon name={pkg.icon}/></div>
      <div className="pkg-row-body">
        <div className="pkg-row-name">
          {pkg.name}
          {isInstalled(pkg) && <span className="row-installed-badge">{t("btn.installed").toLowerCase()}</span>}
        </div>
        <div className="pkg-row-desc">{pkg.desc}</div>
        {pkg.tags && (
          <div className="pkg-row-tags">
            {pkg.tags.slice(0,4).map(tg => <span key={tg} className="tag">{tg}</span>)}
          </div>
        )}
        {showInfo && info && (
          <div className="pkg-info-line">
            {info.version && <span>{t("info.latest")} {info.version}</span>}
            {info.size    && <span>{t("info.size")} {info.size}</span>}
            {info.note    && <span>{info.note}</span>}
            {!info.version && !info.size && !info.note && <span>{t("info.noInfo")}</span>}
          </div>
        )}
      </div>
      <div className="pkg-row-actions">
        <button className="btn-info" onClick={loadInfo} title="Package info"><Info size={14}/></button>
        <InstallBtn pkg={pkg} installing={isInstalling(pkg)} uninstalling={isUninstalling(pkg)}
          installed={isInstalled(pkg)} version={getVersion(pkg)}
          onInstall={onInstall} onUninstall={onUninstall} t={t}/>
      </div>
    </div>
  );
}

// ─── Package List (curated sections) ──────────────────────────────────────────

function PackageList({ title, packages, isInstalling, isUninstalling, isInstalled, getVersion, onInstall, onUninstall, t }: {
  title: string; packages: Package[];
  isInstalling: (p: Package) => boolean;
  isUninstalling: (p: Package) => boolean;
  isInstalled:  (p: Package) => boolean;
  getVersion:   (p: Package) => string|undefined;
  onInstall:    (p: Package) => void;
  onUninstall:  (p: Package) => void;
  t: (k: string) => string;
}) {
  return (
    <div className="view">
      <h1 className="view-title">{title}</h1>
      <div className="pkg-list">
        {packages.map(pkg => (
          <PkgRow key={pkg.name} pkg={pkg} isInstalling={isInstalling} isUninstalling={isUninstalling}
            isInstalled={isInstalled} getVersion={getVersion}
            onInstall={onInstall} onUninstall={onUninstall} t={t}/>
        ))}
      </div>
    </div>
  );
}

// ─── Discover: app card ───────────────────────────────────────────────────────

function AppIcon({ icon, source, size=40 }: { icon?: string|null; source: string; size?: number }) {
  const [failed, setFailed] = useState(false);
  if (icon && !failed) {
    return <img src={icon} className="app-icon-img" style={{width:size,height:size}}
      onError={() => setFailed(true)} alt=""/>;
  }
  return (
    <div className="app-icon-fallback" style={{width:size,height:size,color:sourceColor(source)}}>
      <SourceIcon source={source} size={Math.round(size*0.55)}/>
    </div>
  );
}

function DiscoverCard({ result, installed, busy, onInstall, onUninstall, onOpen, t }: {
  result: DiscoverResult; installed: boolean; busy: boolean;
  onInstall: (i: {package_id:string;source:string;name:string}) => void;
  onUninstall: (i: {package_id:string;source:string;name:string}) => void;
  onOpen: (i: {package_id:string;source:string;name:string}) => void;
  t: (k: string) => string;
}) {
  const item = { package_id: result.package_id, source: result.source, name: result.name };
  return (
    <div className="app-card" onClick={() => onOpen(item)}>
      <AppIcon icon={result.icon} source={result.source} size={48}/>
      <div className="app-card-body">
        <div className="app-card-name">{result.name}</div>
        <span className="discover-source" style={{borderColor:sourceColor(result.source),color:sourceColor(result.source)}}>
          {result.source}
        </span>
        <div className="app-card-desc">{result.desc || "No description available."}</div>
      </div>
      <div className="app-card-actions" onClick={e => e.stopPropagation()}>
        {busy ? (
          <button className="btn btn-installing" disabled><Loader2 size={13} className="spin"/></button>
        ) : installed ? (
          <div className="install-done-wrap">
            <button className="btn btn-installed" disabled><CheckCircle size={13}/></button>
            <button className="btn-uninstall" onClick={() => onUninstall(item)} title={t("btn.uninstall")}>
              <Trash2 size={13}/>
            </button>
          </div>
        ) : (
          <button className="btn btn-install" onClick={() => onInstall(item)}>{t("btn.install")}</button>
        )}
      </div>
    </div>
  );
}

// ─── Discover View ────────────────────────────────────────────────────────────

function DiscoverView({ t, settings, isDiscoverInstalled, discoverBusyKey, onInstall, onUninstall, onOpen }: {
  t: (k: string) => string;
  settings: AppSettings;
  isDiscoverInstalled: (source: string, packageId: string) => boolean;
  discoverBusyKey: string|null;
  onInstall: (i: {package_id:string;source:string;name:string}) => void;
  onUninstall: (i: {package_id:string;source:string;name:string}) => void;
  onOpen: (i: {package_id:string;source:string;name:string}) => void;
}) {
  const [categories, setCategories] = useState<CategoryDef[]>([]);
  const [category, setCategory]     = useState<CategoryDef|null>(null);
  const [query, setQuery]           = useState("");
  const [results, setResults]       = useState<DiscoverResult[]>([]);
  const [loading, setLoading]       = useState(false);
  const [searched, setSearched]     = useState(false);
  const debRef = useRef<ReturnType<typeof setTimeout>|null>(null);

  useEffect(() => {
    invoke<CategoryDef[]>("discover_categories").then(setCategories).catch(() => {});
  }, []);

  useEffect(() => {
    if (debRef.current) clearTimeout(debRef.current);
    if (query.trim().length < 2) { if (!category) { setResults([]); setSearched(false); } return; }
    debRef.current = setTimeout(async () => {
      setLoading(true); setSearched(true);
      try { setResults(await invoke<DiscoverResult[]>("discover_search", { query: query.trim() })); }
      catch { setResults([]); }
      finally { setLoading(false); }
    }, 420);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query]);

  const openCategory = async (c: CategoryDef) => {
    setCategory(c);
    setQuery("");
    setLoading(true);
    setSearched(true);
    try { setResults(await invoke<DiscoverResult[]>("discover_browse", { categoryId: c.id })); }
    catch { setResults([]); }
    finally { setLoading(false); }
  };

  const backToCategories = () => { setCategory(null); setResults([]); setSearched(false); setQuery(""); };

  const noSourcesEnabled = (settings?.enabled_sources?.length ?? 0) === 0;
  const showingResults = query.trim().length >= 2 || category !== null;

  return (
    <div className="view">
      <div className="discover-hero">
        <div className="store-hero-badge">{t("discover.title")}</div>
        <h1 className="store-hero-title">{t("store.hero.title")}</h1>
        <p className="store-hero-sub">{t("discover.sub")}</p>
        <div className="discover-search-wrap">
          <Search size={15} className="discover-search-icon"/>
          <input className="discover-search-input" placeholder={t("discover.searchPlaceholder")}
            value={query} onChange={e => { setQuery(e.target.value); if (category) setCategory(null); }}/>
          {query && <button className="search-clear" onClick={() => setQuery("")}><X size={13}/></button>}
        </div>
      </div>

      {noSourcesEnabled && (
        <p className="view-sub discover-warning">
          <AlertCircle size={14}/> All package sources are disabled in Settings — enable at least one to browse Discover.
        </p>
      )}

      {!showingResults ? (
        <>
          <h2 className="section-heading">{t("discover.categories")}</h2>
          <div className="category-grid">
            {(categories ?? []).map(c => {
              const Icon = ICONS[c.icon] ?? Layers;
              return (
                <button key={c.id} className="category-card" onClick={() => openCategory(c)}>
                  <div className="category-card-icon"><Icon size={26}/></div>
                  <span>{t(`category.${c.id}`)}</span>
                </button>
              );
            })}
          </div>
        </>
      ) : (
        <>
          <div className="discover-results-header">
            {category && (
              <button className="btn-back" onClick={backToCategories}>
                <ChevronLeft size={14}/> {t("discover.back")}
              </button>
            )}
            <h1 className="view-title">
              {loading ? t("discover.loading")
                : category ? t(`category.${category.id}`)
                : `${results.length} ${t("discover.resultsFor")} "${query}"`}
            </h1>
          </div>
          {loading ? (
            <div className="discover-spinner"><Loader2 size={26} className="spin"/></div>
          ) : results.length > 0 ? (
            <div className="app-card-grid">
              {(results ?? []).map(r => (
                <DiscoverCard key={`${r.source}-${r.package_id}`} result={r}
                  installed={isDiscoverInstalled(r.source, r.package_id)}
                  busy={discoverBusyKey === `${r.source}::${r.package_id}`}
                  onInstall={onInstall} onUninstall={onUninstall} onOpen={onOpen} t={t}/>
              ))}
            </div>
          ) : searched && <p className="view-sub">{t("discover.noResults")}</p>}
        </>
      )}
    </div>
  );
}

// ─── App Detail Modal ─────────────────────────────────────────────────────────

function AppDetailModal({ item, onClose, settings, isInstalled, busy, onInstall, onUninstall, t }: {
  item: {package_id:string; source:string; name:string};
  onClose: () => void;
  settings: AppSettings;
  isInstalled: boolean;
  busy: boolean;
  onInstall: () => void;
  onUninstall: () => void;
  t: (k: string) => string;
}) {
  const [details, setDetails] = useState<AppDetails|null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    invoke<AppDetails>("get_app_details", {
      packageId: item.package_id, source: item.source, name: item.name,
    }).then(d => { if (!cancelled) setDetails(d); })
      .catch(() => {})
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [item.package_id, item.source, item.name]);

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="detail-modal" onClick={e => e.stopPropagation()}>
        <button className="detail-close" onClick={onClose}><X size={16}/></button>
        {loading ? (
          <div className="discover-spinner detail-loading"><Loader2 size={26} className="spin"/><span>{t("detail.loading")}</span></div>
        ) : (
          <>
            <div className="detail-header">
              <AppIcon icon={details?.icon} source={item.source} size={64}/>
              <div className="detail-header-body">
                <h1 className="detail-name">{details?.name ?? item.name}</h1>
                <span className="discover-source detail-source-badge"
                  style={{borderColor:sourceColor(item.source),color:sourceColor(item.source)}}>
                  {t(`discover.source.${item.source}`)}
                </span>
                {details?.summary && <p className="detail-summary">{details.summary}</p>}
              </div>
              <div className="detail-header-actions">
                {busy ? (
                  <button className="btn btn-installing" disabled><Loader2 size={13} className="spin"/></button>
                ) : isInstalled ? (
                  <div className="install-done-wrap">
                    <button className="btn btn-installed" disabled><CheckCircle size={13}/> {t("btn.installed")}</button>
                    <button className="btn-uninstall" onClick={onUninstall} title={t("btn.uninstall")}><Trash2 size={13}/></button>
                  </div>
                ) : (
                  <button className="btn btn-install" onClick={onInstall}>{t("btn.install")}</button>
                )}
              </div>
            </div>

            {settings.ratings_enabled ? (
              details?.rating ? (
                <StarRating rating={details.rating} size={16}/>
              ) : item.source === "flatpak" ? (
                <div className="no-rating">{t("detail.noRating")}</div>
              ) : null
            ) : (
              <div className="no-rating">{t("detail.ratingsOff")}</div>
            )}

            <div className="detail-meta-grid">
              {details?.version && <div><span className="about-label">{t("detail.version")}</span><span>{details.version}</span></div>}
              {details?.size    && <div><span className="about-label">{t("detail.size")}</span><span>{details.size}</span></div>}
              {details?.license && <div><span className="about-label">{t("detail.license")}</span><span>{details.license}</span></div>}
              {details?.homepage && (
                <div>
                  <span className="about-label">{t("detail.homepage")}</span>
                  <a href={details.homepage} target="_blank" rel="noreferrer" className="detail-link">
                    {t("detail.homepage")} <ExternalLink size={11}/>
                  </a>
                </div>
              )}
            </div>

            <h2 className="section-heading">{t("detail.screenshots")}</h2>
            {details?.screenshots && details.screenshots.length > 0 ? (
              <div className="screenshot-gallery">
                {details.screenshots.map((s, i) => (
                  <img key={i} src={s} className="screenshot-img" alt=""
                    onError={e => { (e.target as HTMLImageElement).style.display = "none"; }}/>
                ))}
              </div>
            ) : (
              <div className="no-screenshots"><ImageOff size={18}/> {t("detail.noScreenshots")}</div>
            )}

            <h2 className="section-heading">{t("detail.description")}</h2>
            <p className="detail-description">{details?.description || details?.summary || t("info.noInfo")}</p>
          </>
        )}
      </div>
    </div>
  );
}

// ─── Pentest View ─────────────────────────────────────────────────────────────

const PENTEST_TAGS = [
  "all","network","web","password","wifi","mitm","exploit","osint",
  "forensics","reverse","ad","packet","audit","utility"
];

function PentestView({ tag, onTag, isInstalling, isUninstalling, isInstalled, getVersion, onInstall, onUninstall, t }: {
  tag: string; onTag: (t: string) => void;
  isInstalling: (p: Package) => boolean;
  isUninstalling: (p: Package) => boolean;
  isInstalled:  (p: Package) => boolean;
  getVersion:   (p: Package) => string|undefined;
  onInstall:    (p: Package) => void;
  onUninstall:  (p: Package) => void;
  t: (k: string) => string;
}) {
  const filtered = tag==="all" ? PENTEST_TOOLS : PENTEST_TOOLS.filter(p=>p.tags?.includes(tag));
  return (
    <div className="view">
      <h1 className="view-title">{t("nav.pentest_tools")}</h1>
      <p className="view-sub">{t("pentest.sub")}</p>
      <div className="tag-pills">
        {PENTEST_TAGS.map(tg => (
          <button key={tg} className={`tag-pill ${tag===tg?"active":""}`} onClick={()=>onTag(tg)}>{tg}</button>
        ))}
      </div>
      <div className="pentest-count">{filtered.length} tools</div>
      <div className="pkg-list">
        {filtered.map(pkg => (
          <PkgRow key={pkg.name} pkg={pkg} isInstalling={isInstalling} isUninstalling={isUninstalling}
            isInstalled={isInstalled} getVersion={getVersion}
            onInstall={onInstall} onUninstall={onUninstall} t={t}/>
        ))}
      </div>
    </div>
  );
}

// ─── Search Results (curated sections local search) ───────────────────────────

function SearchResults({ results, query, isInstalling, isUninstalling, isInstalled, getVersion, onInstall, onUninstall, t }: {
  results: Package[]; query: string;
  isInstalling: (p: Package) => boolean;
  isUninstalling: (p: Package) => boolean;
  isInstalled:  (p: Package) => boolean;
  getVersion:   (p: Package) => string|undefined;
  onInstall:    (p: Package) => void;
  onUninstall:  (p: Package) => void;
  t: (k: string) => string;
}) {
  return (
    <div className="view">
      <h1 className="view-title">
        {results.length>0
          ? `${results.length} result${results.length!==1?"s":""} for "${query}"`
          : `No results for "${query}"`}
      </h1>
      {results.length===0 && <p className="view-sub">Try a different search term, or search Discover for other apps.</p>}
      <div className="pkg-list">
        {results.map(pkg => (
          <PkgRow key={`${pkg.category}-${pkg.name}`} pkg={pkg}
            isInstalling={isInstalling} isUninstalling={isUninstalling} isInstalled={isInstalled}
            getVersion={getVersion} onInstall={onInstall} onUninstall={onUninstall} t={t}/>
        ))}
      </div>
    </div>
  );
}

// ─── Update View ──────────────────────────────────────────────────────────────

function UpdateView({ updating, onUpdate, progress, onShowLog, updatesAvailable, t }: {
  updating: boolean; onUpdate: () => void; progress: Progress|null; onShowLog: () => void;
  updatesAvailable: number|null;
  t: (k: string) => string;
}) {
  return (
    <div className="view update-view">
      <div className="update-card">
        <div className="update-icon">
          <RefreshCw size={40} className={updating?"spin":""}/>
        </div>
        <h1 className="update-title">{t("update.title")}</h1>
        {!!updatesAvailable && !updating && (
          <p className="view-sub">{updatesAvailable} {t("update.badge")}</p>
        )}
        {progress && updating && (
          <div className="update-progress">
            <div className="update-progress-bar">
              <div className="update-progress-fill" style={{width:`${Math.round(progress.progress*100)}%`}}/>
            </div>
            <div className="update-progress-msg">{progress.message}</div>
          </div>
        )}
        <button className={`btn btn-update ${updating?"disabled":""}`} onClick={onUpdate} disabled={updating}>
          {updating ? <><Loader2 size={16} className="spin"/> {t("btn.updating")}</> : <><RefreshCw size={16}/> {t("btn.updateNow")}</>}
        </button>
        {updating && (
          <button className="btn-show-log" onClick={onShowLog}>
            <Terminal size={13}/> {t("btn.viewLog")}
          </button>
        )}
      </div>
    </div>
  );
}

// ─── Settings View ────────────────────────────────────────────────────────────

function SettingsView({ settings, onSave, onClearCache, onReset, busy, appInfo, t }: {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
  onClearCache: () => void;
  onReset: () => void;
  busy: boolean;
  appInfo: AppInfo|null;
  t: (k: string) => string;
}) {
  const [draft, setDraft] = useState<AppSettings>(settings);
  useEffect(() => setDraft(settings), [settings]);

  const dirty = JSON.stringify(draft) !== JSON.stringify(settings);

  const toggleSource = (id: string) => {
    setDraft(d => ({
      ...d,
      enabled_sources: d.enabled_sources.includes(id)
        ? d.enabled_sources.filter(s => s !== id)
        : [...d.enabled_sources, id],
    }));
  };

  const SECTIONS = ["discover","game_launchers","pentest_tools","drivers","update"];

  return (
    <div className="view settings-view">
      <h1 className="view-title">{t("settings.title")}</h1>

      <section className="settings-section">
        <h2 className="settings-heading">{t("settings.language")}</h2>
        <div className="lang-pills">
          {LANGUAGES.map(l => (
            <button key={l.id} className={`tag-pill ${draft.language===l.id?"active":""}`}
              onClick={() => setDraft(d => ({...d, language: l.id}))}>
              {l.label}
            </button>
          ))}
        </div>
      </section>

      <section className="settings-section">
        <h2 className="settings-heading">{t("settings.sources")}</h2>
        <p className="settings-hint">{t("settings.sourcesHint")}</p>
        <div className="source-toggle-grid">
          {SOURCES.map(s => (
            <label key={s.id} className="source-toggle">
              <input type="checkbox" checked={draft.enabled_sources.includes(s.id)}
                onChange={() => toggleSource(s.id)}/>
              <SourceIcon source={s.id} size={15}/>
              <span>{s.label}</span>
            </label>
          ))}
        </div>
      </section>

      <section className="settings-section">
        <h2 className="settings-heading">{t("settings.ratings")}</h2>
        <label className="settings-checkbox">
          <input type="checkbox" checked={draft.ratings_enabled}
            onChange={e => setDraft(d => ({...d, ratings_enabled: e.target.checked}))}/>
          {t("settings.ratingsToggle")}
        </label>
        <p className="settings-hint">{t("settings.ratingsHint")}</p>
      </section>

      <section className="settings-section">
        <h2 className="settings-heading">{t("settings.defaultSection")}</h2>
        <select className="settings-input" value={draft.default_section}
          onChange={e => setDraft(d => ({...d, default_section: e.target.value}))}>
          {SECTIONS.map(s => <option key={s} value={s}>{t(`nav.${s}`) || t("discover.title")}</option>)}
        </select>
      </section>

      <section className="settings-section">
        <h2 className="settings-heading">{t("settings.mirrors")}</h2>
        <label className="settings-label">{t("settings.flatpakRemote")}</label>
        <input className="settings-input" value={draft.flatpak_remote_url}
          onChange={e => setDraft(d => ({...d, flatpak_remote_url: e.target.value}))}/>
        <label className="settings-label">{t("settings.aptMirror")}</label>
        <input className="settings-input" placeholder="deb.debian.org" value={draft.apt_mirror}
          onChange={e => setDraft(d => ({...d, apt_mirror: e.target.value}))}/>
        <p className="settings-hint">{t("settings.aptMirrorHint")}</p>
        <label className="settings-checkbox">
          <input type="checkbox" checked={draft.check_updates_on_startup}
            onChange={e => setDraft(d => ({...d, check_updates_on_startup: e.target.checked}))}/>
          {t("settings.startupUpdates")}
        </label>
      </section>

      <button className="btn btn-install settings-save-btn" disabled={!dirty} onClick={() => onSave(draft)}>
        {t("btn.save")}
      </button>

      <section className="settings-section">
        <h2 className="settings-heading">{t("settings.maintenance")}</h2>
        <p className="settings-hint">{t("settings.clearCacheHint")}</p>
        <button className="btn btn-uninstall-wide" disabled={busy} onClick={onClearCache}>
          <Trash2 size={14}/> {busy ? t("btn.clearing") : t("btn.clearCache")}
        </button>
        <p className="settings-hint" style={{marginTop:14}}>{t("settings.reset")}</p>
        <button className="btn btn-uninstall-wide" onClick={onReset}>
          <RefreshCw size={14}/> {t("settings.reset")}
        </button>
      </section>

      <section className="settings-section">
        <h2 className="settings-heading">{t("settings.about")}</h2>
        <div className="about-grid">
          <div><span className="about-label">{t("settings.version")}</span><span>{appInfo?.version ?? "—"}</span></div>
          <div><span className="about-label">{t("settings.targetRelease")}</span><span>{appInfo?.target_release ?? "—"}</span></div>
        </div>
      </section>
    </div>
  );
}
