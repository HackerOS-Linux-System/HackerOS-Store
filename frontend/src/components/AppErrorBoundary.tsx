import { ErrorBoundary, type JSX } from "solid-js";
import { AlertTriangle } from "lucide-solid";
import { useI18n } from "../hooks/useI18n";

/**
 * Wraps the whole app. Previously an unexpected render error anywhere in
 * the 52 KB App.tsx would blank the entire window with no recovery path
 * short of relaunching the app. Solid's `<ErrorBoundary>` catches render
 * errors in its subtree and lets us show a small recovery UI instead —
 * installed apps and saved settings are untouched (they live in the Rust
 * backend / on disk, not in the crashed component tree), so "reload this
 * view" is a safe, complete fix for the person using the app.
 */
export function AppErrorBoundary(props: { children: JSX.Element }) {
  const { t } = useI18n();
  return (
    <ErrorBoundary
      fallback={(err, reset) => (
        <div class="error-boundary">
          <AlertTriangle size={32} />
          <h1>{t("error.boundaryTitle")}</h1>
          <p>{t("error.boundaryBody")}</p>
          <pre class="error-boundary-detail">{String(err?.message ?? err)}</pre>
          <button class="btn btn-install" onClick={reset}>{t("btn.retry")}</button>
        </div>
      )}
    >
      {props.children}
    </ErrorBoundary>
  );
}
