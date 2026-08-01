import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type { QueueJob } from "../types";

export type JobExecutor = (kind: QueueJob["kind"], payload: unknown) => Promise<void>;

/**
 * A person can tap "Install" on several apps in a row. Previously each tap
 * would just fire a backend call directly — fine for Flatpak/Snap which
 * don't mind concurrent operations, but genuinely unsafe for apt (dpkg
 * takes an exclusive lock, so a second concurrent `apt-get install` simply
 * errors out, or worse, contends for the lock). This hook gives the UI a
 * visible queue: every install/uninstall/update request is enqueued, and a
 * single in-memory runner drains it one at a time, in order, regardless of
 * how many were queued while another was still running.
 *
 * Unlike the first version of this hook, jobs are now plain, serializable
 * data (`kind` + `label` + a JSON-able `payload`) rather than closures —
 * the actual "how do I run this" logic lives in the `executor` callback
 * passed in by the caller (see `App.tsx`'s `executeJob`). That's what makes
 * it possible to persist the queue: every change is mirrored to
 * `~/.hackeros/store/queue.json` via the backend, and `hydrate()` (called
 * once on startup) reloads whatever was still pending — including a job
 * that was mid-flight when the app last closed, which gets re-queued as
 * "pending" and simply restarts from the top, since we have no way to know
 * how far the interrupted install actually got.
 */
export function useQueue(executor: JobExecutor) {
  const [jobs, setJobs] = createSignal<QueueJob[]>([]);
  const [runningId, setRunningId] = createSignal<string | null>(null);

  function persist() {
    const toSave = jobs()
      .filter(j => j.status === "pending" || j.status === "running")
      .map(j => ({ id: j.id, kind: j.kind, label: j.label, payload: j.payload }));
    void invoke("save_persisted_queue", { jobs: toSave }).catch(() => {});
  }

  /** Loads whatever was still queued the last time the app ran, and starts
   * draining it. Returns the restored jobs so the caller can also restore
   * any UI-only "this row is busy" state tied to them. */
  async function hydrate(): Promise<QueueJob[]> {
    try {
      const persisted = await invoke<{ id: string; kind: string; label: string; payload: unknown }[]>("get_persisted_queue");
      const restored: QueueJob[] = persisted.map(p => ({
        id: p.id, kind: p.kind as QueueJob["kind"], label: p.label, payload: p.payload, status: "pending",
      }));
      if (restored.length > 0) setJobs(restored);
      void tick();
      return restored;
    } catch {
      return [];
    }
  }

  function enqueue(kind: QueueJob["kind"], label: string, payload: unknown): string {
    const id = `q${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    setJobs(js => [...js, { id, kind, label, status: "pending", payload }]);
    persist();
    void tick();
    return id;
  }

  /** Removes a job that hasn't started yet. Running jobs can't be plucked
   * out mid-flight — cancel the active operation via the terminal modal's
   * Cancel button instead, which the queue will treat as a normal failure
   * and move on from. */
  function dequeue(id: string) {
    setJobs(js => js.filter(j => !(j.id === id && j.status === "pending")));
    persist();
  }

  /** Reorders a *pending* job up or down relative to the other pending
   * jobs (the currently-running job, if any, is never touched — you can't
   * un-start an `apt-get install` that's already underway; cancel it
   * instead). */
  function reorder(id: string, direction: "up" | "down") {
    setJobs(js => {
      const pendingIdx = js.map((j, i) => ({ j, i })).filter(x => x.j.status === "pending");
      const pos = pendingIdx.findIndex(x => x.j.id === id);
      if (pos === -1) return js;
      const swapWith = direction === "up" ? pos - 1 : pos + 1;
      if (swapWith < 0 || swapWith >= pendingIdx.length) return js;
      const a = pendingIdx[pos].i;
      const b = pendingIdx[swapWith].i;
      const next = [...js];
      [next[a], next[b]] = [next[b], next[a]];
      return next;
    });
    persist();
  }

  async function tick() {
    if (runningId()) return;
    const next = jobs().find(j => j.status === "pending");
    if (!next) return;

    setRunningId(next.id);
    setJobs(js => js.map(j => (j.id === next.id ? { ...j, status: "running" } : j)));
    persist();

    try {
      await executor(next.kind, next.payload);
      setJobs(js => js.map(j => (j.id === next.id ? { ...j, status: "done" } : j)));
    } catch (e) {
      setJobs(js => js.map(j => (j.id === next.id ? { ...j, status: "error", error: String(e) } : j)));
    } finally {
      setRunningId(null);
      persist();
      setTimeout(() => {
        setJobs(js => js.filter(j => j.id !== next.id || j.status === "error"));
        persist();
        void tick();
      }, 1500);
    }
  }

  return { jobs, enqueue, dequeue, reorder, hydrate };
}
