import { render } from "solid-js/web";
import App from "./App";
import { I18nProvider } from "./hooks/useI18n";
import { AppErrorBoundary } from "./components/AppErrorBoundary";

render(
  () => (
    <I18nProvider initial="en">
      <AppErrorBoundary>
        <App />
      </AppErrorBoundary>
    </I18nProvider>
  ),
  document.getElementById("root") as HTMLElement,
);
