import { Show } from "solid-js";
import { RefreshCw, Loader2, Terminal } from "lucide-solid";
import type { Progress } from "../types";
import { useI18n } from "../hooks/useI18n";

export function UpdateView(props: {
  updating: boolean; onUpdate: () => void; progress: Progress | null; onShowLog: () => void;
  updatesAvailable: number | null;
}) {
  const { t } = useI18n();
  return (
    <div class="view update-view">
      <div class="update-card">
        <div class="update-icon">
          <RefreshCw size={40} class={props.updating ? "spin" : ""} />
        </div>
        <h1 class="update-title">{t("update.title")}</h1>
        <Show when={!!props.updatesAvailable && !props.updating}>
          <p class="view-sub">{props.updatesAvailable} {t("update.badge")}</p>
        </Show>
        <Show when={props.progress && props.updating}>
          <div class="update-progress">
            <div class="update-progress-bar">
              <div class="update-progress-fill" style={{ width: `${Math.round((props.progress?.progress ?? 0) * 100)}%` }} />
            </div>
            <div class="update-progress-msg">{props.progress?.message}</div>
          </div>
        </Show>
        <button class={`btn btn-update ${props.updating ? "disabled" : ""}`} onClick={props.onUpdate} disabled={props.updating}>
          <Show when={props.updating} fallback={<><RefreshCw size={16} /> {t("btn.updateNow")}</>}>
            <Loader2 size={16} class="spin" /> {t("btn.updating")}
          </Show>
        </button>
        <Show when={props.updating}>
          <button class="btn-show-log" onClick={props.onShowLog}>
            <Terminal size={13} /> {t("btn.viewLog")}
          </button>
        </Show>
      </div>
    </div>
  );
}
