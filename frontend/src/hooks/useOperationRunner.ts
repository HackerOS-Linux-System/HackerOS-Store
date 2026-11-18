import { createSignal, onCleanup } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { LogLine, Progress } from "../types";

/**
 * The old App.tsx repeated the same ~12 lines of state juggling (reset log
 * lines, set title, show the terminal modal, mark it active, reset the
 * cancel flag, seed a "Starting…" progress step, and clean up in a
 * `finally`) in five different handlers: curated install, curated
 * uninstall, Discover install, Discover uninstall, and system update. This
 * hook centralizes that so each handler is just
 * `await runner.run(title, () => invoke(...))`.
 */
export function useOperationRunner() {
  const [logLines, setLogLines] = createSignal<LogLine[]>([]);
  const [logTitle, setLogTitle] = createSignal("");
  const [showLog, setShowLog]   = createSignal(false);
  const [logActive, setLogActive] = createSignal(false);
  const [progress, setProgress] = createSignal<Progress | null>(null);
  const [cancelling, setCancelling] = createSignal(false);
  const [busy, setBusy] = createSignal(false);

  const unlistenProgress = listen<Progress>("install_progress", e => setProgress(e.payload));
  const unlistenLog = listen<LogLine>("install_log", e => setLogLines(prev => [...prev, e.payload]));
  onCleanup(() => { unlistenProgress.then(f => f()); unlistenLog.then(f => f()); });

  async function run<T>(title: string, fn: () => Promise<T>): Promise<T> {
    setBusy(true);
    setLogLines([]);
    setLogTitle(title);
    setShowLog(true);
    setLogActive(true);
    setCancelling(false);
    setProgress({ step: "start", message: "Starting…", progress: 0 });
    try {
      return await fn();
    } finally {
      setLogActive(false);
      setBusy(false);
      setCancelling(false);
      setTimeout(() => setProgress(null), 1200);
    }
  }

  async function cancel(onCancelled: () => void) {
    setCancelling(true);
    try {
      await invoke("cancel_install");
      onCancelled();
    } catch {
      // best-effort — nothing to do if the backend has no active job
    }
  }

  return {
    logLines, logTitle, showLog, setShowLog, logActive,
    progress, cancelling, busy, run, cancel,
  };
}
