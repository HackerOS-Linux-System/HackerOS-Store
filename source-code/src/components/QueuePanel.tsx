import { For, Show, createMemo } from "solid-js";
import { Loader2, X, ListOrdered, ChevronUp, ChevronDown } from "lucide-solid";
import type { QueueJob } from "../types";
import { useI18n } from "../hooks/useI18n";

export function QueuePanel(props: {
  jobs: QueueJob[];
  onDequeue: (id: string) => void;
  onReorder: (id: string, direction: "up" | "down") => void;
}) {
  const { t } = useI18n();
  const pendingIds = createMemo(() => props.jobs.filter(j => j.status === "pending").map(j => j.id));

  return (
    <Show when={props.jobs.length > 0}>
      <div class="queue-panel">
        <div class="queue-panel-header"><ListOrdered size={13} /> {t("queue.title")} ({props.jobs.length})</div>
        <ul class="queue-panel-list">
          <For each={props.jobs}>
            {j => {
              const idx = () => pendingIds().indexOf(j.id);
              const isPending = () => j.status === "pending";
              return (
                <li class={`queue-panel-item queue-panel-item--${j.status}`}>
                  <Show when={j.status === "running"}><Loader2 size={12} class="spin" /></Show>
                  <span class="queue-panel-label">{j.label}</span>
                  <span class="queue-panel-status">
                    {j.status === "pending" ? t("queue.pending") : j.status === "running" ? t("queue.running") : ""}
                  </span>
                  <Show when={isPending()}>
                    <div class="queue-panel-reorder">
                      <button class="queue-panel-reorder-btn" disabled={idx() <= 0}
                        onClick={() => props.onReorder(j.id, "up")} title={t("queue.moveUp")} aria-label={t("queue.moveUp")}>
                        <ChevronUp size={11} />
                      </button>
                      <button class="queue-panel-reorder-btn" disabled={idx() < 0 || idx() >= pendingIds().length - 1}
                        onClick={() => props.onReorder(j.id, "down")} title={t("queue.moveDown")} aria-label={t("queue.moveDown")}>
                        <ChevronDown size={11} />
                      </button>
                    </div>
                    <button class="queue-panel-remove" onClick={() => props.onDequeue(j.id)} title={t("btn.dequeue")}>
                      <X size={11} />
                    </button>
                  </Show>
                </li>
              );
            }}
          </For>
        </ul>
        <p class="queue-panel-note">{t("queue.note")}</p>
      </div>
    </Show>
  );
}
