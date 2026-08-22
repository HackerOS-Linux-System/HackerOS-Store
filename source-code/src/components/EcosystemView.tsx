import { For, Show, createMemo } from "solid-js";
import { AlertTriangle } from "lucide-solid";
import type { Package } from "../data/packages";
import { HACKEROS_ECOSYSTEM } from "../data/packages";
import { ECOSYSTEM_TAGS } from "../types";
import { PkgRow } from "./PkgRow";
import { InstallAllButton } from "./InstallAllButton";
import { useI18n } from "../hooks/useI18n";

/** First-party HackerOS tools/add-ons/environments, installed and removed
 * via the system's own `hacker` CLI (`hacker unpack <slug>` / `hacker pack
 * <slug>`) rather than apt/flatpak/snap/etc — see lib.rs's "HackerOS
 * Ecosystem" section for how each row maps to a slug. */
export function EcosystemView(props: {
  tag: string; onTag: (t: string) => void;
  hackerAvailable: boolean | null;
  isInstalling: (p: Package) => boolean;
  isUninstalling: (p: Package) => boolean;
  isInstalled: (p: Package) => boolean;
  getVersion: (p: Package) => string | undefined;
  onInstall: (p: Package) => void;
  onUninstall: (p: Package) => void;
}) {
  const { t } = useI18n();
  const filtered = createMemo(() =>
    props.tag === "all" ? HACKEROS_ECOSYSTEM : HACKEROS_ECOSYSTEM.filter(p => p.tags?.includes(props.tag)),
  );
  return (
    <div class="view">
      <div class="view-header-row">
        <h1 class="view-title">{t("nav.hackeros_ecosystem")}</h1>
        <InstallAllButton packages={filtered()} isInstalled={props.isInstalled}
          isInstalling={props.isInstalling} onInstall={props.onInstall} />
      </div>
      <p class="view-sub">{t("ecosystem.sub")}</p>
      <Show when={props.hackerAvailable === false}>
        <div class="offline-banner" role="status">
          <AlertTriangle size={14} />
          <span>{t("ecosystem.hackerMissing")}</span>
        </div>
      </Show>
      <div class="tag-pills">
        <For each={ECOSYSTEM_TAGS}>
          {tg => (
            <button class={`tag-pill ${props.tag === tg ? "active" : ""}`} onClick={() => props.onTag(tg)}>{tg}</button>
          )}
        </For>
      </div>
      <div class="pentest-count">{filtered().length} tools</div>
      <div class="pkg-list">
        <For each={filtered()}>
          {pkg => (
            <PkgRow pkg={pkg} isInstalling={props.isInstalling} isUninstalling={props.isUninstalling}
              isInstalled={props.isInstalled} getVersion={props.getVersion}
              onInstall={props.onInstall} onUninstall={props.onUninstall} />
          )}
        </For>
      </div>
    </div>
  );
}
