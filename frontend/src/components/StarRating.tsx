import { For } from "solid-js";
import { Star } from "lucide-solid";
import type { RatingInfo } from "../types";
import { useI18n } from "../hooks/useI18n";

export function StarRating(props: { rating: RatingInfo; size?: number }) {
  const { t } = useI18n();
  const full = () => Math.round(props.rating.average);
  return (
    <div class="star-rating" role="img" aria-label={t("a11y.starRating", { n: props.rating.average.toFixed(1) })}>
      <For each={[1, 2, 3, 4, 5]}>
        {i => (
          <Star size={props.size ?? 14} fill={i <= full() ? "currentColor" : "none"}
            class={i <= full() ? "star-filled" : "star-empty"} />
        )}
      </For>
      <span class="star-rating-text">{props.rating.average.toFixed(1)} ({props.rating.count})</span>
    </div>
  );
}
