import { For, Show } from "solid-js";
import { CheckCircle, AlertCircle, Info } from "lucide-solid";
import type { ToastItem } from "../types";

export function Toasts(props: { toasts: ToastItem[] }) {
  return (
    <div class="toast-stack" role="status" aria-live="polite">
      <For each={props.toasts}>
        {ts => (
          <div class={`toast toast-${ts.type}`}>
            <Show when={ts.type === "success"}><CheckCircle size={15} /></Show>
            <Show when={ts.type === "error"}><AlertCircle size={15} /></Show>
            <Show when={ts.type === "info"}><Info size={15} /></Show>
            <span>{ts.message}</span>
          </div>
        )}
      </For>
    </div>
  );
}
