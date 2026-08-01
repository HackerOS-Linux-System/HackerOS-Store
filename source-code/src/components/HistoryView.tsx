import { createSignal, For, Show, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { CheckCircle, XCircle, RotateCcw, Trash2, Loader2 } from "lucide-solid";
import type { HistoryEntry } from "../types";
import { useI18n } from "../hooks/useI18n";

export function HistoryView(props: {
  addToast: (type: "success" | "error", message: string) => void;
}) {
  const { t } = useI18n();
  const [entries, setEntries] = createSignal<HistoryEntry[]>([]);
  const [rollingBack, setRollingBack] = createSignal<string | null>(null);

  async function load() {
    try { setEntries(await invoke<HistoryEntry[]>("get_install_history")); }
    catch { setEntries([]); }
  }
  onMount(load);

  async function rollback(entry: HistoryEntry) {
    setRollingBack(entry.id);
    try {
      const msg = await invoke<string>("rollback_history_entry", { entryId: entry.id });
      props.addToast("success", msg || t("toast.rollbackOk"));
      await load();
    } catch (err) {
      props.addToast("error", `${t("toast.rollbackFail")} ${err}`);
    } finally {
      setRollingBack(null);
    }
  }

  async function clearAll() {
    if (!window.confirm(t("history.confirmClear"))) return;
    try { await invoke("clear_install_history"); await load(); }
    catch (err) { props.addToast("error", String(err)); }
  }

  function timeLabel(iso: string): string {
    const m = /^epoch:(\d+)$/.exec(iso);
    if (!m) return iso;
    return new Date(Number(m[1]) * 1000).toLocaleString();
  }

  function actionLabel(a: HistoryEntry["action"]): string {
    return t(`history.action.${a}`);
  }

  // Rollback is offered per-source, matching exactly what
  // `rollback_history_entry`/`history.rs` can actually do for each one
  // (see that module's doc comment):
  //   - apt: a single recorded version, or a per-package version list
  //     (drivers touch several apt packages at once)
  //   - hpm: hpm tracks its own pre-install/remove snapshot, so any
  //     successful hpm install entry can attempt it — no extra metadata
  //     needed from history.json itself
  //   - appimage: same idea — appimage.rs tracks the previous cached
  //     version itself; attempting is safe even if there turns out to be
  //     nothing to roll back to (a clear error comes back either way)
  //   - flatpak: only if an ostree commit was actually recorded
  //   - nix: only if a generation number was recorded — and only makes
  //     sense for install/uninstall (not e.g. an already-recorded
  //     rollback entry), since it reverts the whole nix profile
  function canRollback(e: HistoryEntry): boolean {
    if (!e.success) return false;
    switch (e.source) {
      case "apt": return e.action === "install" && (!!e.version || !!(e.packages && e.packages.length > 0));
      case "hpm": return e.action === "install";
      case "appimage": return e.action === "install";
      case "flatpak": return e.action === "install" && !!e.commit;
      case "nix": return (e.action === "install" || e.action === "uninstall") && !!e.nix_generation;
      default: return false;
    }
  }

  return (
    <div class="view">
      <h1 class="view-title">{t("history.title")}</h1>
      <p class="view-sub">{t("history.sub")}</p>
      <p class="settings-hint">{t("history.rollbackNote")}</p>

      <Show when={entries().length === 0}>
        <p class="view-sub">{t("history.empty")}</p>
      </Show>

      <Show when={entries().length > 0}>
        <button class="btn btn-uninstall-wide" style={{ "margin-bottom": "12px" }} onClick={clearAll}>
          <Trash2 size={14} /> {t("btn.clearHistory")}
        </button>
        <ul class="history-list">
          <For each={entries()}>
            {e => (
              <li class={`history-item ${e.success ? "history-item--ok" : "history-item--fail"}`}>
                {e.success ? <CheckCircle size={16} /> : <XCircle size={16} />}
                <div class="history-item-body">
                  <div class="history-item-title">
                    {actionLabel(e.action)} — {e.name} <span class="history-item-source">({e.source})</span>
                  </div>
                  <Show when={e.version}><div class="history-item-version">v{e.version}</div></Show>
                  <Show when={e.nix_generation}><div class="history-item-version">generation {e.nix_generation}</div></Show>
                  <Show when={e.packages && e.packages.length > 0}>
                    <div class="history-item-version">
                      {e.packages!.map(p => `${p.name}=${p.version}`).join(", ")}
                    </div>
                  </Show>
                  <Show when={e.message && !e.success}><div class="history-item-message">{e.message}</div></Show>
                  <div class="history-item-time">{timeLabel(e.timestamp)}</div>
                </div>
                <Show when={canRollback(e)}>
                  <button class="btn-info" title={t("btn.rollback")} disabled={rollingBack() === e.id}
                    onClick={() => rollback(e)}>
                    <Show when={rollingBack() === e.id} fallback={<RotateCcw size={14} />}>
                      <Loader2 size={14} class="spin" />
                    </Show>
                  </button>
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}
