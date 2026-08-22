import { Show, createMemo } from "solid-js";
import { CloudDownload } from "lucide-solid";
import type { Package } from "../data/packages";
import { useI18n } from "../hooks/useI18n";

/**
 * "Install everything not already installed" for whatever list of
 * packages a section is currently showing — the current tag/filter
 * (pentest tags, ecosystem tags) narrows `packages` before it ever
 * reaches here, so this only ever installs what's actually visible, not
 * the section's full catalog.
 *
 * Just calls `onInstall` once per pending package — `useQueue` (see
 * `App.tsx`) already serializes actual installs one at a time, so this
 * is safe to fire in a tight loop; it only enqueues, it doesn't run
 * anything concurrently.
 */
export function InstallAllButton(props: {
  packages: Package[];
  isInstalled: (p: Package) => boolean;
  isInstalling: (p: Package) => boolean;
  onInstall: (p: Package) => void;
}) {
  const { t } = useI18n();
  const pending = createMemo(() => props.packages.filter(p => !props.isInstalled(p) && !props.isInstalling(p)));

  function run() {
    const list = pending();
    if (list.length === 0) return;
    if (!window.confirm(t("confirm.installAll", { n: list.length }))) return;
    for (const pkg of list) props.onInstall(pkg);
  }

  return (
    <Show when={pending().length > 0}>
      <button class="btn btn-install-all" onClick={run}>
        <CloudDownload size={14} /> {t("btn.installAll")} ({pending().length})
      </button>
    </Show>
  );
}
