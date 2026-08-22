import { For } from "solid-js";
import type { Package } from "../data/packages";
import { PkgRow } from "./PkgRow";
import { InstallAllButton } from "./InstallAllButton";

export function PackageList(props: {
  title: string; packages: Package[];
  isInstalling: (p: Package) => boolean;
  isUninstalling: (p: Package) => boolean;
  isInstalled: (p: Package) => boolean;
  getVersion: (p: Package) => string | undefined;
  onInstall: (p: Package) => void;
  onUninstall: (p: Package) => void;
}) {
  return (
    <div class="view">
      <div class="view-header-row">
        <h1 class="view-title">{props.title}</h1>
        <InstallAllButton packages={props.packages} isInstalled={props.isInstalled}
          isInstalling={props.isInstalling} onInstall={props.onInstall} />
      </div>
      <div class="pkg-list">
        <For each={props.packages}>
          {pkg => (
            <PkgRow pkg={pkg} isInstalling={props.isInstalling} isUninstalling={props.isUninstalling}
              isInstalled={props.isInstalled} getVersion={props.getVersion}
              onInstall={props.onInstall} onUninstall={props.onUninstall} />
          )}
        </For>
      </div>
    </div>
  );
}
