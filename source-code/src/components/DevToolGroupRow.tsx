import { Show } from "solid-js";
import { Loader2, Trash2, Plus, HardDrive, Container as ContainerIcon } from "lucide-solid";
import type { DevToolGroup, Package } from "../data/packages";
import { PkgIcon } from "../iconMap";
import { useI18n } from "../hooks/useI18n";

/**
 * One row per toolchain (Rust, Node.js, ...) rather than one row per
 * Local/Container variant — the primary "Install" action is deliberately
 * mode-agnostic and hands off to `props.onAsk` (DevToolsView's
 * Local-vs-Container prompt) rather than installing either variant
 * directly, since a fresh HackerOS install is assumed to have neither.
 * Once one variant is installed, a small "+ Local"/"+ Container" button
 * lets someone add the other mode too without going through the prompt
 * again (at that point there's nothing left to ask — they've already
 * shown a preference by having one installed).
 */
export function DevToolGroupRow(props: {
  group: DevToolGroup;
  isInstalling: (p: Package) => boolean;
  isUninstalling: (p: Package) => boolean;
  isInstalled: (p: Package) => boolean;
  getVersion: (p: Package) => string | undefined;
  onInstall: (p: Package) => void;
  onUninstall: (p: Package) => void;
  onAsk: (group: DevToolGroup) => void;
}) {
  const { t } = useI18n();
  const localInstalled = () => props.isInstalled(props.group.local);
  const containerInstalled = () => props.isInstalled(props.group.container);
  const localInstalling = () => props.isInstalling(props.group.local);
  const containerInstalling = () => props.isInstalling(props.group.container);
  const localUninstalling = () => props.isUninstalling(props.group.local);
  const containerUninstalling = () => props.isUninstalling(props.group.container);
  const anyInstalled = () => localInstalled() || containerInstalled();
  const anyBusy = () => localInstalling() || containerInstalling() || localUninstalling() || containerUninstalling();

  return (
    <div class={`pkg-row ${anyInstalled() ? "pkg-row--installed" : ""}`}>
      <div class="pkg-row-icon"><PkgIcon name={props.group.icon} /></div>
      <div class="pkg-row-body">
        <div class="pkg-row-name">{props.group.label}</div>
        <div class="pkg-row-desc">{t("devtools.groupDesc")}</div>
      </div>
      <div class="pkg-row-actions devtool-row-actions">
        <Show when={localInstalled()}>
          <div class="devtool-mode-badge">
            <HardDrive size={11} />
            <span>{t("devtools.mode.local")}</span>
            <Show when={props.getVersion(props.group.local)}>
              <span class="installed-version">{props.getVersion(props.group.local)}</span>
            </Show>
            <Show when={!localUninstalling()} fallback={<Loader2 size={12} class="spin" />}>
              <button class="btn-uninstall" onClick={() => props.onUninstall(props.group.local)}
                title={t("btn.uninstall")} aria-label={`${t("btn.uninstall")} ${props.group.label} (${t("devtools.mode.local")})`}>
                <Trash2 size={12} />
              </button>
            </Show>
          </div>
        </Show>

        <Show when={containerInstalled()}>
          <div class="devtool-mode-badge">
            <ContainerIcon size={11} />
            <span>{t("devtools.mode.container")}</span>
            <Show when={!containerUninstalling()} fallback={<Loader2 size={12} class="spin" />}>
              <button class="btn-uninstall" onClick={() => props.onUninstall(props.group.container)}
                title={t("btn.uninstall")} aria-label={`${t("btn.uninstall")} ${props.group.label} (${t("devtools.mode.container")})`}>
                <Trash2 size={12} />
              </button>
            </Show>
          </div>
        </Show>

        <Show when={!anyInstalled()}>
          <Show
            when={!anyBusy()}
            fallback={<button class="btn btn-installing" disabled><Loader2 size={13} class="spin" /> {t("btn.installing")}</button>}
          >
            <button class="btn btn-install" onClick={() => props.onAsk(props.group)}>{t("btn.install")}</button>
          </Show>
        </Show>

        <Show when={anyInstalled() && !localInstalled() && !localInstalling()}>
          <button class="btn-add-mode" onClick={() => props.onInstall(props.group.local)}
            title={t("devtools.addMode", { mode: t("devtools.mode.local") })}>
            <Plus size={12} /> {t("devtools.mode.local")}
          </button>
        </Show>
        <Show when={anyInstalled() && !containerInstalled() && !containerInstalling()}>
          <button class="btn-add-mode" onClick={() => props.onInstall(props.group.container)}
            title={t("devtools.addMode", { mode: t("devtools.mode.container") })}>
            <Plus size={12} /> {t("devtools.mode.container")}
          </button>
        </Show>
      </div>
    </div>
  );
}
