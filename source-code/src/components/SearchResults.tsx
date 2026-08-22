import { For, Show } from "solid-js";
import { Loader2 } from "lucide-solid";
import type { Package } from "../data/packages";
import type { DiscoverResult, DiscoverItem } from "../types";
import { PkgRow } from "./PkgRow";
import { DiscoverCard } from "./DiscoverCard";
import { useI18n } from "../hooks/useI18n";

export function SearchResults(props: {
  results: Package[]; query: string;
  isInstalling: (p: Package) => boolean;
  isUninstalling: (p: Package) => boolean;
  isInstalled: (p: Package) => boolean;
  getVersion: (p: Package) => string | undefined;
  onInstall: (p: Package) => void;
  onUninstall: (p: Package) => void;
  // Discover results for the same query — searching used to only cover
  // the curated sections (game launchers, pentest tools, drivers,
  // HackerOS Ecosystem), leaving Discover's much larger catalog (apt/
  // flatpak/snap/brew/hpm/nix/appimage) out of the same box entirely.
  // These make the global search box cover both in one place.
  discoverResults: DiscoverResult[];
  discoverLoading: boolean;
  isDiscoverInstalled: (source: string, packageId: string) => boolean;
  discoverBusyKey: string | null;
  onDiscoverInstall: (i: DiscoverItem) => void;
  onDiscoverUninstall: (i: DiscoverItem) => void;
  onDiscoverOpen: (i: DiscoverItem) => void;
}) {
  const { t } = useI18n();
  const title = () => {
    const n = props.results.length;
    if (n === 0) return t("search.noResultsFor", { q: props.query });
    if (n === 1) return t("search.resultFor", { q: props.query });
    return t("search.resultsFor", { n, q: props.query });
  };
  const nothingAtAll = () =>
    props.results.length === 0 && props.discoverResults.length === 0 && !props.discoverLoading;

  return (
    <div class="view">
      <h1 class="view-title">{title()}</h1>
      <Show when={nothingAtAll()}>
        <p class="view-sub">{t("search.tryDifferent")}</p>
      </Show>
      <Show when={props.results.length > 0}>
        <div class="pkg-list">
          <For each={props.results}>
            {pkg => (
              <PkgRow pkg={pkg}
                isInstalling={props.isInstalling} isUninstalling={props.isUninstalling} isInstalled={props.isInstalled}
                getVersion={props.getVersion} onInstall={props.onInstall} onUninstall={props.onUninstall} />
            )}
          </For>
        </div>
      </Show>

      <Show when={props.discoverLoading || props.discoverResults.length > 0}>
        <h2 class="section-heading">{t("search.fromDiscover")}</h2>
        <Show when={props.discoverLoading}>
          <div class="discover-spinner"><Loader2 size={22} class="spin" /></div>
        </Show>
        <Show when={!props.discoverLoading}>
          <div class="app-card-grid">
            <For each={props.discoverResults}>
              {r => (
                <DiscoverCard result={r}
                  installed={props.isDiscoverInstalled(r.source, r.package_id)}
                  busy={props.discoverBusyKey === `${r.source}::${r.package_id}`}
                  onInstall={props.onDiscoverInstall} onUninstall={props.onDiscoverUninstall} onOpen={props.onDiscoverOpen} />
              )}
            </For>
          </div>
        </Show>
      </Show>
    </div>
  );
}
