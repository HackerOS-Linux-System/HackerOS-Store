import { createSignal, Show } from "solid-js";
import { SourceIcon } from "../iconMap";
import { sourceColor } from "../types";

export function AppIcon(props: { icon?: string | null; source: string; size?: number }) {
  const [failed, setFailed] = createSignal(false);
  const size = () => props.size ?? 40;
  return (
    <Show
      when={props.icon && !failed()}
      fallback={
        <div class="app-icon-fallback" style={{ width: `${size()}px`, height: `${size()}px`, color: sourceColor(props.source) }}>
          <SourceIcon source={props.source} size={Math.round(size() * 0.55)} />
        </div>
      }
    >
      <img src={props.icon!} class="app-icon-img" style={{ width: `${size()}px`, height: `${size()}px` }}
        onError={() => setFailed(true)} alt="" />
    </Show>
  );
}
