import { createSignal, onCleanup } from "solid-js";

/**
 * Tracks `navigator.onLine` and updates on the browser's `online`/`offline`
 * events. This is what powers the new offline banner and the "you're
 * offline" message in Discover — previously there was no distinction
 * between "no results" and "can't reach the network at all", so someone
 * without a connection just saw an empty, unexplained results list.
 *
 * Caveat (documented rather than hidden): `navigator.onLine` only reflects
 * whether the OS reports an active network interface, not whether flathub/
 * odrs.gnome.org/the configured apt mirror are actually reachable. A
 * connected-but-broken network still reports `true` here; that residual
 * case is handled separately by Discover's "all sources failed" message
 * once a search/browse actually times out.
 */
export function useOnlineStatus() {
  const [online, setOnline] = createSignal(typeof navigator === "undefined" ? true : navigator.onLine);

  const goOnline = () => setOnline(true);
  const goOffline = () => setOnline(false);
  window.addEventListener("online", goOnline);
  window.addEventListener("offline", goOffline);
  onCleanup(() => {
    window.removeEventListener("online", goOnline);
    window.removeEventListener("offline", goOffline);
  });

  return online;
}
