import { WifiOff } from "lucide-solid";
import { useI18n } from "../hooks/useI18n";

/** Shown app-wide (not just in Discover) whenever `navigator.onLine` is
 * false, so the "why isn't this working" moment happens once, at the top
 * of the app, instead of separately in every place that happens to need
 * the network. */
export function OfflineBanner() {
  const { t } = useI18n();
  return (
    <div class="offline-banner" role="status">
      <WifiOff size={14} />
      <span>{t("offline.banner")}</span>
    </div>
  );
}
