import { Show } from "solid-js";
import { Loader2, CheckCircle, Trash2, Lock } from "lucide-solid";
import type { Package } from "../data/packages";
import { useI18n } from "../hooks/useI18n";

export function InstallBtn(props: {
  pkg: Package; installing: boolean; uninstalling: boolean; installed: boolean;
  version?: string; onInstall: (p: Package) => void; onUninstall: (p: Package) => void;
}) {
  const { t } = useI18n();
  // Defaults to removable (true) — only false for entries that explicitly
  // opt out, currently just the HackerOS Ecosystem's Hydra (see
  // data/packages.ts).
  const removable = () => props.pkg.uninstallable !== false;
  return (
    <Show
      when={!props.uninstalling}
      fallback={
        <button class="btn btn-installing" disabled>
          <Loader2 size={13} class="spin" /> {t("btn.uninstalling")}
        </button>
      }
    >
      <Show
        when={!props.installed}
        fallback={
          <div class="install-done-wrap">
            <button class="btn btn-installed" disabled>
              <CheckCircle size={13} /> {t("btn.installed")}
            </button>
            <Show when={props.version}><span class="installed-version">{props.version}</span></Show>
            <Show
              when={removable()}
              fallback={
                <span class="btn-uninstall btn-uninstall--locked" title={t("btn.cannotUninstall")} aria-label={t("btn.cannotUninstall")}>
                  <Lock size={13} />
                </span>
              }
            >
              <button class="btn-uninstall" onClick={() => props.onUninstall(props.pkg)} title={t("btn.uninstall")} aria-label={`${t("btn.uninstall")} ${props.pkg.name}`}>
                <Trash2 size={13} />
              </button>
            </Show>
          </div>
        }
      >
        <Show
          when={!props.installing}
          fallback={
            <button class="btn btn-installing" disabled>
              <Loader2 size={13} class="spin" /> {t("btn.installing")}
            </button>
          }
        >
          <button class="btn btn-install" onClick={() => props.onInstall(props.pkg)}>{t("btn.install")}</button>
        </Show>
      </Show>
    </Show>
  );
}
