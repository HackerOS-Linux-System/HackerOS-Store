import { createSignal, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Star, Loader2 } from "lucide-solid";
import type { LocalReview, RatingInfo } from "../types";
import { useI18n } from "../hooks/useI18n";

export function ReviewForm(props: {
  source: string;
  packageId: string;
  localRating: RatingInfo | null | undefined;
  reviews: LocalReview[];
  onSubmitted: (rating: RatingInfo) => void;
  addToast: (type: "success" | "error", message: string) => void;
}) {
  const { t } = useI18n();
  const [hoverStars, setHoverStars] = createSignal(0);
  const [picked, setPicked] = createSignal(0);
  const [comment, setComment] = createSignal("");
  const [submitting, setSubmitting] = createSignal(false);

  const submit = async () => {
    if (picked() < 1) return;
    setSubmitting(true);
    try {
      const rating = await invoke<RatingInfo>("submit_rating", {
        source: props.source,
        packageId: props.packageId,
        stars: picked(),
        comment: comment().trim() || null,
      });
      props.onSubmitted(rating);
      props.addToast("success", t("toast.ratingSaved"));
      setComment("");
      setPicked(0);
    } catch (err) {
      props.addToast("error", String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div class="review-form">
      <h2 class="section-heading">{t("detail.yourRating")}</h2>
      <div class="star-picker" role="radiogroup" aria-label={t("detail.yourRating")}>
        <For each={[1, 2, 3, 4, 5]}>
          {i => (
            <button
              type="button"
              class="star-picker-btn"
              role="radio"
              aria-checked={picked() === i}
              aria-label={t("a11y.starRating", { n: i })}
              onMouseEnter={() => setHoverStars(i)}
              onMouseLeave={() => setHoverStars(0)}
              onClick={() => setPicked(i)}
            >
              <Star size={22} fill={(hoverStars() || picked()) >= i ? "currentColor" : "none"}
                class={(hoverStars() || picked()) >= i ? "star-filled" : "star-empty"} />
            </button>
          )}
        </For>
      </div>
      <textarea class="review-comment-input" placeholder={t("detail.commentPlaceholder")}
        value={comment()} onInput={e => setComment(e.currentTarget.value)} maxLength={500} rows={2} />
      <button class="btn btn-install" disabled={picked() < 1 || submitting()} onClick={submit}>
        <Show when={submitting()} fallback={t("btn.submitRating")}>
          <Loader2 size={13} class="spin" /> {t("btn.submitRating")}
        </Show>
      </button>
      <p class="settings-hint">{t("detail.localRatingNote")}</p>

      <Show when={props.reviews.length > 0}>
        <h3 class="settings-heading" style={{ "margin-top": "16px" }}>{t("detail.recentReviews")}</h3>
        <ul class="review-list">
          <For each={props.reviews.slice(0, 10)}>
            {r => (
              <li class="review-list-item">
                <span class="review-stars">{"★".repeat(r.stars)}{"☆".repeat(5 - r.stars)}</span>
                <Show when={r.comment}><span class="review-comment">{r.comment}</span></Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={props.reviews.length === 0}>
        <p class="settings-hint">{t("detail.noReviews")}</p>
      </Show>
    </div>
  );
}
