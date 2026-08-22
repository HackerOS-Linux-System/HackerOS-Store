import { For, Show, createMemo, createSignal } from "solid-js";
import { AlertTriangle, CloudDownload } from "lucide-solid";
import type { DevToolGroup, Package } from "../data/packages";
import { DEV_TOOL_GROUPS } from "../data/packages";
import { DEV_TOOLS_LANG_TAGS } from "../types";
import { DevToolGroupRow } from "./DevToolGroupRow";
import { DevToolModePrompt } from "./DevToolModePrompt";
import { useI18n } from "../hooks/useI18n";

/** Language toolchains not preinstalled on a fresh HackerOS install (no
 * cargo, no npm, ...). Each toolchain gets one grouped row (see
 * `DevToolGroupRow`) with a single mode-agnostic "Install" action that
 * asks Local-vs-Container per `defaultMode` — "ask" (the default) shows
 * the prompt every time, "local"/"container" skip it and always install
 * that way. See lib.rs's "Dev Tools" section for how each variant maps
 * to an apt package / the shared `hackeros-devbox` Podman container. */
export function DevToolsView(props: {
  langTag: string; onLangTag: (t: string) => void;
  defaultMode: string;
  podmanAvailable: boolean | null;
  isInstalling: (p: Package) => boolean;
  isUninstalling: (p: Package) => boolean;
  isInstalled: (p: Package) => boolean;
  getVersion: (p: Package) => string | undefined;
  onInstall: (p: Package) => void;
  onUninstall: (p: Package) => void;
}) {
  const { t } = useI18n();
  const [asking, setAsking] = createSignal<DevToolGroup | null>(null);
  const [batchAsking, setBatchAsking] = createSignal(false);

  const filtered = createMemo(() =>
    props.langTag === "all" ? DEV_TOOL_GROUPS : DEV_TOOL_GROUPS.filter(g => g.tags.includes(props.langTag)),
  );
  const missing = createMemo(() =>
    filtered().filter(g => !props.isInstalled(g.local) && !props.isInstalled(g.container)),
  );

  function installGroup(group: DevToolGroup, mode: "local" | "container") {
    props.onInstall(mode === "local" ? group.local : group.container);
  }

  function requestInstall(group: DevToolGroup) {
    if (props.defaultMode === "local" || props.defaultMode === "container") {
      installGroup(group, props.defaultMode);
    } else {
      setAsking(group);
    }
  }

  function installAllMissing(mode: "local" | "container") {
    for (const g of missing()) installGroup(g, mode);
  }

  function requestInstallAll() {
    if (missing().length === 0) return;
    if (props.defaultMode === "local" || props.defaultMode === "container") {
      if (!window.confirm(t("confirm.installAll", { n: missing().length }))) return;
      installAllMissing(props.defaultMode);
    } else {
      setBatchAsking(true);
    }
  }

  return (
    <div class="view">
      <div class="view-header-row">
        <h1 class="view-title">{t("nav.dev_tools")}</h1>
        <Show when={missing().length > 0}>
          <button class="btn btn-install-all" onClick={requestInstallAll}>
            <CloudDownload size={14} /> {t("btn.installAll")} ({missing().length})
          </button>
        </Show>
      </div>
      <p class="view-sub">{t("devtools.sub")}</p>
      <Show when={props.podmanAvailable === false}>
        <div class="offline-banner" role="status">
          <AlertTriangle size={14} />
          <span>{t("devtools.podmanHint")}</span>
        </div>
      </Show>

      <div class="tag-pills">
        <For each={DEV_TOOLS_LANG_TAGS}>
          {tg => (
            <button class={`tag-pill ${props.langTag === tg ? "active" : ""}`} onClick={() => props.onLangTag(tg)}>{tg}</button>
          )}
        </For>
      </div>
      <div class="pentest-count">{filtered().length} tools</div>

      <div class="pkg-list">
        <For each={filtered()}>
          {group => (
            <DevToolGroupRow group={group}
              isInstalling={props.isInstalling} isUninstalling={props.isUninstalling} isInstalled={props.isInstalled}
              getVersion={props.getVersion} onInstall={props.onInstall} onUninstall={props.onUninstall}
              onAsk={requestInstall} />
          )}
        </For>
      </div>

      <Show when={asking()}>
        <DevToolModePrompt group={asking()!}
          onChoose={mode => { installGroup(asking()!, mode); setAsking(null); }}
          onClose={() => setAsking(null)} />
      </Show>
      <Show when={batchAsking() && missing().length > 0}>
        <DevToolModePrompt group={{ ...missing()[0], label: t("devtools.askAllLabel", { n: missing().length }) }}
          onChoose={mode => { installAllMissing(mode); setBatchAsking(false); }}
          onClose={() => setBatchAsking(false)} />
      </Show>
    </div>
  );
}
