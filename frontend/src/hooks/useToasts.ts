import { createSignal } from "solid-js";
import type { ToastItem } from "../types";

export function useToasts() {
  const [toasts, setToasts] = createSignal<ToastItem[]>([]);
  let nextId = 0;

  function addToast(type: ToastItem["type"], message: string) {
    const id = ++nextId;
    setToasts(t => [...t, { id, type, message }]);
    setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 5000);
  }

  return { toasts, addToast };
}
