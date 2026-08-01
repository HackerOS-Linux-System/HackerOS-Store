import { Show } from "solid-js";
import { Loader2, CheckCircle, Trash2 } from "lucide-solid";
import type { Package } from "../data/packages";
import { useI18n } from "../hooks/useI18n";

export function InstallBtn(props: {
  pkg: Package; installing: boolean; uninstalling: boolean; installed: boolean;
  version?: string; onInstall: (p: Package) => void; onUninstall: (p: Package) => void;
}) {
  const { t } = useI18n();
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
            <button class="btn-uninstall" onClick={() => props.onUninstall(props.pkg)} title={t("btn.uninstall")} aria-label={`${t("btn.uninstall")} ${props.pkg.name}`}>
              <Trash2 size={13} />
            </button>
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
