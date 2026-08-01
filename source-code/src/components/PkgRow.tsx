import { createSignal, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Info } from "lucide-solid";
import type { Package } from "../data/packages";
import { PkgIcon } from "../iconMap";
import { InstallBtn } from "./InstallBtn";
import { useI18n } from "../hooks/useI18n";

export function PkgRow(props: {
  pkg: Package;
  isInstalling: (p: Package) => boolean;
  isUninstalling: (p: Package) => boolean;
  isInstalled: (p: Package) => boolean;
  getVersion: (p: Package) => string | undefined;
  onInstall: (p: Package) => void;
  onUninstall: (p: Package) => void;
}) {
  const { t } = useI18n();
  const [info, setInfo] = createSignal<{ size?: string; version?: string; note?: string } | null>(null);
  const [showInfo, setShowInfo] = createSignal(false);

  const loadInfo = async () => {
    if (info()) { setShowInfo(v => !v); return; }
    try {
      const r = await invoke<{ size: string | null; version: string | null; note?: string }>(
        "get_package_info", { name: props.pkg.name, category: props.pkg.category },
      );
      setInfo({ size: r.size ?? undefined, version: r.version ?? undefined, note: r.note });
      setShowInfo(true);
    } catch {
      setInfo({});
      setShowInfo(true);
    }
  };

  return (
    <div class={`pkg-row ${props.isInstalled(props.pkg) ? "pkg-row--installed" : ""}`}>
      <div class="pkg-row-icon"><PkgIcon name={props.pkg.icon} /></div>
      <div class="pkg-row-body">
        <div class="pkg-row-name">
          {props.pkg.name}
          <Show when={props.isInstalled(props.pkg)}>
            <span class="row-installed-badge">{t("btn.installed").toLowerCase()}</span>
          </Show>
        </div>
        <div class="pkg-row-desc">{props.pkg.desc}</div>
        <Show when={props.pkg.tags}>
          <div class="pkg-row-tags">
            <For each={props.pkg.tags!.slice(0, 4)}>
              {tg => <span class="tag">{tg}</span>}
            </For>
          </div>
        </Show>
        <Show when={showInfo() && info()}>
          <div class="pkg-info-line">
            <Show when={info()!.version}><span>{t("info.latest")} {info()!.version}</span></Show>
            <Show when={info()!.size}><span>{t("info.size")} {info()!.size}</span></Show>
            <Show when={info()!.note}><span>{info()!.note}</span></Show>
            <Show when={!info()!.version && !info()!.size && !info()!.note}><span>{t("info.noInfo")}</span></Show>
          </div>
        </Show>
      </div>
      <div class="pkg-row-actions">
        <button class="btn-info" onClick={loadInfo} title={t("a11y.viewInfo")} aria-label={t("a11y.viewInfo")}><Info size={14} /></button>
        <InstallBtn pkg={props.pkg} installing={props.isInstalling(props.pkg)} uninstalling={props.isUninstalling(props.pkg)}
          installed={props.isInstalled(props.pkg)} version={props.getVersion(props.pkg)}
          onInstall={props.onInstall} onUninstall={props.onUninstall} />
      </div>
    </div>
  );
}
