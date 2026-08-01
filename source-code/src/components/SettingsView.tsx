import { createSignal, createEffect, createMemo, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Trash2, RefreshCw, Plus, Snowflake } from "lucide-solid";
import type { AppInfo, AppSettings, FlatpakRemote } from "../types";
import { SOURCES, OPT_IN_SOURCES, SNAP_CHANNELS } from "../types";
import { LANGUAGES } from "../i18n";
import { SourceIcon } from "../iconMap";
import { useI18n } from "../hooks/useI18n";

const SECTIONS = ["discover", "game_launchers", "pentest_tools", "drivers", "update"];

/** Sources with a "is the underlying tool actually installed" check —
 * apt is assumed present (this is a Debian-based OS) and appimage has no
 * single binary to check for (it's a feed + GitHub Releases, not a CLI),
 * so only these get the "not detected" treatment. */
const STATUS_COMMANDS: Record<string, string> = {
  flatpak: "is_flatpak_available",
  snap: "is_snap_available",
  brew: "is_brew_available",
  hpm: "is_hpm_available",
  nix: "is_nix_available",
};

export function SettingsView(props: {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
  onClearCache: () => void;
  onReset: () => void;
  onBuildNixIndex: () => void;
  busy: boolean;
  appInfo: AppInfo | null;
}) {
  const { t } = useI18n();
  const [draft, setDraft] = createSignal<AppSettings>(props.settings);
  const [sourceStatus, setSourceStatus] = createSignal<Record<string, boolean | null>>({});
  const [newRemoteName, setNewRemoteName] = createSignal("");
  const [newRemoteUrl, setNewRemoteUrl] = createSignal("");

  createEffect(() => setDraft(props.settings));

  for (const [id, cmd] of Object.entries(STATUS_COMMANDS)) {
    invoke<boolean>(cmd)
      .then(ok => setSourceStatus(s => ({ ...s, [id]: ok })))
      .catch(() => setSourceStatus(s => ({ ...s, [id]: null })));
  }

  const dirty = createMemo(() => JSON.stringify(draft()) !== JSON.stringify(props.settings));

  const toggleSource = (id: string) => {
    setDraft(d => ({
      ...d,
      enabled_sources: d.enabled_sources.includes(id)
        ? d.enabled_sources.filter(s => s !== id)
        : [...d.enabled_sources, id],
    }));
  };

  const addRemote = () => {
    const name = newRemoteName().trim();
    const url = newRemoteUrl().trim();
    if (!name || !url) return;
    setDraft(d => {
      const remotes: FlatpakRemote[] = [...d.flatpak_remotes.filter(r => r.name !== name), { name, url }];
      return { ...d, flatpak_remotes: remotes };
    });
    setNewRemoteName(""); setNewRemoteUrl("");
  };

  const removeRemote = (name: string) => {
    setDraft(d => ({
      ...d,
      flatpak_remotes: d.flatpak_remotes.filter(r => r.name !== name),
      flatpak_default_remote: d.flatpak_default_remote === name
        ? (d.flatpak_remotes.find(r => r.name !== name)?.name ?? "flathub")
        : d.flatpak_default_remote,
    }));
  };

  return (
    <div class="view settings-view">
      <h1 class="view-title">{t("settings.title")}</h1>

      <section class="settings-section">
        <h2 class="settings-heading">{t("settings.language")}</h2>
        <div class="lang-pills">
          <For each={LANGUAGES}>
            {l => (
              <button class={`tag-pill ${draft().language === l.id ? "active" : ""}`}
                onClick={() => setDraft(d => ({ ...d, language: l.id }))}>
                {l.label}
              </button>
            )}
          </For>
        </div>
      </section>

      <section class="settings-section">
        <h2 class="settings-heading">{t("settings.sources")}</h2>
        <p class="settings-hint">{t("settings.sourcesHint")}</p>
        <div class="source-toggle-grid">
          <For each={SOURCES}>
            {s => {
              const status = () => sourceStatus()[s.id] ?? null;
              return (
                <label class="source-toggle">
                  <input type="checkbox" checked={draft().enabled_sources.includes(s.id)}
                    onChange={() => toggleSource(s.id)} />
                  <SourceIcon source={s.id} size={15} />
                  <span>{s.label}</span>
                  <Show when={status() !== null}>
                    <span class={`brew-status ${status() ? "brew-status--ok" : "brew-status--missing"}`}>
                      {status() ? t("settings.sourceDetected") : t("settings.sourceNotDetected")}
                    </span>
                  </Show>
                  <Show when={OPT_IN_SOURCES[s.id]}>
                    <span class="settings-hint source-optin-note">{OPT_IN_SOURCES[s.id]}</span>
                  </Show>
                </label>
              );
            }}
          </For>
        </div>
        <p class="settings-hint">{t("settings.brewNote")}</p>
      </section>

      <section class="settings-section">
        <h2 class="settings-heading">{t("settings.ratings")}</h2>
        <label class="settings-checkbox">
          <input type="checkbox" checked={draft().ratings_enabled}
            onChange={e => setDraft(d => ({ ...d, ratings_enabled: e.currentTarget.checked }))} />
          {t("settings.ratingsToggle")}
        </label>
        <p class="settings-hint">{t("settings.ratingsHint")}</p>
      </section>

      <section class="settings-section">
        <h2 class="settings-heading">{t("settings.defaultSection")}</h2>
        <select class="settings-input" value={draft().default_section}
          onChange={e => setDraft(d => ({ ...d, default_section: e.currentTarget.value }))}>
          <For each={SECTIONS}>
            {s => <option value={s}>{t(`nav.${s}`) || t("discover.title")}</option>}
          </For>
        </select>
      </section>

      <section class="settings-section">
        <h2 class="settings-heading">{t("settings.flatpakRemotes")}</h2>
        <p class="settings-hint">{t("settings.flatpakRemotesHint")}</p>
        <div class="remote-list">
          <For each={draft().flatpak_remotes}>
            {r => (
              <div class="remote-row">
                <label class="settings-checkbox" style={{ flex: "0 0 auto" }}>
                  <input type="radio" name="default-remote" checked={draft().flatpak_default_remote === r.name}
                    onChange={() => setDraft(d => ({ ...d, flatpak_default_remote: r.name }))} />
                </label>
                <span class="remote-name">{r.name}</span>
                <span class="remote-url">{r.url}</span>
                <button class="btn-uninstall" title={t("btn.uninstall")} onClick={() => removeRemote(r.name)}>
                  <Trash2 size={13} />
                </button>
              </div>
            )}
          </For>
        </div>
        <div class="remote-row remote-row--add">
          <input class="settings-input" placeholder={t("settings.remoteNamePlaceholder")}
            value={newRemoteName()} onInput={e => setNewRemoteName(e.currentTarget.value)} />
          <input class="settings-input" placeholder="https://…/repo.flatpakrepo"
            value={newRemoteUrl()} onInput={e => setNewRemoteUrl(e.currentTarget.value)} />
          <button class="btn btn-install" onClick={addRemote}><Plus size={13} /></button>
        </div>
        <label class="settings-label">{t("settings.flatpakBranch")}</label>
        <input class="settings-input" placeholder="stable" value={draft().flatpak_default_branch}
          onInput={e => setDraft(d => ({ ...d, flatpak_default_branch: e.currentTarget.value }))} />
        <p class="settings-hint">{t("settings.flatpakBranchHint")}</p>
      </section>

      <section class="settings-section">
        <h2 class="settings-heading">{t("settings.snapChannel")}</h2>
        <select class="settings-input" value={draft().snap_default_channel}
          onChange={e => setDraft(d => ({ ...d, snap_default_channel: e.currentTarget.value }))}>
          <For each={SNAP_CHANNELS}>{c => <option value={c}>{c}</option>}</For>
        </select>
        <p class="settings-hint">{t("settings.snapChannelHint")}</p>
      </section>

      <section class="settings-section">
        <h2 class="settings-heading">{t("settings.mirrors")}</h2>
        <label class="settings-label">{t("settings.aptMirror")}</label>
        <input class="settings-input" placeholder="deb.debian.org" value={draft().apt_mirror}
          onInput={e => setDraft(d => ({ ...d, apt_mirror: e.currentTarget.value }))} />
        <p class="settings-hint">{t("settings.aptMirrorHint")}</p>
        <label class="settings-checkbox">
          <input type="checkbox" checked={draft().check_updates_on_startup}
            onChange={e => setDraft(d => ({ ...d, check_updates_on_startup: e.currentTarget.checked }))} />
          {t("settings.startupUpdates")}
        </label>
      </section>

      <button class="btn btn-install settings-save-btn" disabled={!dirty()} onClick={() => props.onSave(draft())}>
        {t("btn.save")}
      </button>

      <section class="settings-section">
        <h2 class="settings-heading">{t("settings.maintenance")}</h2>
        <p class="settings-hint">{t("settings.clearCacheHint")}</p>
        <button class="btn btn-uninstall-wide" disabled={props.busy} onClick={props.onClearCache}>
          <Trash2 size={14} /> {props.busy ? t("btn.clearing") : t("btn.clearCache")}
        </button>
        <p class="settings-hint" style={{ "margin-top": "14px" }}>
          {t("settings.refreshAppImageCatalog")}
        </p>
        <button class="btn btn-uninstall-wide" onClick={() => invoke("refresh_appimage_feed").catch(() => {})}>
          <RefreshCw size={14} /> {t("settings.refreshAppImageCatalog")}
        </button>
        <p class="settings-hint" style={{ "margin-top": "14px" }}>
          {t("settings.buildNixIndexHint")}
        </p>
        <button class="btn btn-uninstall-wide" disabled={props.busy} onClick={props.onBuildNixIndex}>
          <Snowflake size={14} /> {props.busy ? t("btn.working") : t("settings.buildNixIndex")}
        </button>
        <p class="settings-hint" style={{ "margin-top": "14px" }}>{t("settings.reset")}</p>
        <button class="btn btn-uninstall-wide" onClick={props.onReset}>
          <RefreshCw size={14} /> {t("settings.reset")}
        </button>
      </section>

      <section class="settings-section">
        <h2 class="settings-heading">{t("settings.about")}</h2>
        <div class="about-grid">
          <div><span class="about-label">{t("settings.version")}</span><span>{props.appInfo?.version ?? "—"}</span></div>
          <div><span class="about-label">{t("settings.targetRelease")}</span><span>{props.appInfo?.target_release ?? "—"}</span></div>
        </div>
      </section>
    </div>
  );
}
