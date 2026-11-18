import { createSignal, createEffect, onMount, Show, For } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { X, Loader2, CheckCircle, Trash2, ExternalLink, ImageOff, ShieldAlert } from "lucide-solid";
import type { AppDetails, AppSettings, DiscoverItem, LocalReview, RatingInfo } from "../types";
import { sourceColor, SNAP_CHANNELS } from "../types";
import { AppIcon } from "./AppIcon";
import { StarRating } from "./StarRating";
import { ReviewForm } from "./ReviewForm";
import { useI18n } from "../hooks/useI18n";
import { useFocusTrap } from "../hooks/useFocusTrap";

export function AppDetailModal(props: {
  item: DiscoverItem;
  onClose: () => void;
  settings: AppSettings;
  isInstalled: boolean;
  busy: boolean;
  onInstall: (opts?: { remote?: string; branch?: string; channel?: string }) => void;
  onUninstall: () => void;
  addToast: (type: "success" | "error", message: string) => void;
}) {
  const { t } = useI18n();
  const [details, setDetails] = createSignal<AppDetails | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [reviews, setReviews] = createSignal<LocalReview[]>([]);
  const [selRemote, setSelRemote] = createSignal(props.settings.flatpak_default_remote);
  const [selBranch, setSelBranch] = createSignal(props.settings.flatpak_default_branch);
  const [selChannel, setSelChannel] = createSignal(props.settings.snap_default_channel);
  let closeBtn: HTMLButtonElement | undefined;
  let modalRef: HTMLDivElement | undefined;

  useFocusTrap(() => modalRef);
  onMount(() => closeBtn?.focus());

  createEffect(() => {
    const { package_id, source, name } = props.item;
    let cancelled = false;
    setLoading(true);
    invoke<AppDetails>("get_app_details", { packageId: package_id, source, name })
      .then(d => { if (!cancelled) setDetails(d); })
      .catch(() => {})
      .finally(() => { if (!cancelled) setLoading(false); });
    invoke<LocalReview[]>("get_reviews", { source, packageId: package_id })
      .then(r => { if (!cancelled) setReviews(r); })
      .catch(() => {});
    return () => { cancelled = true; };
  });

  function onRatingSubmitted(rating: RatingInfo) {
    setDetails(d => (d ? { ...d, local_rating: rating } : d));
    invoke<LocalReview[]>("get_reviews", { source: props.item.source, packageId: props.item.package_id })
      .then(setReviews).catch(() => {});
  }

  function install() {
    if (props.item.source === "flatpak") {
      props.onInstall({ remote: selRemote() || undefined, branch: selBranch() || undefined });
    } else if (props.item.source === "snap") {
      props.onInstall({ channel: selChannel() || undefined });
    } else {
      props.onInstall();
    }
  }

  return (
    <div class="modal-overlay" onClick={props.onClose}>
      <div ref={modalRef} class="detail-modal" role="dialog" aria-modal="true" aria-label={props.item.name} onClick={e => e.stopPropagation()}>
        <button ref={closeBtn} class="detail-close" onClick={props.onClose} aria-label={t("detail.close")}><X size={16} /></button>
        <Show
          when={!loading()}
          fallback={<div class="discover-spinner detail-loading"><Loader2 size={26} class="spin" /><span>{t("detail.loading")}</span></div>}
        >
          <div class="detail-header">
            <AppIcon icon={details()?.icon} source={props.item.source} size={64} />
            <div class="detail-header-body">
              <h1 class="detail-name">{details()?.name ?? props.item.name}</h1>
              <span class="discover-source detail-source-badge"
                style={{ "border-color": sourceColor(props.item.source), color: sourceColor(props.item.source) }}>
                {t(`discover.source.${props.item.source}`)}
              </span>
              <Show when={details()?.summary}><p class="detail-summary">{details()!.summary}</p></Show>
            </div>
            <div class="detail-header-actions">
              <Show when={!props.busy} fallback={<button class="btn btn-installing" disabled><Loader2 size={13} class="spin" /></button>}>
                <Show
                  when={!props.isInstalled}
                  fallback={
                    <div class="install-done-wrap">
                      <button class="btn btn-installed" disabled><CheckCircle size={13} /> {t("btn.installed")}</button>
                      <button class="btn-uninstall" onClick={props.onUninstall} title={t("btn.uninstall")}><Trash2 size={13} /></button>
                    </div>
                  }
                >
                  <button class="btn btn-install" onClick={install}>{t("btn.install")}</button>
                </Show>
              </Show>
            </div>
          </div>

          <Show when={props.item.source === "snap" && details()?.confinement === "classic"}>
            <p class="view-sub discover-warning">
              <ShieldAlert size={14} /> {t("detail.classicConfinement")}
            </p>
          </Show>

          <Show when={!props.isInstalled && props.item.source === "flatpak" && props.settings.flatpak_remotes.length > 0}>
            <div class="detail-meta-grid">
              <div>
                <span class="about-label">{t("detail.remote")}</span>
                <select value={selRemote()} onChange={e => setSelRemote(e.currentTarget.value)}>
                  <For each={props.settings.flatpak_remotes}>
                    {r => <option value={r.name}>{r.name}</option>}
                  </For>
                </select>
              </div>
              <div>
                <span class="about-label">{t("detail.branch")}</span>
                <input type="text" placeholder="stable" value={selBranch()}
                  onInput={e => setSelBranch(e.currentTarget.value)} />
              </div>
            </div>
          </Show>

          <Show when={!props.isInstalled && props.item.source === "snap"}>
            <div class="detail-meta-grid">
              <div>
                <span class="about-label">{t("detail.channel")}</span>
                <select value={selChannel()} onChange={e => setSelChannel(e.currentTarget.value)}>
                  <For each={SNAP_CHANNELS}>
                    {c => <option value={c}>{c}</option>}
                  </For>
                </select>
              </div>
            </div>
          </Show>

          <Show when={props.settings.ratings_enabled} fallback={<div class="no-rating">{t("detail.ratingsOff")}</div>}>
            <Show when={props.item.source === "flatpak"}>
              <div class="rating-row">
                <span class="about-label">{t("detail.odrsRating")}</span>
                <Show when={details()?.rating} fallback={<div class="no-rating">{t("detail.noRating")}</div>}>
                  <StarRating rating={details()!.rating!} size={16} />
                </Show>
              </div>
            </Show>
            <Show when={details()?.local_rating}>
              <div class="rating-row">
                <span class="about-label">{t("detail.yourRatingSaved")}</span>
                <StarRating rating={details()!.local_rating!} size={16} />
              </div>
            </Show>
          </Show>

          <div class="detail-meta-grid">
            <Show when={details()?.version}><div><span class="about-label">{t("detail.version")}</span><span>{details()!.version}</span></div></Show>
            <Show
              when={details()?.size}
              fallback={
                // apt/flatpak/snap/brew always *should* report a size —
                // if one's missing there it's just this item, so the row
                // stays hidden like before. hpm/nix structurally never
                // report one at all (no download-size metadata to query
                // before install), so say that plainly instead of a
                // silently blank row that looks like a loading glitch.
                <Show when={props.item.source === "hpm" || props.item.source === "nix"}>
                  <div><span class="about-label">{t("detail.size")}</span><span class="detail-size-unknown">{t("detail.sizeUnavailable")}</span></div>
                </Show>
              }
            >
              <div><span class="about-label">{t("detail.size")}</span><span>{details()!.size}</span></div>
            </Show>
            <Show when={details()?.license}><div><span class="about-label">{t("detail.license")}</span><span>{details()!.license}</span></div></Show>
            <Show when={details()?.homepage}>
              <div>
                <span class="about-label">{t("detail.homepage")}</span>
                <a href={details()!.homepage} target="_blank" rel="noreferrer" class="detail-link">
                  {t("detail.homepage")} <ExternalLink size={11} />
                </a>
              </div>
            </Show>
          </div>

          <h2 class="section-heading">{t("detail.screenshots")}</h2>
          <Show
            when={details()?.screenshots && details()!.screenshots.length > 0}
            fallback={<div class="no-screenshots"><ImageOff size={18} /> {t("detail.noScreenshots")}</div>}
          >
            <div class="screenshot-gallery">
              <For each={details()!.screenshots}>
                {s => <img src={s} class="screenshot-img" alt="" onError={e => { (e.target as HTMLImageElement).style.display = "none"; }} />}
              </For>
            </div>
          </Show>

          <h2 class="section-heading">{t("detail.description")}</h2>
          <p class="detail-description">{details()?.description || details()?.summary || t("info.noInfo")}</p>

          <ReviewForm
            source={props.item.source}
            packageId={props.item.package_id}
            localRating={details()?.local_rating}
            reviews={reviews()}
            onSubmitted={onRatingSubmitted}
            addToast={props.addToast}
          />
        </Show>
      </div>
    </div>
  );
}
