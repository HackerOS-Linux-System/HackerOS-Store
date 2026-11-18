import { Show } from "solid-js";
import { Loader2, CheckCircle, Trash2 } from "lucide-solid";
import type { DiscoverResult, DiscoverItem } from "../types";
import { sourceColor } from "../types";
import { AppIcon } from "./AppIcon";
import { useI18n } from "../hooks/useI18n";

export function DiscoverCard(props: {
  result: DiscoverResult; installed: boolean; busy: boolean;
  onInstall: (i: DiscoverItem) => void;
  onUninstall: (i: DiscoverItem) => void;
  onOpen: (i: DiscoverItem) => void;
}) {
  const { t } = useI18n();
  const item = (): DiscoverItem => ({ package_id: props.result.package_id, source: props.result.source, name: props.result.name });
  return (
    <div class="app-card" onClick={() => props.onOpen(item())} role="button" tabIndex={0}
      onKeyDown={e => { if (e.key === "Enter") props.onOpen(item()); }}>
      <AppIcon icon={props.result.icon} source={props.result.source} size={48} />
      <div class="app-card-body">
        <div class="app-card-name">{props.result.name}</div>
        <span class="discover-source" style={{ "border-color": sourceColor(props.result.source), color: sourceColor(props.result.source) }}>
          {props.result.source}
        </span>
        <div class="app-card-desc">{props.result.desc || "No description available."}</div>
      </div>
      <div class="app-card-actions" onClick={e => e.stopPropagation()}>
        <Show when={!props.busy} fallback={<button class="btn btn-installing" disabled><Loader2 size={13} class="spin" /></button>}>
          <Show
            when={!props.installed}
            fallback={
              <div class="install-done-wrap">
                <button class="btn btn-installed" disabled><CheckCircle size={13} /></button>
                <button class="btn-uninstall" onClick={() => props.onUninstall(item())} title={t("btn.uninstall")}>
                  <Trash2 size={13} />
                </button>
              </div>
            }
          >
            <button class="btn btn-install" onClick={() => props.onInstall(item())}>{t("btn.install")}</button>
          </Show>
        </Show>
      </div>
    </div>
  );
}
