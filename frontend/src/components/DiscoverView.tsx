import { createSignal, createEffect, onCleanup, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Search, X, Loader2, ChevronLeft, AlertCircle, AlertTriangle, Layers } from "lucide-solid";
import type { AppSettings, CategoryDef, DiscoverResult, DiscoverItem, DiscoverResponse, SourceIssue } from "../types";
import { ICONS } from "../iconMap";
import { DiscoverCard } from "./DiscoverCard";
import { useI18n } from "../hooks/useI18n";
import { useOnlineStatus } from "../hooks/useOnlineStatus";

export function DiscoverView(props: {
  settings: AppSettings;
  isDiscoverInstalled: (source: string, packageId: string) => boolean;
  discoverBusyKey: string | null;
  onInstall: (i: DiscoverItem) => void;
  onUninstall: (i: DiscoverItem) => void;
  onOpen: (i: DiscoverItem) => void;
}) {
  const { t } = useI18n();
  const online = useOnlineStatus();
  const [categories, setCategories] = createSignal<CategoryDef[]>([]);
  const [category, setCategory] = createSignal<CategoryDef | null>(null);
  const [query, setQuery] = createSignal("");
  const [results, setResults] = createSignal<DiscoverResult[]>([]);
  const [issues, setIssues] = createSignal<SourceIssue[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [searched, setSearched] = createSignal(false);
  const [failed, setFailed] = createSignal(false);

  invoke<CategoryDef[]>("discover_categories").then(setCategories).catch(() => {});

  let debounceHandle: ReturnType<typeof setTimeout> | undefined;
  createEffect(() => {
    const q = query();
    if (debounceHandle) clearTimeout(debounceHandle);
    if (q.trim().length < 2) {
      if (!category()) { setResults([]); setIssues([]); setSearched(false); setFailed(false); }
      return;
    }
    debounceHandle = setTimeout(async () => {
      setLoading(true); setSearched(true); setFailed(false);
      try {
        const resp = await invoke<DiscoverResponse>("discover_search", { query: q.trim() });
        setResults(resp.results);
        setIssues(resp.issues);
      } catch {
        setResults([]);
        setIssues([]);
        setFailed(true);
      } finally {
        setLoading(false);
      }
    }, 420);
  });
  onCleanup(() => { if (debounceHandle) clearTimeout(debounceHandle); });

  const openCategory = async (c: CategoryDef) => {
    setCategory(c);
    setQuery("");
    setLoading(true);
    setSearched(true);
    setFailed(false);
    try {
      const resp = await invoke<DiscoverResponse>("discover_browse", { categoryId: c.id });
      setResults(resp.results);
      setIssues(resp.issues);
    } catch {
      setResults([]);
      setIssues([]);
      setFailed(true);
    } finally {
      setLoading(false);
    }
  };

  const backToCategories = () => { setCategory(null); setResults([]); setIssues([]); setSearched(false); setQuery(""); setFailed(false); };

  const noSourcesEnabled = () => (props.settings?.enabled_sources?.length ?? 0) === 0;
  const showingResults = () => query().trim().length >= 2 || category() !== null;
  // The old binary "all sources failed" case is really just this: every
  // *enabled* source reported an issue and none returned anything —
  // distinct from "some sources are having trouble but others answered
  // fine", which gets the softer inline banner below instead.
  const allSourcesFailed = () => {
    const enabled = props.settings?.enabled_sources?.length ?? 0;
    return enabled > 0 && issues().length >= enabled && results().length === 0;
  };

  return (
    <div class="view">
      <div class="discover-hero">
        <div class="store-hero-badge">{t("discover.title")}</div>
        <h1 class="store-hero-title">{t("store.hero.title")}</h1>
        <p class="store-hero-sub">{t("discover.sub")}</p>
        <div class="discover-search-wrap">
          <Search size={15} class="discover-search-icon" />
          <input class="discover-search-input" placeholder={t("discover.searchPlaceholder")} aria-label={t("discover.searchPlaceholder")}
            value={query()} onInput={e => { setQuery(e.currentTarget.value); if (category()) setCategory(null); }} />
          <Show when={query()}>
            <button class="search-clear" onClick={() => setQuery("")} aria-label={t("a11y.closeSearch")}><X size={13} /></button>
          </Show>
        </div>
      </div>

      <Show when={!online()}>
        <p class="view-sub discover-warning"><AlertCircle size={14} /> {t("discover.offline")}</p>
      </Show>

      <Show when={online() && noSourcesEnabled()}>
        <p class="view-sub discover-warning"><AlertCircle size={14} /> {t("discover.noSourcesEnabled")}</p>
      </Show>

      <Show
        when={showingResults()}
        fallback={
          <>
            <h2 class="section-heading">{t("discover.categories")}</h2>
            <div class="category-grid">
              <For each={categories()}>
                {c => {
                  const Icon = ICONS[c.icon] ?? Layers;
                  return (
                    <button class="category-card" onClick={() => openCategory(c)}>
                      <div class="category-card-icon"><Icon size={26} /></div>
                      <span>{t(`category.${c.id}`)}</span>
                    </button>
                  );
                }}
              </For>
            </div>
          </>
        }
      >
        <div class="discover-results-header">
          <Show when={category()}>
            <button class="btn-back" onClick={backToCategories}>
              <ChevronLeft size={14} /> {t("discover.back")}
            </button>
          </Show>
          <h1 class="view-title">
            {loading() ? t("discover.loading")
              : category() ? t(`category.${category()!.id}`)
              : `${results().length} ${t("discover.resultsFor")} "${query()}"`}
          </h1>
        </div>
        <Show
          when={!loading()}
          fallback={<div class="discover-spinner"><Loader2 size={26} class="spin" /></div>}
        >
          <Show when={issues().length > 0 && !allSourcesFailed()}>
            <p class="view-sub discover-warning">
              <AlertTriangle size={14} />
              {issues().map(i => i.message).join(" ")}
            </p>
          </Show>
          <Show
            when={results().length > 0}
            fallback={
              <Show when={searched()}>
                <p class="view-sub">
                  {failed() || allSourcesFailed() ? t("discover.allSourcesFailed") : t("discover.noResults")}
                </p>
                <Show when={allSourcesFailed()}>
                  <p class="view-sub discover-warning">
                    {issues().map(i => i.message).join(" ")}
                  </p>
                </Show>
              </Show>
            }
          >
            <div class="app-card-grid">
              <For each={results()}>
                {r => (
                  <DiscoverCard result={r}
                    installed={props.isDiscoverInstalled(r.source, r.package_id)}
                    busy={props.discoverBusyKey === `${r.source}::${r.package_id}`}
                    onInstall={props.onInstall} onUninstall={props.onUninstall} onOpen={props.onOpen} />
                )}
              </For>
            </div>
          </Show>
        </Show>
      </Show>
    </div>
  );
}
