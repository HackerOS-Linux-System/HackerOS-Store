import { For, Show } from "solid-js";
import type { Package } from "../data/packages";
import { PkgRow } from "./PkgRow";
import { useI18n } from "../hooks/useI18n";

export function SearchResults(props: {
  results: Package[]; query: string;
  isInstalling: (p: Package) => boolean;
  isUninstalling: (p: Package) => boolean;
  isInstalled: (p: Package) => boolean;
  getVersion: (p: Package) => string | undefined;
  onInstall: (p: Package) => void;
  onUninstall: (p: Package) => void;
}) {
  const { t } = useI18n();
  const title = () => {
    const n = props.results.length;
    if (n === 0) return t("search.noResultsFor", { q: props.query });
    if (n === 1) return t("search.resultFor", { q: props.query });
    return t("search.resultsFor", { n, q: props.query });
  };

  return (
    <div class="view">
      <h1 class="view-title">{title()}</h1>
      <Show when={props.results.length === 0}>
        <p class="view-sub">{t("search.tryDifferent")}</p>
      </Show>
      <div class="pkg-list">
        <For each={props.results}>
          {pkg => (
            <PkgRow pkg={pkg}
              isInstalling={props.isInstalling} isUninstalling={props.isUninstalling} isInstalled={props.isInstalled}
              getVersion={props.getVersion} onInstall={props.onInstall} onUninstall={props.onUninstall} />
          )}
        </For>
      </div>
    </div>
  );
}
