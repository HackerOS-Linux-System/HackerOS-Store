import { onMount, onCleanup } from "solid-js";

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), ' +
  'select:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * A modal previously took focus on open (via a ref + `.focus()` on its
 * close button) but did nothing to keep it there: pressing Tab enough
 * times moved focus straight out into the page behind the modal, and
 * closing the modal never gave focus back to whatever had opened it (the
 * Discover card, the nav item, …) — both are real problems for anyone
 * navigating by keyboard or screen reader.
 *
 * This hook fixes both, for any modal: pass an accessor for the modal's
 * root element and it will
 *   1. remember whatever had focus right before the modal opened,
 *   2. intercept Tab/Shift+Tab so focus cycles between the first and last
 *      focusable element *inside* the modal instead of escaping it, and
 *   3. restore focus to the original element once the modal unmounts.
 *
 * Usage: call it once from the modal component, alongside the existing
 * "focus something inside on mount" logic:
 *
 *   let rootRef: HTMLDivElement | undefined;
 *   useFocusTrap(() => rootRef);
 *   return <div ref={rootRef} role="dialog" aria-modal="true">...</div>;
 */
export function useFocusTrap(getContainer: () => HTMLElement | undefined) {
  let previouslyFocused: HTMLElement | null = null;

  onMount(() => {
    previouslyFocused = document.activeElement as HTMLElement | null;

    const onKeydown = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const container = getContainer();
      if (!container) return;

      const focusables = Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR))
        .filter(el => el.offsetParent !== null); // skip hidden elements
      if (focusables.length === 0) {
        e.preventDefault();
        return;
      }

      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = document.activeElement as HTMLElement | null;
      const activeIsInside = !!active && container.contains(active);

      if (e.shiftKey) {
        if (!activeIsInside || active === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (!activeIsInside || active === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };

    // `capture: true` so this runs before any other Tab handling and can't
    // be skipped by a click that happened to land focus outside the modal.
    document.addEventListener("keydown", onKeydown, true);

    onCleanup(() => {
      document.removeEventListener("keydown", onKeydown, true);
      previouslyFocused?.focus?.();
    });
  });
}
