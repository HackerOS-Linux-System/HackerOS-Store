import { onMount } from "solid-js";
import { X, HardDrive, Container as ContainerIcon } from "lucide-solid";
import type { DevToolGroup } from "../data/packages";
import { useI18n } from "../hooks/useI18n";
import { useFocusTrap } from "../hooks/useFocusTrap";

/**
 * "How do you want this installed?" — shown by DevToolsView when the
 * person clicks Install on a toolchain with neither Local nor Container
 * variant present yet (a fresh HackerOS install is assumed to have
 * none of these), unless they've set a fixed default in Settings
 * (`AppSettings.dev_tools_default_mode`), in which case DevToolsView
 * skips this and installs that way directly.
 */
export function DevToolModePrompt(props: {
  group: DevToolGroup;
  onChoose: (mode: "local" | "container") => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  let modalRef: HTMLDivElement | undefined;
  let closeBtn: HTMLButtonElement | undefined;
  useFocusTrap(() => modalRef);
  onMount(() => closeBtn?.focus());

  return (
    <div class="modal-overlay" onClick={props.onClose}>
      <div ref={modalRef} class="devtool-prompt-modal" role="dialog" aria-modal="true"
        aria-label={t("devtools.askTitle", { name: props.group.label })} onClick={e => e.stopPropagation()}>
        <button ref={closeBtn} class="detail-close" onClick={props.onClose} aria-label={t("detail.close")}>
          <X size={16} />
        </button>
        <h2 class="devtool-prompt-title">{t("devtools.askTitle", { name: props.group.label })}</h2>
        <p class="settings-hint">{t("devtools.askHint")}</p>
        <div class="devtool-prompt-choices">
          <button class="devtool-prompt-choice" onClick={() => props.onChoose("local")}>
            <HardDrive size={20} />
            <span class="devtool-prompt-choice-title">{t("devtools.mode.local")}</span>
            <span class="devtool-prompt-choice-desc">{t("devtools.askLocalDesc")}</span>
          </button>
          <button class="devtool-prompt-choice" onClick={() => props.onChoose("container")}>
            <ContainerIcon size={20} />
            <span class="devtool-prompt-choice-title">{t("devtools.mode.container")}</span>
            <span class="devtool-prompt-choice-desc">{t("devtools.askContainerDesc")}</span>
          </button>
        </div>
        <button class="btn btn-uninstall-wide" onClick={props.onClose}>{t("btn.cancel")}</button>
      </div>
    </div>
  );
}
