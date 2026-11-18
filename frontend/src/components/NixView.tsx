import { createSignal, createMemo, For, Show, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import {
  Snowflake, RotateCcw, Pin, PinOff, Trash2, Sparkles, RefreshCw,
  Stethoscope, ShieldCheck, Terminal, Loader2, HardDrive,
} from "lucide-solid";
import type { NixGeneration, NixInstalledPkg } from "../types";
import { useI18n } from "../hooks/useI18n";

/**
 * The dedicated Nix panel — everything `hnm` offers beyond Discover's
 * generic search/install/remove/info (see `hnm.rs`'s module doc comment):
 * generations + rollback, pin/unpin, `gc`, `clean`, `update` (index
 * rebuild), `env activate/deactivate/status`, `doctor`, `check`.
 *
 * Read-only data (generations, installed packages, store size, env
 * status) is loaded directly with `invoke()` and just re-fetched after
 * whichever mutating action needs it. Mutating actions go through
 * `props.run`/`props.busy` — the same `useOperationRunner` the rest of
 * the app uses — so Cancel and the shared TerminalLog work here exactly
 * like they do for install/uninstall/update.
 */
export function NixView(props: {
  available: boolean;
  run: <T,>(title: string, fn: () => Promise<T>) => Promise<T>;
  busy: boolean;
  addToast: (type: "success" | "error", message: string) => void;
  onBuildIndex: () => void;
}) {
  const { t } = useI18n();

  const [generations, setGenerations] = createSignal<NixGeneration[]>([]);
  const [installed, setInstalled] = createSignal<NixInstalledPkg[]>([]);
  const [storeSize, setStoreSize] = createSignal<string>("…");
  const [envStatus, setEnvStatus] = createSignal<string>("");
  const [report, setReport] = createSignal<{ title: string; text: string } | null>(null);
  const [pinTarget, setPinTarget] = createSignal<string | null>(null);
  const [pinVersion, setPinVersion] = createSignal("");
  const [loading, setLoading] = createSignal(false);

  const currentGeneration = createMemo(() => generations().find(g => g.current)?.generation);

  async function loadAll() {
    if (!props.available) return;
    setLoading(true);
    try {
      const [gens, pkgs, size, status] = await Promise.all([
        invoke<NixGeneration[]>("nix_list_generations").catch(() => []),
        invoke<NixInstalledPkg[]>("nix_list_installed").catch(() => []),
        invoke<string>("nix_store_size").catch(() => "?"),
        invoke<string>("nix_env_status").catch(() => ""),
      ]);
      setGenerations(gens.slice().sort((a, b) => b.generation - a.generation));
      setInstalled(pkgs);
      setStoreSize(size);
      setEnvStatus(status);
    } finally {
      setLoading(false);
    }
  }

  onMount(loadAll);

  async function doRollback(gen: NixGeneration) {
    if (gen.current) return;
    if (!window.confirm(t("nix.confirmRollback", { gen: String(gen.generation) }))) return;
    try {
      const msg = await props.run(t("nix.rollingBackTo", { gen: String(gen.generation) }),
        () => invoke<string>("nix_rollback", { generation: gen.generation }));
      props.addToast("success", msg || t("toast.rollbackOk"));
      await loadAll();
    } catch (err) {
      props.addToast("error", `${t("toast.rollbackFail")} ${err}`);
    }
  }

  function openPin(pkg: NixInstalledPkg) {
    setPinTarget(pkg.name);
    setPinVersion(pkg.pinned ?? pkg.version);
  }

  async function confirmPin() {
    const name = pinTarget();
    if (!name) return;
    try {
      const msg = await props.run(`${t("nix.pinning")} ${name}`,
        () => invoke<string>("nix_pin", { package: name, version: pinVersion().trim() || null }));
      props.addToast("success", msg);
      setPinTarget(null);
      await loadAll();
    } catch (err) {
      props.addToast("error", String(err));
    }
  }

  async function unpin(pkg: NixInstalledPkg) {
    try {
      const msg = await props.run(`${t("nix.unpinning")} ${pkg.name}`,
        () => invoke<string>("nix_unpin", { package: pkg.name }));
      props.addToast("success", msg);
      await loadAll();
    } catch (err) {
      props.addToast("error", String(err));
    }
  }

  async function runGc() {
    try {
      const msg = await props.run(t("nix.runningGc"), () => invoke<string>("nix_gc"));
      props.addToast("success", msg);
      await loadAll();
    } catch (err) {
      props.addToast("error", String(err));
    }
  }

  async function runClean() {
    try {
      const msg = await props.run(t("nix.cleaning"), () => invoke<string>("nix_clean"));
      props.addToast("success", msg);
    } catch (err) {
      props.addToast("error", String(err));
    }
  }

  async function runDoctor() {
    try {
      const text = await invoke<string>("nix_doctor");
      setReport({ title: t("nix.doctor"), text });
    } catch (err) {
      props.addToast("error", String(err));
    }
  }

  async function runCheck() {
    try {
      const text = await invoke<string>("nix_check");
      setReport({ title: t("nix.check"), text });
    } catch (err) {
      props.addToast("error", String(err));
    }
  }

  async function envActivate() {
    try {
      const msg = await props.run(t("nix.activatingEnv"), () => invoke<string>("nix_env_activate"));
      props.addToast("success", msg);
      await loadAll();
    } catch (err) {
      props.addToast("error", String(err));
    }
  }

  async function envDeactivate() {
    try {
      const msg = await props.run(t("nix.deactivatingEnv"), () => invoke<string>("nix_env_deactivate"));
      props.addToast("success", msg);
      await loadAll();
    } catch (err) {
      props.addToast("error", String(err));
    }
  }

  return (
    <div class="view nix-view">
      <h1 class="view-title"><Snowflake size={22} class="nix-title-icon" /> {t("nix.title")}</h1>
      <p class="view-sub">{t("nix.sub")}</p>

      <Show when={!props.available}>
        <section class="settings-section nix-unavailable">
          <p class="settings-hint">{t("nix.notDetected")}</p>
        </section>
      </Show>

      <Show when={props.available}>
        <section class="settings-section nix-stats-row">
          <div class="nix-stat"><HardDrive size={14} /> {t("nix.storeSize")}: <strong>{storeSize()}</strong></div>
          <div class="nix-stat"><Snowflake size={14} /> {t("nix.currentGeneration")}: <strong>{currentGeneration() ?? "—"}</strong></div>
          <button class="btn-info" title={t("btn.refresh")} onClick={loadAll} disabled={loading()}>
            <Show when={loading()} fallback={<RefreshCw size={14} />}><Loader2 size={14} class="spin" /></Show>
          </button>
        </section>

        <section class="settings-section">
          <h2 class="settings-heading">{t("nix.indexHeading")}</h2>
          <p class="settings-hint">{t("settings.buildNixIndexHint")}</p>
          <button class="btn btn-uninstall-wide" disabled={props.busy} onClick={props.onBuildIndex}>
            <RefreshCw size={14} /> {props.busy ? t("btn.working") : t("settings.buildNixIndex")}
          </button>
        </section>

        <section class="settings-section">
          <h2 class="settings-heading">{t("nix.generationsHeading")}</h2>
          <p class="settings-hint">{t("nix.generationsHint")}</p>
          <Show when={generations().length === 0}>
            <p class="view-sub">{t("nix.noGenerations")}</p>
          </Show>
          <div class="remote-list">
            <For each={generations()}>
              {gen => (
                <div class="remote-row">
                  <span class="remote-name">{t("nix.generation")} {gen.generation}</span>
                  <span class="remote-url">{gen.date}</span>
                  <Show when={gen.current}>
                    <span class="brew-status brew-status--ok">{t("nix.current")}</span>
                  </Show>
                  <Show when={!gen.current}>
                    <button class="btn-info" title={t("btn.rollback")} disabled={props.busy}
                      onClick={() => doRollback(gen)}>
                      <RotateCcw size={13} />
                    </button>
                  </Show>
                </div>
              )}
            </For>
          </div>
        </section>

        <section class="settings-section">
          <h2 class="settings-heading">{t("nix.installedHeading")}</h2>
          <Show when={installed().length === 0}>
            <p class="view-sub">{t("nix.noInstalled")}</p>
          </Show>
          <div class="remote-list">
            <For each={installed()}>
              {pkg => (
                <div class="remote-row">
                  <span class="remote-name">{pkg.name}</span>
                  <span class="remote-url">
                    v{pkg.version}
                    <Show when={pkg.pinned}> · {t("nix.pinnedTo")} {pkg.pinned}</Show>
                  </span>
                  <Show when={pkg.pinned} fallback={
                    <button class="btn-info" title={t("nix.pin")} disabled={props.busy} onClick={() => openPin(pkg)}>
                      <Pin size={13} />
                    </button>
                  }>
                    <button class="btn-info" title={t("nix.unpin")} disabled={props.busy} onClick={() => unpin(pkg)}>
                      <PinOff size={13} />
                    </button>
                  </Show>
                </div>
              )}
            </For>
          </div>

          <Show when={pinTarget()}>
            <div class="remote-row remote-row--add" style={{ "margin-top": "10px" }}>
              <span class="remote-name">{t("nix.pinning")} {pinTarget()}</span>
              <input class="settings-input" placeholder={t("nix.pinVersionPlaceholder")}
                value={pinVersion()} onInput={e => setPinVersion(e.currentTarget.value)} />
              <button class="btn btn-install" onClick={confirmPin}>{t("btn.save")}</button>
              <button class="btn-info" onClick={() => setPinTarget(null)}>{t("btn.cancel")}</button>
            </div>
          </Show>
        </section>

        <section class="settings-section">
          <h2 class="settings-heading">{t("nix.maintenanceHeading")}</h2>
          <p class="settings-hint">{t("nix.gcHint")}</p>
          <button class="btn btn-uninstall-wide" disabled={props.busy} onClick={runGc}>
            <Sparkles size={14} /> {t("nix.runGc")}
          </button>
          <p class="settings-hint" style={{ "margin-top": "14px" }}>{t("nix.cleanHint")}</p>
          <button class="btn btn-uninstall-wide" disabled={props.busy} onClick={runClean}>
            <Trash2 size={14} /> {t("nix.runClean")}
          </button>
          <p class="settings-hint" style={{ "margin-top": "14px" }}>{t("nix.diagnosticsHint")}</p>
          <div class="nix-diag-row">
            <button class="btn btn-uninstall-wide" onClick={runDoctor}>
              <Stethoscope size={14} /> {t("nix.doctor")}
            </button>
            <button class="btn btn-uninstall-wide" onClick={runCheck}>
              <ShieldCheck size={14} /> {t("nix.check")}
            </button>
          </div>
          <Show when={report()}>
            <pre class="nix-report">{report()!.text || t("nix.reportEmpty")}</pre>
          </Show>
        </section>

        <section class="settings-section">
          <h2 class="settings-heading">{t("nix.envHeading")}</h2>
          <p class="settings-hint">{t("nix.envHint")}</p>
          <pre class="nix-report">{envStatus() || "…"}</pre>
          <div class="nix-diag-row">
            <button class="btn btn-uninstall-wide" disabled={props.busy} onClick={envActivate}>
              <Terminal size={14} /> {t("nix.activateEnv")}
            </button>
            <button class="btn btn-uninstall-wide" disabled={props.busy} onClick={envDeactivate}>
              <Terminal size={14} /> {t("nix.deactivateEnv")}
            </button>
          </div>
        </section>
      </Show>
    </div>
  );
}
