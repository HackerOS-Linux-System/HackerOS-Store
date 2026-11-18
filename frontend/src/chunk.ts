import { Accessor, createEffect, createMemo, createSignal, on, onCleanup, onMount } from "solid-js";

/**
 * Silver renderuje DOM wolniej niż WebView, więc długie listy pokazujemy partiami:
 * kolejna partia dochodzi, gdy użytkownik przewinie `.main` blisko końca.
 */
export function createChunked<T>(list: Accessor<T[]>, step = 20): Accessor<T[]> {
  const [n, setN] = createSignal(step);
  createEffect(on(list, () => setN(step), { defer: true }));
  onMount(() => {
    const el = document.querySelector(".main") as HTMLElement | null;
    if (!el) return;
    const check = () => {
      if (n() >= list().length) return;
      if (el.scrollTop + el.clientHeight >= el.scrollHeight - 500) setN(n() + step);
    };
    el.addEventListener("scroll", check);
    onCleanup(() => el.removeEventListener("scroll", check));
  });
  return createMemo(() => list().slice(0, n()));
}
