import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Search, Gamepad2, Shield, ShoppingBag, Cpu, RefreshCw,
  X, CheckCircle, AlertCircle, Loader2, ChevronRight, Star,
  Zap, Lock, Palette, Wrench, MessageCircle, Code, Music,
  Terminal, Info
} from "lucide-react";
import {
  ALL_PACKAGES, GAME_LAUNCHERS, PENTEST_TOOLS, STORE_PACKAGES,
  DRIVERS, Package, StoreSection, Category
} from "./data/packages";
import "./App.css";

// ─── Types ────────────────────────────────────────────────────────────────────

interface Progress { step: string; message: string; progress: number; }
interface LogLine  { stream: "stdout"|"stderr"|"info"|"error"|"success"; line: string; }
interface ToastItem { id: number; type: "success"|"error"|"info"; message: string; }
interface InstalledState { key: string; installed: boolean; version?: string; }
interface DiscoverResult {
  name: string; version: string; desc: string;
  source: "apt"|"flatpak"|"snap"|"brew"; package_id: string; size?: string;
}
type InstalledMap  = Record<string, { installed: boolean; version?: string }>;
type InstallingMap = Record<string, boolean>;

// ─── Terminal log panel ───────────────────────────────────────────────────────

function TerminalLog({ lines, onClose, title, active }: {
  lines: LogLine[]; onClose: () => void; title: string; active: boolean;
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

// ─── Constants ────────────────────────────────────────────────────────────────

const NAV = [
  { id:"store"          as Category, label:"Discover",      icon:ShoppingBag },
  { id:"game_launchers" as Category, label:"Game Launchers", icon:Gamepad2    },
  { id:"pentest_tools"  as Category, label:"Pentest Tools",  icon:Shield      },
  { id:"drivers"        as Category, label:"Drivers",        icon:Cpu         },
  { id:"update"         as Category, label:"Update System",  icon:RefreshCw   },
];

const STORE_SECTIONS: { id: StoreSection; label: string; Icon: typeof Star }[] = [
  { id:"featured",      label:"Featured",     Icon:Star         },
  { id:"productivity",  label:"Productivity", Icon:Zap          },
  { id:"development",   label:"Development",  Icon:Code         },
  { id:"media",         label:"Media",        Icon:Music        },
  { id:"communication", label:"Messaging",    Icon:MessageCircle},
  { id:"security",      label:"Security",     Icon:Lock         },
  { id:"graphics",      label:"Graphics",     Icon:Palette      },
  { id:"utilities",     label:"Utilities",    Icon:Wrench       },
];

const PENTEST_TAGS = [
  "all","network","web","password","wifi","mitm","exploit","osint",
  "forensics","reverse","ad","packet","utility"
];

// ─── App ──────────────────────────────────────────────────────────────────────

export default function App() {
  const [active, setActive]             = useState<Category>("store");
  const [search, setSearch]             = useState("");
  const [installing, setInstalling]     = useState<InstallingMap>({});
  const [installed, setInstalled]       = useState<InstalledMap>({});
  const [toasts, setToasts]             = useState<ToastItem[]>([]);
  const [storeSection, setStoreSection] = useState<StoreSection>("featured");
  const [discoverSearch, setDiscoverSearch] = useState("");
  const [pentestTag, setPentestTag]     = useState("all");
  const [updating, setUpdating]         = useState(false);
  const [logLines, setLogLines]         = useState<LogLine[]>([]);
  const [logTitle, setLogTitle]         = useState("");
  const [showLog, setShowLog]           = useState(false);
  const [logActive, setLogActive]       = useState(false);
  const [progress, setProgress]         = useState<Progress|null>(null);
  const toastId  = useRef(0);
  const searchRef = useRef<HTMLInputElement>(null);

  // ── Load installed state on startup ──────────────────────────────────────
  useEffect(() => {
    invoke<InstalledState[]>("check_all_installed").then(states => {
      const map: InstalledMap = {};
      states.forEach(s => { map[s.key] = { installed: s.installed, version: s.version }; });
      setInstalled(map);
    }).catch(() => {});
  }, []);

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
      if (e.key==="Escape") { setSearch(""); setShowLog(false); }
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, []);

  const addToast = useCallback((type: ToastItem["type"], message: string) => {
    const id = ++toastId.current;
    setToasts(t => [...t, { id, type, message }]);
    setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 5000);
  }, []);

  const handleInstall = useCallback(async (pkg: Package) => {
    const key = `${pkg.category}::${pkg.name}`;
    setInstalling(m => ({...m, [key]: true}));
    setLogLines([]);
    setLogTitle(`Installing ${pkg.name}`);
    setShowLog(true);
    setLogActive(true);
    setProgress({step:"start", message:"Starting…", progress:0});
    try {
      await invoke("install_package", { name: pkg.name, category: pkg.category });
      setInstalled(m => ({...m, [key]: {installed:true}}));
      addToast("success", `${pkg.name} installed successfully.`);
    } catch (err) {
      addToast("error", `Failed: ${err}`);
    } finally {
      setInstalling(m => ({...m, [key]: false}));
      setLogActive(false);
      setTimeout(() => setProgress(null), 1200);
    }
  }, [addToast]);

  const handleUpdate = useCallback(async () => {
    setUpdating(true);
    setLogLines([]);
    setLogTitle("System Update");
    setShowLog(true);
    setLogActive(true);
    setProgress({step:"update", message:"Updating…", progress:0.05});
    try {
      await invoke("update_system");
      addToast("success", "System updated!");
    } catch (err) {
      addToast("error", `Update failed: ${err}`);
    } finally {
      setUpdating(false);
      setLogActive(false);
      setTimeout(() => setProgress(null), 1200);
    }
  }, [addToast]);

  const isInstalling = (pkg: Package) => installing[`${pkg.category}::${pkg.name}`] ?? false;
  const isInstalled  = (pkg: Package) => installed[`${pkg.category}::${pkg.name}`]?.installed ?? false;
  const getVersion   = (pkg: Package) => installed[`${pkg.category}::${pkg.name}`]?.version;

  const searchResults = search.trim().length > 1
    ? ALL_PACKAGES.filter(p =>
        p.name.toLowerCase().includes(search.toLowerCase()) ||
        p.desc.toLowerCase().includes(search.toLowerCase()) ||
        (p.tags??[]).some(t => t.includes(search.toLowerCase()))
      ) : [];

  const sharedProps = { isInstalling, isInstalled, getVersion, onInstall: handleInstall };

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
          <input ref={searchRef} className="search-input" placeholder="Search… (Ctrl+F)"
            value={search} onChange={e => setSearch(e.target.value)}/>
          {search && <button className="search-clear" onClick={() => setSearch("")}><X size={12}/></button>}
        </div>

        <nav className="nav">
          {NAV.map(({id, label, icon:Icon}) => (
            <button key={id} className={`nav-item ${active===id && !search ? "active" : ""}`}
              onClick={() => { setActive(id); setSearch(""); }}>
              <Icon size={16} className="nav-icon"/>
              <span>{label}</span>
              {active===id && !search && <ChevronRight size={12} className="nav-arrow"/>}
            </button>
          ))}
        </nav>

        {logLines.length > 0 && (
          <button className="log-toggle" onClick={() => setShowLog(v=>!v)}>
            <Terminal size={13}/>
            <span>{logActive ? "Installing…" : "View log"}</span>
            {logActive && <Loader2 size={11} className="spin"/>}
          </button>
        )}

        <div className="sidebar-footer">v0.8 · Debian</div>
      </aside>

      {/* Main */}
      <main className="main">
        {search.trim().length > 1 ? (
          <SearchResults results={searchResults} query={search} {...sharedProps}/>
        ) : active==="store" ? (
          <StoreView section={storeSection} onSection={setStoreSection}
            searchQuery={discoverSearch} onSearchChange={setDiscoverSearch} {...sharedProps}/>
        ) : active==="game_launchers" ? (
          <PackageList title="Game Launchers" packages={GAME_LAUNCHERS} {...sharedProps}/>
        ) : active==="pentest_tools" ? (
          <PentestView tag={pentestTag} onTag={setPentestTag} {...sharedProps}/>
        ) : active==="drivers" ? (
          <PackageList title="Drivers & Firmware" packages={DRIVERS} {...sharedProps}/>
        ) : active==="update" ? (
          <UpdateView updating={updating} onUpdate={handleUpdate}
            progress={progress} onShowLog={() => setShowLog(true)}/>
        ) : null}
      </main>

      {/* Global progress bar */}
      {progress && (
        <div className="progress-bar-global">
          <div className="progress-bar-fill" style={{width:`${Math.round(progress.progress*100)}%`}}/>
        </div>
      )}

      {/* Terminal modal */}
      {showLog && (
        <TerminalLog lines={logLines} onClose={() => setShowLog(false)}
          title={logTitle} active={logActive}/>
      )}

      {/* Toasts */}
      <div className="toast-stack">
        {toasts.map(t => (
          <div key={t.id} className={`toast toast-${t.type}`}>
            {t.type==="success" && <CheckCircle size={15}/>}
            {t.type==="error"   && <AlertCircle size={15}/>}
            {t.type==="info"    && <Info size={15}/>}
            <span>{t.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ─── Install Button ───────────────────────────────────────────────────────────

function InstallBtn({ pkg, installing, installed, version, onInstall }: {
  pkg: Package; installing: boolean; installed: boolean;
  version?: string; onInstall: (p: Package) => void;
}) {
  if (installed) return (
    <div className="install-done-wrap">
      <button className="btn btn-installed" disabled>✓ Installed</button>
      {version && <span className="installed-version">{version}</span>}
    </div>
  );
  if (installing) return (
    <button className="btn btn-installing" disabled>
      <Loader2 size={13} className="spin"/> Installing…
    </button>
  );
  return <button className="btn btn-install" onClick={() => onInstall(pkg)}>Install</button>;
}

// ─── Package Row ──────────────────────────────────────────────────────────────

function PkgRow({ pkg, isInstalling, isInstalled, getVersion, onInstall }: {
  pkg: Package;
  isInstalling: (p: Package) => boolean;
  isInstalled:  (p: Package) => boolean;
  getVersion:   (p: Package) => string|undefined;
  onInstall:    (p: Package) => void;
}) {
  const [info, setInfo]     = useState<{size?:string;version?:string}|null>(null);
  const [showInfo, setShowInfo] = useState(false);

  const loadInfo = async () => {
    if (info) { setShowInfo(v=>!v); return; }
    try {
      const r = await invoke<{size:string|null;version:string|null}>(
        "get_package_info", { name: pkg.name, category: pkg.category }
      );
      setInfo({ size: r.size??undefined, version: r.version??undefined });
      setShowInfo(true);
    } catch { setInfo({}); setShowInfo(true); }
  };

  return (
    <div className={`pkg-row ${isInstalled(pkg) ? "pkg-row--installed" : ""}`}>
      <div className="pkg-row-icon">{pkg.icon}</div>
      <div className="pkg-row-body">
        <div className="pkg-row-name">
          {pkg.name}
          {isInstalled(pkg) && <span className="row-installed-badge">installed</span>}
        </div>
        <div className="pkg-row-desc">{pkg.desc}</div>
        {pkg.tags && (
          <div className="pkg-row-tags">
            {pkg.tags.slice(0,4).map(t => <span key={t} className="tag">{t}</span>)}
          </div>
        )}
        {showInfo && info && (
          <div className="pkg-info-line">
            {info.version && <span>Latest: {info.version}</span>}
            {info.size    && <span>Size: {info.size}</span>}
            {!info.version && !info.size && <span>No info available</span>}
          </div>
        )}
      </div>
      <div className="pkg-row-actions">
        <button className="btn-info" onClick={loadInfo} title="Package info"><Info size={14}/></button>
        <InstallBtn pkg={pkg} installing={isInstalling(pkg)} installed={isInstalled(pkg)}
          version={getVersion(pkg)} onInstall={onInstall}/>
      </div>
    </div>
  );
}

// ─── Package List ─────────────────────────────────────────────────────────────

function PackageList({ title, packages, isInstalling, isInstalled, getVersion, onInstall }: {
  title: string; packages: Package[];
  isInstalling: (p: Package) => boolean;
  isInstalled:  (p: Package) => boolean;
  getVersion:   (p: Package) => string|undefined;
  onInstall:    (p: Package) => void;
}) {
  return (
    <div className="view">
      <h1 className="view-title">{title}</h1>
      <div className="pkg-list">
        {packages.map(pkg => (
          <PkgRow key={pkg.name} pkg={pkg} isInstalling={isInstalling}
            isInstalled={isInstalled} getVersion={getVersion} onInstall={onInstall}/>
        ))}
      </div>
    </div>
  );
}

// ─── Discover row ─────────────────────────────────────────────────────────────

function sourceColor(s: DiscoverResult["source"]) {
  return {apt:"#f97316",flatpak:"#3b82f6",snap:"#e11d48",brew:"#84cc16"}[s]??"#8e8e93";
}

function DiscoverRow({ result }: { result: DiscoverResult }) {
  const [state, setState] = useState<"idle"|"installing"|"done"|"error">("idle");
  const [err, setErr]     = useState("");
  const icons = {apt:"📦",flatpak:"📱",snap:"🔵",brew:"🍺"};

  const handleInstall = async () => {
    setState("installing");
    try {
      await invoke("discover_install", { packageId: result.package_id, source: result.source });
      setState("done");
    } catch(e) { setErr(String(e)); setState("error"); }
  };

  return (
    <div className="pkg-row">
      <div className="pkg-row-icon">{icons[result.source]??"📦"}</div>
      <div className="pkg-row-body">
        <div className="pkg-row-name">
          {result.name}
          <span className="discover-source"
            style={{borderColor:sourceColor(result.source),color:sourceColor(result.source)}}>
            {result.source}
          </span>
          {result.version && <span className="discover-version">{result.version}</span>}
          {result.size    && <span className="discover-version">{result.size}</span>}
        </div>
        <div className="pkg-row-desc">{result.desc||"No description available."}</div>
        {state==="error" && <div className="discover-err">{err}</div>}
      </div>
      <div className="pkg-row-actions">
        {state==="done"
          ? <button className="btn btn-installed" disabled>✓ Installed</button>
          : state==="installing"
          ? <button className="btn btn-installing" disabled><Loader2 size={13} className="spin"/>Installing…</button>
          : <button className="btn btn-install" onClick={handleInstall}>Install</button>
        }
      </div>
    </div>
  );
}

// ─── Store / Discover View ────────────────────────────────────────────────────

function StoreView({ section, onSection, isInstalling, isInstalled, getVersion, onInstall, searchQuery, onSearchChange }: {
  section: StoreSection; onSection: (s: StoreSection) => void;
  isInstalling: (p: Package) => boolean;
  isInstalled:  (p: Package) => boolean;
  getVersion:   (p: Package) => string|undefined;
  onInstall:    (p: Package) => void;
  searchQuery: string; onSearchChange: (q: string) => void;
}) {
  const [results, setResults]   = useState<DiscoverResult[]>([]);
  const [loading, setLoading]   = useState(false);
  const [searched, setSearched] = useState(false);
  const debRef = useRef<ReturnType<typeof setTimeout>|null>(null);

  useEffect(() => {
    if (debRef.current) clearTimeout(debRef.current);
    if (searchQuery.trim().length < 2) { setResults([]); setSearched(false); return; }
    debRef.current = setTimeout(async () => {
      setLoading(true); setSearched(true);
      try {
        const r = await invoke<DiscoverResult[]>("discover_search",{query:searchQuery.trim()});
        setResults(r);
      } catch { setResults([]); } finally { setLoading(false); }
    }, 420);
  }, [searchQuery]);

  const pkgs     = STORE_PACKAGES.filter(p => p.section===section);
  const featured = STORE_PACKAGES.filter(p => p.section==="featured");

  if (searchQuery.trim().length >= 2) {
    return (
      <div className="view">
        <div className="section-pills">
          {STORE_SECTIONS.map(({id,label,Icon}) => (
            <button key={id} className={`section-pill ${section===id?"active":""}`}
              onClick={() => { onSection(id); onSearchChange(""); }}>
              <Icon size={13}/>{label}
            </button>
          ))}
        </div>
        <h1 className="view-title">
          {loading ? "Searching all sources…"
            : `${results.length} result${results.length!==1?"s":""} for "${searchQuery}"`}
        </h1>
        {loading
          ? <div className="discover-spinner"><Loader2 size={26} className="spin"/></div>
          : results.length>0
            ? <div className="pkg-list">{results.map(r =>
                <DiscoverRow key={`${r.source}-${r.package_id}`} result={r}/>)}</div>
            : searched && <p className="view-sub">No packages found. Try a different term.</p>
        }
      </div>
    );
  }

  return (
    <div className="view">
      {section==="featured" && (
        <div className="store-hero">
          <div className="store-hero-badge">Discover</div>
          <h1 className="store-hero-title">Find great software</h1>
          <p className="store-hero-sub">Search apt, Flatpak, Snap and Homebrew in one place.</p>
          <div className="discover-search-wrap">
            <Search size={15} className="discover-search-icon"/>
            <input className="discover-search-input" placeholder="Search all package sources…"
              value={searchQuery} onChange={e => onSearchChange(e.target.value)}/>
            {searchQuery && <button className="search-clear" onClick={()=>onSearchChange("")}><X size={13}/></button>}
          </div>
        </div>
      )}
      <div className="section-pills">
        {STORE_SECTIONS.map(({id,label,Icon}) => (
          <button key={id} className={`section-pill ${section===id?"active":""}`} onClick={()=>onSection(id)}>
            <Icon size={13}/>{label}
          </button>
        ))}
      </div>
      {section==="featured" ? (
        <>
          <div className="store-cards">
            {featured.slice(0,3).map(pkg => (
              <div key={pkg.name} className="store-card">
                <div className="store-card-emoji">{pkg.icon}</div>
                <div className="store-card-name">{pkg.name}</div>
                <div className="store-card-desc">{pkg.desc}</div>
                <InstallBtn pkg={pkg} installing={isInstalling(pkg)} installed={isInstalled(pkg)}
                  version={getVersion(pkg)} onInstall={onInstall}/>
              </div>
            ))}
          </div>
          <h2 className="section-heading">All Featured</h2>
          <div className="pkg-list">
            {featured.map(pkg => (
              <PkgRow key={pkg.name} pkg={pkg} isInstalling={isInstalling}
                isInstalled={isInstalled} getVersion={getVersion} onInstall={onInstall}/>
            ))}
          </div>
        </>
      ) : (
        <div className="pkg-list">
          {pkgs.map(pkg => (
            <PkgRow key={pkg.name} pkg={pkg} isInstalling={isInstalling}
              isInstalled={isInstalled} getVersion={getVersion} onInstall={onInstall}/>
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Pentest View ─────────────────────────────────────────────────────────────

function PentestView({ tag, onTag, isInstalling, isInstalled, getVersion, onInstall }: {
  tag: string; onTag: (t: string) => void;
  isInstalling: (p: Package) => boolean;
  isInstalled:  (p: Package) => boolean;
  getVersion:   (p: Package) => string|undefined;
  onInstall:    (p: Package) => void;
}) {
  const filtered = tag==="all" ? PENTEST_TOOLS : PENTEST_TOOLS.filter(p=>p.tags?.includes(tag));
  return (
    <div className="view">
      <h1 className="view-title">Pentest Tools</h1>
      <p className="view-sub">
        Debian-repo tools install via apt directly.
        Others use a <strong>Kali Linux distrobox</strong> container (created once, ~5 min first run).
      </p>
      <div className="tag-pills">
        {PENTEST_TAGS.map(t => (
          <button key={t} className={`tag-pill ${tag===t?"active":""}`} onClick={()=>onTag(t)}>{t}</button>
        ))}
      </div>
      <div className="pentest-count">{filtered.length} tools</div>
      <div className="pkg-list">
        {filtered.map(pkg => (
          <PkgRow key={pkg.name} pkg={pkg} isInstalling={isInstalling}
            isInstalled={isInstalled} getVersion={getVersion} onInstall={onInstall}/>
        ))}
      </div>
    </div>
  );
}

// ─── Search Results ───────────────────────────────────────────────────────────

function SearchResults({ results, query, isInstalling, isInstalled, getVersion, onInstall }: {
  results: Package[]; query: string;
  isInstalling: (p: Package) => boolean;
  isInstalled:  (p: Package) => boolean;
  getVersion:   (p: Package) => string|undefined;
  onInstall:    (p: Package) => void;
}) {
  return (
    <div className="view">
      <h1 className="view-title">
        {results.length>0
          ? `${results.length} result${results.length!==1?"s":""} for "${query}"`
          : `No results for "${query}"`}
      </h1>
      {results.length===0 && <p className="view-sub">Try a different search term.</p>}
      <div className="pkg-list">
        {results.map(pkg => (
          <PkgRow key={`${pkg.category}-${pkg.name}`} pkg={pkg}
            isInstalling={isInstalling} isInstalled={isInstalled}
            getVersion={getVersion} onInstall={onInstall}/>
        ))}
      </div>
    </div>
  );
}

// ─── Update View ──────────────────────────────────────────────────────────────

function UpdateView({ updating, onUpdate, progress, onShowLog }: {
  updating: boolean; onUpdate: () => void; progress: Progress|null; onShowLog: () => void;
}) {
  return (
    <div className="view update-view">
      <div className="update-card">
        <div className="update-icon">
          <RefreshCw size={40} className={updating?"spin":""}/>
        </div>
        <h1 className="update-title">Update System</h1>
        {progress && updating && (
          <div className="update-progress">
            <div className="update-progress-bar">
              <div className="update-progress-fill" style={{width:`${Math.round(progress.progress*100)}%`}}/>
            </div>
            <div className="update-progress-msg">{progress.message}</div>
          </div>
        )}
        <button className={`btn btn-update ${updating?"disabled":""}`} onClick={onUpdate} disabled={updating}>
          {updating ? <><Loader2 size={16} className="spin"/> Updating…</> : <><RefreshCw size={16}/> Update Now</>}
        </button>
        {updating && (
          <button className="btn-show-log" onClick={onShowLog}>
            <Terminal size={13}/> View terminal log
          </button>
        )}
      </div>
    </div>
  );
}
