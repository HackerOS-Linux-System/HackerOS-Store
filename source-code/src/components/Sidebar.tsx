import { For, Show } from "solid-js";
import { Search, X, ChevronRight, Terminal, Loader2, Sun, Moon, Monitor } from "lucide-solid";
import type { Category } from "../data/packages";
import type { AppInfo, QueueJob } from "../types";
import { QueuePanel } from "./QueuePanel";
import { useI18n } from "../hooks/useI18n";
import type { IconComponent } from "../iconMap";

export interface NavItem { id: Category; label: string; icon: IconComponent; badge?: number; }

/** Cycles dark -> light -> system -> dark on each click of the sidebar's
 * quick-toggle button (see `sidebar-footer` below). Kept as a plain
 * function (not inline) since `SettingsView`'s theme pills need the same
 * three-way set of values and this keeps the cycle order in one place. */
function nextTheme(current: string): string {
  if (current === "dark") return "light";
  if (current === "light") return "system";
  return "dark";
}

export function Sidebar(props: {
  navItems: NavItem[];
  active: Category;
  search: string;
  onSearch: (v: string) => void;
  onNav: (id: Category) => void;
  logCount: number;
  logActive: boolean;
  showLog: boolean;
  onToggleLog: () => void;
  appInfo: AppInfo | null;
  queueJobs: QueueJob[];
  onDequeue: (id: string) => void;
  onReorderQueue: (id: string, direction: "up" | "down") => void;
  searchInputRef?: (el: HTMLInputElement) => void;
  theme: string;
  onSetTheme: (t: string) => void;
}) {
  const { t } = useI18n();
  const ThemeIcon = () => (props.theme === "light" ? Sun : props.theme === "system" ? Monitor : Moon);
  return (
    <aside class="sidebar">
      <div class="sidebar-logo">
        <Terminal size={20} class="logo-icon" />
        <span class="logo-text">HackerOS Store</span>
      </div>

      <div class="search-wrap">
        <Search size={14} class="search-icon" />
        <input ref={props.searchInputRef} class="search-input" placeholder={t("search.placeholder")}
          value={props.search} onInput={e => props.onSearch(e.currentTarget.value)} />
        <Show when={props.search}>
          <button class="search-clear" onClick={() => props.onSearch("")} aria-label={t("a11y.closeSearch")}><X size={12} /></button>
        </Show>
      </div>

      <nav class="nav" aria-label={t("a11y.mainNavigation")}>
        <For each={props.navItems}>
          {({ id, label, icon: Icon, badge }) => (
            <button class={`nav-item ${props.active === id && !props.search ? "active" : ""}`}
              aria-current={props.active === id && !props.search ? "page" : undefined}
              onClick={() => { props.onNav(id); props.onSearch(""); }}>
              <Icon size={16} class="nav-icon" />
              <span>{label}</span>
              <Show when={!!badge}><span class="nav-badge">{badge}</span></Show>
              <Show when={props.active === id && !props.search}><ChevronRight size={12} class="nav-arrow" /></Show>
            </button>
          )}
        </For>
      </nav>

      <QueuePanel jobs={props.queueJobs} onDequeue={props.onDequeue} onReorder={props.onReorderQueue} />

      <Show when={props.logCount > 0}>
        <button class="log-toggle" onClick={props.onToggleLog}>
          <Terminal size={13} />
          <span>{props.logActive ? "…" : t("btn.viewLog")}</span>
          <Show when={props.logActive}><Loader2 size={11} class="spin" /></Show>
        </button>
      </Show>

      <button class="theme-toggle" onClick={() => props.onSetTheme(nextTheme(props.theme))}
        title={t(`settings.theme.${props.theme}`)} aria-label={t("settings.theme")}>
        {(() => { const Icon = ThemeIcon(); return <Icon size={13} />; })()}
        <span>{t(`settings.theme.${props.theme}`)}</span>
      </button>

      <div class="sidebar-footer">
        v{props.appInfo?.version ?? "0.7.0"} · {props.appInfo?.target_release?.split(" ")[0] ?? "Debian"}
      </div>
    </aside>
  );
}
