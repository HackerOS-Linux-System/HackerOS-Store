import { For, Show, createEffect } from "solid-js";
import { Loader2, X, XCircle } from "lucide-solid";
import type { LogLine } from "../types";
import { useI18n } from "../hooks/useI18n";
import { useFocusTrap } from "../hooks/useFocusTrap";

export function TerminalLog(props: {
  lines: LogLine[]; onClose: () => void; title: string; active: boolean;
  onCancel?: () => void; cancelling?: boolean;
}) {
  const { t } = useI18n();
  let bottomRef: HTMLDivElement | undefined;
  let modalRef: HTMLDivElement | undefined;
  useFocusTrap(() => modalRef);
  createEffect(() => {
    // Reading props.lines here (rather than props.lines.length) keeps this
    // effect re-running on every new line, matching the old
    // `useEffect(..., [lines])` dependency.
    void props.lines;
    bottomRef?.scrollIntoView({ behavior: "smooth" });
  });

  return (
    <div class="modal-overlay" onClick={props.onClose}>
      <div ref={modalRef} class="terminal-modal" role="dialog" aria-modal="true" aria-label={props.title} onClick={e => e.stopPropagation()}>
        <div class="terminal-header">
          <div class="terminal-dots">
            <span class="dot dot-red" /><span class="dot dot-yellow" /><span class="dot dot-green" />
          </div>
          <span class="terminal-title">{props.title}</span>
          <Show when={props.active}><Loader2 size={13} class="spin terminal-spinner" /></Show>
          <Show when={props.active && props.onCancel}>
            <button class="terminal-cancel" onClick={props.onCancel} disabled={props.cancelling} title={t("btn.cancel")}>
              <XCircle size={13} /> {props.cancelling ? t("btn.cancelling") : t("btn.cancel")}
            </button>
          </Show>
          <button class="terminal-close" onClick={props.onClose} aria-label={t("a11y.closeDialog")}><X size={14} /></button>
        </div>
        <div class="terminal-body">
          <For each={props.lines}>
            {l => (
              <div class={`log-line log-${l.stream}`}>
                <span class="log-prefix">
                  {l.stream === "stdout" ? ">" : l.stream === "stderr" ? "!" :
                   l.stream === "info" ? "•" : l.stream === "success" ? "✓" : "✗"}
                </span>
                <span class="log-text">{l.line}</span>
              </div>
            )}
          </For>
          <div ref={bottomRef} />
        </div>
      </div>
    </div>
  );
}
