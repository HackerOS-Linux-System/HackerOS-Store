#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinSet;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

mod security;
mod ratings;
mod history;
mod queue_store;
mod hpm;
mod hnm;
mod appimage;
mod pkgbackend;

use security::{validate_pkg_token, validate_display_name};

// ─── Event payloads ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallProgress {
    pub step: String,
    pub message: String,
    pub progress: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogLine {
    pub stream: String,
    pub line: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstalledState {
    pub key: String,
    pub installed: bool,
    pub version: Option<String>,
}

/// A single result row in the Discover browse/search view. Unlike the old
/// hardcoded "featured apps" list, every row here comes from a live query
/// against the enabled package sources
/// (apt/flatpak/snap/brew/hpm/appimage) — there is no static catalog
/// backing this type any more.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscoverResult {
    pub name: String,
    pub version: String,
    pub desc: String,
    pub source: String,        // "apt" | "flatpak" | "snap" | "brew" | "hpm" | "appimage"
    pub package_id: String,    // id used to install/uninstall/query details ("owner/repo" for appimage)
    pub size: Option<String>,
    pub icon: Option<String>,  // "data:image/png;base64,..." | None (frontend falls back to a source badge icon)
}

/// One source's problem during a Discover query — shown alongside results
/// rather than just silently reducing the result count. Distinguishes "this
/// source is fine, it just has 0 matches" (no `SourceIssue` at all) from
/// "this source didn't get a chance to answer" (timeout/unavailable/error),
/// which previously looked identical to the person: fewer results, no
/// explanation. The blanket "every source failed" case the frontend
/// already handled is still just `results.is_empty() && !issues.is_empty()`
/// with `issues.len() == enabled_sources.len()`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceIssue {
    pub source: String,
    /// "timeout" | "unavailable" | "error"
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DiscoverResponse {
    pub results: Vec<DiscoverResult>,
    pub issues: Vec<SourceIssue>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryDef {
    pub id: String,
    pub label: String,
    pub icon: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RatingInfo {
    pub average: f32,
    pub count: u32,
}

/// Full detail payload for the app detail view (screenshots, long
/// description, license, homepage, community rating...). Fetched lazily
/// only when the person clicks into an app, exactly like GNOME
/// Software / Plasma Discover do — the browse/search list only ever carries
/// the lightweight `DiscoverResult` summary.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppDetails {
    pub id: String,
    pub name: String,
    pub source: String,
    pub package_id: String,
    pub summary: String,
    pub description: String,
    pub icon: Option<String>,
    pub screenshots: Vec<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub categories: Vec<String>,
    pub size: Option<String>,
    pub rating: Option<RatingInfo>,
    /// Locally-stored community rating (this machine's own submitted
    /// ratings/reviews). Unlike `rating` (ODRS, Flatpak-only), this is
    /// populated for every source — apt, flatpak, snap, and brew.
    pub local_rating: Option<RatingInfo>,
    /// Snap only: "strict" | "classic" | "devmode", parsed from
    /// `snap info`. `Some("classic")` is what makes `snap_install`
    /// automatically add `--classic` instead of failing with a raw CLI
    /// error — surfaced here too so the frontend can show a "requires
    /// classic confinement (broader system access)" notice before install.
    #[serde(default)]
    pub confinement: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct InstalledSets {
    pub apt: Vec<String>,
    pub flatpak: Vec<String>,
    pub snap: Vec<String>,
    pub brew: Vec<String>,
    #[serde(default)]
    pub hpm: Vec<String>,
    /// Packages installed via `hnm` (HackerOS Nix Manager), keyed by the
    /// bare nixpkgs attribute name (e.g. "ripgrep"), same as `package_id`
    /// for this source.
    #[serde(default)]
    pub nix: Vec<String>,
    /// Installed AppImages, keyed by "owner/repo" (matches `package_id`
    /// for this source) rather than a display name — same convention
    /// `flatpak` already uses (app-id, not label).
    #[serde(default)]
    pub appimage: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FlatpakRemote {
    /// The name flatpak knows it by, e.g. "flathub" or "flathub-beta" —
    /// used verbatim as the `<remote>` argument to `flatpak remote-add`/
    /// `install`/`remote-info`, so it goes through `validate_pkg_token`
    /// wherever it's used in a command.
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    /// UI language. Supported: "en", "pl". Frontend owns the actual string
    /// tables; the backend only persists the chosen code.
    #[serde(default = "default_language")]
    pub language: String,
    /// Configured Flatpak remotes. Defaults to just Flathub, but a person
    /// can add Flathub Beta (`https://dl.flathub.org/beta-repo/flathub-beta.flatpakrepo`),
    /// a regional mirror, or any other ostree remote alongside it —
    /// `ensure_flatpak` adds all of them, not just one hardcoded remote.
    #[serde(default = "default_flatpak_remotes")]
    pub flatpak_remotes: Vec<FlatpakRemote>,
    /// Which configured remote (by `FlatpakRemote.name`) Discover installs/
    /// searches/shows remote-info against by default.
    #[serde(default = "default_flatpak_default_remote")]
    pub flatpak_default_remote: String,
    /// Default branch (`flatpak install <remote> <id>//<branch>`) used when
    /// a Discover install doesn't explicitly pick one. Empty string means
    /// "the remote's own default branch" (almost always "stable" — flatpak
    /// itself decides, we just don't force one).
    #[serde(default)]
    pub flatpak_default_branch: String,
    /// Deprecated, superseded by `flatpak_remotes`/`flatpak_default_remote`.
    /// Kept only so a `settings.json` written by an older version of this
    /// app still deserializes instead of silently losing a person's custom
    /// mirror URL — `current_settings()` migrates it into `flatpak_remotes`
    /// the first time it's read. Nothing else reads this field any more.
    #[serde(default = "default_flatpak_remote")]
    pub flatpak_remote_url: String,
    /// Optional custom APT mirror (host only, e.g. "deb.debian.org").
    /// Empty string = leave system sources.list untouched.
    #[serde(default)]
    pub apt_mirror: String,
    /// Automatically check for HackerOS system updates on startup.
    #[serde(default = "default_true")]
    pub check_updates_on_startup: bool,
    /// Which package sources Discover should query. Subset of
    /// ["apt","flatpak","snap","brew","hpm","nix","appimage"]. Lets a
    /// person turn off a source they don't have installed (or don't
    /// trust) instead of always paying the query cost / seeing errors
    /// for it.
    /// "appimage" is deliberately *not* in the default set (see
    /// `default_sources`) — unlike the others it depends on a
    /// third-party community feed and does its own desktop-file/icon
    /// integration on install, so it's opt-in rather than on by default.
    /// "nix" (via `hnm`) is likewise opt-in: its local package index
    /// (`~/.local/share/hnm/pkgdb.tsv`) has to be built once with
    /// `hnm update` before search returns anything, and a from-scratch
    /// Nix bootstrap on first install can take several minutes — both
    /// unlike the always-ready system package managers.
    /// `#[serde(default = ...)]` here (and on the fields below) means a
    /// settings.json written by an older version of this app — before these
    /// fields existed — still deserializes successfully instead of falling
    /// back to full factory defaults and silently discarding the person's
    /// saved mirror/language/etc. preferences.
    #[serde(default = "default_sources")]
    pub enabled_sources: Vec<String>,
    /// Default snap channel/track (`--channel=`) used when a Discover
    /// install doesn't explicitly pick one. "stable" = flatpak-style
    /// default, so `snap_install` simply omits `--channel` in that case.
    #[serde(default = "default_snap_channel")]
    pub snap_default_channel: String,
    /// Whether to fetch community star ratings from the GNOME ODRS service
    /// for Flatpak apps in the detail view. Off by default for anyone who
    /// doesn't want the app phoning home at all.
    #[serde(default = "default_true")]
    pub ratings_enabled: bool,
    /// Which section the app opens on when launched.
    #[serde(default = "default_section")]
    pub default_section: String,
    /// UI color theme: "dark" | "light" | "system". Frontend owns the
    /// actual CSS variables for each; the backend only persists the
    /// chosen value, same as `language`.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Default answer for "how do you want this Dev Tools entry
    /// installed?": "ask" (default — show the Local/Container prompt
    /// every time), "local", or "container". Purely a frontend concern
    /// (DevToolsView.tsx) — the backend just persists it, same as
    /// `theme`/`language`.
    #[serde(default = "default_dev_tools_mode")]
    pub dev_tools_default_mode: String,
}

fn default_language() -> String { "en".into() }
fn default_flatpak_remote() -> String { "https://dl.flathub.org/repo/flathub.flatpakrepo".into() }
fn default_flatpak_remotes() -> Vec<FlatpakRemote> {
    vec![FlatpakRemote { name: "flathub".into(), url: default_flatpak_remote() }]
}
fn default_flatpak_default_remote() -> String { "flathub".into() }
fn default_snap_channel() -> String { "stable".into() }
fn default_true() -> bool { true }
fn default_sources() -> Vec<String> { vec!["apt".into(), "flatpak".into(), "snap".into(), "brew".into(), "hpm".into()] }
fn default_section() -> String { "discover".into() }
fn default_theme() -> String { "dark".into() }
fn default_dev_tools_mode() -> String { "ask".into() }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            flatpak_remotes: default_flatpak_remotes(),
            flatpak_default_remote: default_flatpak_default_remote(),
            flatpak_default_branch: String::new(),
            flatpak_remote_url: default_flatpak_remote(),
            apt_mirror: String::new(),
            check_updates_on_startup: true,
            enabled_sources: vec!["apt".into(), "flatpak".into(), "snap".into(), "brew".into(), "hpm".into()],
            snap_default_channel: default_snap_channel(),
            ratings_enabled: true,
            default_section: "discover".into(),
            theme: default_theme(),
            dev_tools_default_mode: default_dev_tools_mode(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppInfo {
    pub version: String,
    pub name: String,
    pub target_release: String,
    /// Which package manager Discover's "apt" source, driver installs, and
    /// Debian-native pentest tools actually talk to on this machine:
    /// "apt", "hammer (normal)", or "hammer (oci)" — see `pkgbackend.rs`.
    /// Surfaced in the UI (About/Settings) so it's never a silent swap.
    pub pkg_backend: String,
}

// ─── Job state (used for cooperative cancellation) ────────────────────────────
//
// Only one long-running install/uninstall/update job runs at a time in this
// app (the UI disables concurrent actions), so a single shared slot is enough.
// `current_pid` holds the OS pid of the process currently attached to
// run_streaming (if any); `cancel_requested` is checked between steps of a
// multi-step job (e.g. ensure_wine -> download -> wineboot -> install) so a
// cancellation lands even while no child process is alive at that instant.

#[derive(Default)]
pub struct JobState {
    pub current_pid: AsyncMutex<Option<u32>>,
    pub cancel_requested: Arc<AtomicBool>,
}

const CANCELLED_MSG: &str = "Cancelled by user.";

fn is_cancelled(app: &tauri::AppHandle) -> bool {
    app.state::<JobState>().cancel_requested.load(Ordering::SeqCst)
}

fn check_cancel(app: &tauri::AppHandle) -> Result<(), String> {
    if is_cancelled(app) { Err(CANCELLED_MSG.to_string()) } else { Ok(()) }
}

fn reset_job(app: &tauri::AppHandle) {
    app.state::<JobState>().cancel_requested.store(false, Ordering::SeqCst);
}

// ─── Discover result cache ────────────────────────────────────────────────────
//
// Every category click or search keystroke used to re-run a full round of
// apt/flatpak/snap/brew/hpm/nix/appimage subprocess calls from scratch,
// even for a category the person just looked at 5 seconds ago. A
// short-TTL in-memory cache makes flipping back to a recently-viewed
// category or re-typing a recent search near-instant, without risking
// showing very stale data (entries expire after CACHE_TTL regardless).
// See `SourceCacheState` below for the finer-grained, per-source layer
// underneath this one.
const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(120);

#[derive(Default)]
pub struct DiscoverCacheState {
    pub entries: AsyncMutex<std::collections::HashMap<String, (std::time::Instant, DiscoverResponse)>>,
}

async fn cache_get(app: &tauri::AppHandle, key: &str) -> Option<DiscoverResponse> {
    let state = app.state::<DiscoverCacheState>();
    let map = state.entries.lock().await;
    map.get(key).and_then(|(t, v)| if t.elapsed() < CACHE_TTL { Some(v.clone()) } else { None })
}

pub async fn cache_set(app: &tauri::AppHandle, key: String, val: DiscoverResponse) {
    let state = app.state::<DiscoverCacheState>();
    let mut map = state.entries.lock().await;
    map.insert(key, (std::time::Instant::now(), val));
    if map.len() > 60 {
        // Cheap best-effort eviction so a long session can't grow this
        // unboundedly — drop whatever the map's own (arbitrary) iteration
        // order gives us first rather than tracking true LRU order.
        let stale: Vec<String> = map.keys().take(map.len() - 40).cloned().collect();
        for k in stale { map.remove(&k); }
    }
}

// ─── Per-source Discover cache ────────────────────────────────────────────────
//
// The combined-response cache above is keyed by the *whole* enabled-sources
// set, which is great for "flip back to a search/category I just looked
// at" but is all-or-nothing: toggling one source on/off — or one slow/
// erroring source, like nix bootstrapping Nix on first use, or the
// AppImage feed being momentarily slow — misses (or poisons) the combined
// entry for *every* source in that query, not just the one that changed.
// Previously this meant apt/flatpak/snap/brew effectively got a "recently
// viewed" speedup while nix (freshly opt-in, so its presence in
// `enabled_sources` changes the combined key more often) rarely got to
// reuse anything.
//
// This second, finer-grained cache stores each *individual* source's own
// raw search result (or `SourceIssue`) — before dedupe/icon-enrichment —
// so a change to one source's availability/enabled-state doesn't force
// every other, already-fine source to be re-queried too. `run_all_sources`
// consults this per source; the combined cache above still exists on top
// of it for the common case (exact same query + exact same enabled set)
// so that case still costs zero work, not even the per-source lookups or
// `dedupe_and_enrich`'s icon-index scans.
#[derive(Default)]
pub struct SourceCacheState {
    pub entries: AsyncMutex<std::collections::HashMap<String, (std::time::Instant, Result<Vec<DiscoverResult>, SourceIssue>)>>,
}

async fn source_cache_get(app: &tauri::AppHandle, key: &str) -> Option<Result<Vec<DiscoverResult>, SourceIssue>> {
    let state = app.state::<SourceCacheState>();
    let map = state.entries.lock().await;
    map.get(key).and_then(|(t, v)| if t.elapsed() < CACHE_TTL { Some(v.clone()) } else { None })
}

async fn source_cache_set(app: &tauri::AppHandle, key: String, val: Result<Vec<DiscoverResult>, SourceIssue>) {
    let state = app.state::<SourceCacheState>();
    let mut map = state.entries.lock().await;
    map.insert(key, (std::time::Instant::now(), val));
    if map.len() > 400 {
        // Same best-effort eviction as `cache_set`, just a higher
        // ceiling since keys here are one per (source, query) pair
        // rather than one per whole multi-source query.
        let stale: Vec<String> = map.keys().take(map.len() - 250).cloned().collect();
        for k in stale { map.remove(&k); }
    }
}

/// Drops every cached entry — both the per-source cache above and any
/// combined-response entry that could include this source's results —
/// for `source`. Call this after anything that changes what a source's
/// search actually returns *without* going through the normal
/// install/remove flow: `hnm update` rebuilding the local nixpkgs index,
/// and the AppImage feed's manual refresh are the two that exist today.
/// Without this, running "Build Nix index" (or "Refresh AppImage
/// catalog") and immediately searching again could still show the exact
/// same stale results — or the exact same "index not built" issue — for
/// up to `CACHE_TTL`, which would make the button feel like it did
/// nothing.
async fn invalidate_source_cache(app: &tauri::AppHandle, source: &str) {
    {
        let state = app.state::<SourceCacheState>();
        let mut map = state.entries.lock().await;
        let prefix = format!("{source}:");
        map.retain(|k, _| !k.starts_with(&prefix));
    }
    {
        let state = app.state::<DiscoverCacheState>();
        let mut map = state.entries.lock().await;
        // Key shape is "search:{query}:{sources}" / "browse:{cat}:{sources}"
        // (see `discover_search`/`discover_browse`) — the enabled-sources
        // list is always the last ':'-separated segment, itself
        // comma-separated; an exact token match (not a substring check)
        // avoids e.g. a hypothetical "unix" source matching "nix".
        map.retain(|k, _| {
            match k.rsplit(':').next() {
                Some(sources) => !sources.split(',').any(|s| s == source),
                None => true,
            }
        });
    }
}

// ─── Emit helpers ─────────────────────────────────────────────────────────────

fn emit_prog(app: &tauri::AppHandle, step: &str, msg: &str, pct: f32) {
    let _ = app.emit("install_progress", InstallProgress {
        step: step.into(), message: msg.into(), progress: pct,
    });
}

fn emit_log(app: &tauri::AppHandle, stream: &str, line: &str) {
    let _ = app.emit("install_log", LogLine {
        stream: stream.into(), line: line.into(),
    });
}

// ─── Streaming process runner ─────────────────────────────────────────────────

async fn run_streaming(app: &tauri::AppHandle, argv: &[&str]) -> Result<(), String> {
    run_streaming_env(app, argv, &[]).await
}

/// Same as [`run_streaming`], but sets the given environment variables on
/// the child process directly (`Command::env`) instead of the old approach
/// of prefixing `VAR=value ...` onto a `sh -c` string. Used for the Wine
/// launchers (`WINEPREFIX`/`WINEARCH`/`WINEDEBUG`) so no shell is involved
/// and no interpolated path/value can be mis-parsed as shell syntax.
async fn run_streaming_env(app: &tauri::AppHandle, argv: &[&str], envs: &[(&str, &str)]) -> Result<(), String> {
    check_cancel(app)?;

    let mut cmd = Command::new(argv[0]);
    for a in &argv[1..] { cmd.arg(a); }
    for (k, v) in envs { cmd.env(k, v); }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);

    let mut child = cmd.spawn()
        .map_err(|e| format!("spawn '{}': {}", argv[0], e))?;

    let pid = child.id();
    if let Some(pid) = pid {
        *app.state::<JobState>().current_pid.lock().await = Some(pid);
    }

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let app_o = app.clone();
    let app_e = app.clone();

    let t1 = tokio::spawn(async move {
        let mut r = BufReader::new(stdout).lines();
        while let Ok(Some(l)) = r.next_line().await { emit_log(&app_o, "stdout", &l); }
    });
    let t2 = tokio::spawn(async move {
        let mut r = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = r.next_line().await { emit_log(&app_e, "stderr", &l); }
    });

    let status = child.wait().await.map_err(|e| e.to_string());
    let _ = tokio::join!(t1, t2);
    *app.state::<JobState>().current_pid.lock().await = None;

    if is_cancelled(app) { return Err(CANCELLED_MSG.to_string()); }

    let status = status.map_err(|e| e.to_string())?;
    if status.success() { Ok(()) }
    else { Err(format!("'{}' exited {}", argv[0], status.code().unwrap_or(-1))) }
}

/// Low-level "run a real shell command" primitive. Every call site that
/// used to build a `sh -c` string by `format!`-interpolating a package
/// name/id/URL into it has been migrated to argv-based `run_streaming`/
/// `priv_run` calls instead (no shell involved -> nothing to escape).
/// `run_sh` is kept only for the rare case a genuine shell feature (a pipe,
/// `&&`, redirection) is unavoidable. **Never** interpolate unsanitized
/// input into `cmd` — validate with `security::validate_pkg_token` first
/// and, if a value still must be embedded in the string, escape it with
/// `security::sh_quote`. Currently unused; kept for future use under that
/// contract.
#[allow(dead_code)]
async fn run_sh(app: &tauri::AppHandle, cmd: &str) -> Result<(), String> {
    run_streaming(app, &["sh", "-c", cmd]).await
}

/// Runs a privileged command, trying `pkexec` first (GUI polkit prompt) and
/// falling back to `sudo`. If both fail, returns a clear, actionable error
/// (instead of silently swallowing the pkexec failure like before) so the
/// UI can show the person exactly what to run by hand.
async fn priv_run(app: &tauri::AppHandle, args: &[&str]) -> Result<(), String> {
    check_cancel(app)?;
    let cmd_str = args.join(" ");

    let has_pkexec = Command::new("which").arg("pkexec").output().await
        .map(|o| o.status.success()).unwrap_or(false);
    if has_pkexec {
        let mut full = vec!["pkexec"];
        full.extend_from_slice(args);
        if run_streaming(app, &full).await.is_ok() { return Ok(()); }
        if is_cancelled(app) { return Err(CANCELLED_MSG.to_string()); }
        emit_log(app, "info", "pkexec failed or was dismissed — falling back to sudo...");
    }

    let mut full = vec!["sudo", "-n"];
    full.extend_from_slice(args);
    // First try non-interactive sudo (works if NOPASSWD is configured or a
    // credential cache is still warm); if that fails, retry with a normal
    // interactive sudo that can prompt on the controlling TTY.
    if run_streaming(app, &full).await.is_ok() { return Ok(()); }
    if is_cancelled(app) { return Err(CANCELLED_MSG.to_string()); }

    let mut full2 = vec!["sudo"];
    full2.extend_from_slice(args);
    match run_streaming(app, &full2).await {
        Ok(()) => Ok(()),
        Err(sudo_err) => {
            if is_cancelled(app) { return Err(CANCELLED_MSG.to_string()); }
            let msg = format!(
                "Privilege escalation failed: neither pkexec nor sudo could run this command.\n\
                 Please open a terminal and run manually:\n\n  sudo {cmd_str}\n\n(sudo error: {sudo_err})"
            );
            emit_log(app, "error", &msg);
            Err(msg)
        }
    }
}

/// Installs one or more Debian packages. Delegates to [`pkgbackend`],
/// which uses real `apt-get` when it's present on the system and
/// transparently falls back to `hammer` (in whichever of its normal/oci
/// modes is active) otherwise — see `pkgbackend.rs` for the full
/// rationale. Every call site below (drivers, apt-backed pentest tools,
/// Discover's "apt" source, Wine's i386 bootstrap, ...) keeps working
/// unmodified either way.
async fn apt_install(app: &tauri::AppHandle, pkgs: &[&str]) -> Result<(), String> {
    pkgbackend::install(app, pkgs).await
}

/// Removes one or more Debian packages. See [`apt_install`]'s doc comment.
async fn apt_remove(app: &tauri::AppHandle, pkgs: &[&str]) -> Result<(), String> {
    pkgbackend::remove(app, pkgs).await
}

// ─── Flatpak ──────────────────────────────────────────────────────────────────

async fn ensure_flatpak(app: &tauri::AppHandle) -> Result<(), String> {
    let has = Command::new("which").arg("flatpak").output().await
        .map(|o| o.status.success()).unwrap_or(false);
    if !has {
        emit_log(app, "info", "Installing Flatpak...");
        apt_install(app, &["flatpak"]).await?;
    }
    // Note: remote name/URL are operator-chosen settings (not attacker
    // input from search results), but we still avoid the shell here —
    // argv-based Command needs no quoting/escaping at all.
    //
    // Every configured remote gets added (not just a single hardcoded
    // "flathub"), so a person who's added Flathub Beta or a mirror
    // actually gets to search/install from it, not just have it sit
    // unused in Settings.
    for remote in current_settings().flatpak_remotes {
        let name = match validate_pkg_token(&remote.name) { Ok(n) => n, Err(_) => continue };
        let _ = run_streaming(app, &["flatpak", "remote-add", "--if-not-exists", "--user", &name, &remote.url]).await;
        let _ = priv_run(app, &["flatpak", "remote-add", "--if-not-exists", &name, &remote.url]).await;
    }
    Ok(())
}

/// "id" -> "id//branch" when a non-empty branch is given, matching
/// flatpak's own ref syntax for `install`/`update` (e.g.
/// `org.gimp.GIMP//beta`). An empty/`"stable"` branch is left as a bare
/// id, since "stable" is what flatpak defaults to anyway and passing it
/// explicitly isn't necessary.
fn flatpak_ref(id: &str, branch: &str) -> String {
    if branch.is_empty() || branch.eq_ignore_ascii_case("stable") {
        id.to_string()
    } else {
        format!("{id}//{branch}")
    }
}

async fn flatpak_install(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let id = validate_pkg_token(id)?;
    ensure_flatpak(app).await?;
    check_cancel(app)?;
    let settings = current_settings();
    let remote = validate_pkg_token(&settings.flatpak_default_remote).unwrap_or_else(|_| "flathub".into());
    let ref_arg = flatpak_ref(&id, &settings.flatpak_default_branch);
    emit_log(app, "info", &format!("Installing {} from {}...", id, remote));
    emit_prog(app, "install", &format!("Installing {}...", id), 0.3);
    if run_streaming(app, &["flatpak", "install", "-y", "--user", &remote, &ref_arg]).await.is_err() {
        check_cancel(app)?;
        priv_run(app, &["flatpak", "install", "-y", &remote, &ref_arg]).await?;
    }
    emit_prog(app, "done", "Done!", 1.0);
    emit_log(app, "success", "Installation complete.");
    Ok(())
}

async fn flatpak_uninstall(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let id = validate_pkg_token(id)?;
    emit_log(app, "info", &format!("Removing {}...", id));
    emit_prog(app, "uninstall", &format!("Removing {}...", id), 0.3);
    if run_streaming(app, &["flatpak", "uninstall", "-y", "--user", &id]).await.is_err() {
        check_cancel(app)?;
        priv_run(app, &["flatpak", "uninstall", "-y", &id]).await?;
    }
    emit_prog(app, "done", "Removed.", 1.0);
    emit_log(app, "success", "Removed successfully.");
    Ok(())
}

async fn flatpak_remote_info(id: &str) -> serde_json::Value {
    let mut info = serde_json::json!({"size":null,"version":null});
    if id.is_empty() { return info; }
    let remote = validate_pkg_token(&current_settings().flatpak_default_remote).unwrap_or_else(|_| "flathub".into());
    let mut cmd = Command::new("flatpak");
    cmd.args(["remote-info","--user",&remote,id]);
    if let Some(out) = run_timeout(cmd, 6).await {
        let s = String::from_utf8_lossy(&out.stdout).to_string();
        for line in s.lines() {
            if line.contains("Version:") {
                info["version"]=serde_json::json!(line.split(':').nth(1).unwrap_or("").trim());
            }
            if line.contains("Download Size:") || line.contains("Installed Size:") {
                info["size"]=serde_json::json!(line.split(':').nth(1).unwrap_or("").trim());
            }
        }
    }
    info
}

/// Reads the currently-active ostree commit for an installed Flatpak app,
/// used to record a rollback target right after install/uninstall (see
/// `discover_install`). Tries the user installation first, then system —
/// same fallback order everything else in this file uses for Flatpak.
async fn flatpak_current_commit(id: &str) -> Option<String> {
    for scope in [["info", "--user", id].as_slice(), ["info", id].as_slice()] {
        let mut cmd = Command::new("flatpak");
        cmd.args(scope);
        if let Some(out) = run_timeout(cmd, 5).await {
            let text = String::from_utf8_lossy(&out.stdout);
            let commit = text.lines()
                .find_map(|l| l.trim_start().strip_prefix("Commit:"))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            if commit.is_some() { return commit; }
        }
    }
    None
}

/// Pins an installed Flatpak app to a specific previously-recorded commit —
/// this is what makes Flatpak rollback possible at all, unlike apt where
/// downgrading just means re-installing an older version string. Requires
/// that commit to still be present in the remote's history (flatpak/ostree
/// prune old commits over time, so a very old rollback can legitimately
/// fail with "not found" — surfaced as-is rather than hidden).
async fn flatpak_rollback_to_commit(app: &tauri::AppHandle, id: &str, commit: &str) -> Result<(), String> {
    let id = validate_pkg_token(id)?;
    let commit = validate_pkg_token(commit)?;
    let arg = format!("--commit={commit}");
    emit_log(app, "info", &format!("Pinning {id} to commit {}...", &commit[..commit.len().min(12)]));
    if run_streaming(app, &["flatpak", "update", "-y", "--user", &arg, &id]).await.is_err() {
        priv_run(app, &["flatpak", "update", "-y", &arg, &id]).await?;
    }
    Ok(())
}

// ─── Wine ─────────────────────────────────────────────────────────────────────

async fn ensure_wine(app: &tauri::AppHandle) -> Result<(), String> {
    let has = Command::new("which").arg("wine").output().await
        .map(|o| o.status.success()).unwrap_or(false);
    if !has {
        emit_log(app, "info", "Wine not found — installing...");
        pkgbackend::add_foreign_arch(app, "i386").await?;
        pkgbackend::update_index(app).await?;
        apt_install(app, &["wine", "wine32", "wine64", "winetricks", "libgl1"]).await?;
        return Ok(());
    }
    // Was: a direct `dpkg-query -W -f='${Status}' wine32` check for the
    // exact "install ok installed" string. `pkgbackend::installed_version`
    // is a coarser "is it installed at all" proxy (no distinction from a
    // half-configured dpkg state), but that distinction was never actually
    // used here beyond "installed or not" — and unlike a raw dpkg-query
    // call, it degrades correctly to `hammer`'s own installed-package
    // tracking on a system that doesn't have apt-get.
    let wine32_ok = pkgbackend::installed_version("wine32").await.is_some();
    if !wine32_ok {
        emit_log(app, "info", "wine32 not found — installing...");
        pkgbackend::add_foreign_arch(app, "i386").await?;
        pkgbackend::update_index(app).await?;
        apt_install(app, &["wine32", "libgl1"]).await
            .map_err(|_| "Failed to install wine32. Please run manually:\n  sudo dpkg --add-architecture i386\n  sudo apt-get update\n  sudo apt-get install wine32\n(or, on a hammer-based system: sudo hammer dpkg-arch add i386 && sudo hammer sync && sudo hammer install wine32)".to_string())?;
    }
    Ok(())
}

/// Best-effort SHA-256 verification for downloaded Wine installers.
///
/// GOG/Battle.net/EA do not publish stable, versioned checksums for their
/// bootstrap installers (the binaries are updated silently on their CDN), so
/// this cannot be a hard allow-list the way a Linux package's checksum
/// would be. Instead: we always compute and log the SHA-256 of what was
/// downloaded (so it is auditable / reportable), and only *hard-fail* when a
/// known-bad or known-good hash has been explicitly configured below.
const KNOWN_GOOD_SHA256: &[(&str, &str)] = &[
    // ("gog", "‹sha256 of a verified GOG Galaxy bootstrap installer›"),
    // ("battlenet", "‹sha256 of a verified Battle.net-Setup.exe›"),
    // ("ea", "‹sha256 of a verified EAappInstaller.exe›"),
];

async fn verify_download(app: &tauri::AppHandle, id: &str, path: &str) -> Result<(), String> {
    let out = Command::new("sha256sum").arg(path).output().await
        .map_err(|e| format!("sha256sum failed: {e}"))?;
    let digest = String::from_utf8_lossy(&out.stdout)
        .split_whitespace().next().unwrap_or("").to_string();
    if digest.is_empty() {
        emit_log(app, "stderr", "Could not compute a checksum for the downloaded installer.");
        return Ok(());
    }
    emit_log(app, "info", &format!("Downloaded installer SHA-256: {digest}"));
    match KNOWN_GOOD_SHA256.iter().find(|(k, _)| *k == id) {
        Some((_, expected)) if *expected != digest => {
            Err(format!(
                "Checksum mismatch for {id} installer!\n  expected: {expected}\n  got:      {digest}\n\
                 Refusing to run an installer that does not match the pinned checksum."
            ))
        }
        Some(_) => {
            emit_log(app, "success", "Checksum matches the pinned known-good value.");
            Ok(())
        }
        None => {
            emit_log(app, "stderr",
                "No pinned checksum is configured for this installer (the vendor does not publish stable hashes). \
                 Proceeding, but you may want to verify this binary yourself before trusting it.");
            Ok(())
        }
    }
}

// ─── Non-free repos ───────────────────────────────────────────────────────────

async fn ensure_nonfree(app: &tauri::AppHandle) -> Result<(), String> {
    match pkgbackend::backend().await {
        pkgbackend::Backend::Apt => ensure_nonfree_apt(app).await,
        pkgbackend::Backend::Hammer => ensure_nonfree_hammer(app).await,
    }
}

async fn ensure_nonfree_apt(app: &tauri::AppHandle) -> Result<(), String> {
    let ok = Command::new("sh").arg("-c")
        .arg("grep -r non-free /etc/apt/sources.list /etc/apt/sources.list.d/ 2>/dev/null | grep -v '#' | grep -q non-free && echo yes")
        .output().await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "yes")
        .unwrap_or(false);
    if ok { return Ok(()); }
    let cn = Command::new("lsb_release").arg("-sc").output().await.ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        // HackerOS tracks Debian testing/trixie-based forks rather than the
        // stable "bookworm" release, so that is the safer fallback codename
        // when `lsb_release` is unavailable or returns something unexpected
        // (e.g. inside a minimal container image).
        .unwrap_or_else(|| "trixie".into());
    let mirror = current_settings().apt_mirror;
    let host = if mirror.trim().is_empty() { "deb.debian.org".to_string() } else { mirror };
    let line = format!("deb http://{host}/debian {cn} main contrib non-free non-free-firmware\n");
    emit_log(app, "info", &format!("Adding non-free repositories for '{cn}'..."));
    // Write the sources-list line to a normal, unprivileged temp file, then
    // use `install` (argv-based, no shell) to place it with root
    // ownership/permissions. This avoids the `echo ... | sudo tee ...`
    // shell pipe the previous version used, which required interpolating
    // untrusted-ish values (mirror host / codename) directly into a
    // `sh -c` string.
    let tmp_path = std::env::temp_dir().join(format!("hackeros-nonfree-{}.list", std::process::id()));
    std::fs::write(&tmp_path, &line).map_err(|e| format!("Failed writing temp sources file: {e}"))?;
    let tmp_str = tmp_path.to_string_lossy().to_string();
    let result = priv_run(app, &["install", "-m", "0644", &tmp_str, "/etc/apt/sources.list.d/hackeros-nonfree.list"]).await;
    let _ = std::fs::remove_file(&tmp_path);
    result?;
    priv_run(app, &["apt-get", "update", "-qq"]).await?;
    Ok(())
}

/// Hammer equivalent of [`ensure_nonfree_apt`].
///  - normal mode: `hammer repo add <uri> <suite> <components…>` is a
///    documented drop-in for an apt `deb` line (it even accepts the raw
///    apt-style "deb <uri> <suite> <comps>" string), so this adds the
///    same `contrib non-free non-free-firmware` components against
///    `/etc/hammer/sources-list.hk` and re-syncs.
///  - oci mode: package sources for an OSTree base image are baked in at
///    build/deploy time from the *image's* `/etc/apt/sources.list(.d)`,
///    not something this Store can add live at runtime the way a normal
///    apt/hammer sources file can be. Logs an explanation and leaves the
///    index alone rather than silently pretending it worked.
async fn ensure_nonfree_hammer(app: &tauri::AppHandle) -> Result<(), String> {
    if matches!(pkgbackend::hammer_mode().await, pkgbackend::HammerMode::Oci) {
        emit_log(
            app,
            "info",
            "hammer oci mode: non-free components come from the base image's own build-time \
             sources, not something the Store can add live — skipping.",
        );
        return Ok(());
    }
    let Some(bin) = pkgbackend::hammer_bin_pub().await else {
        return Err("hammer is not installed on this system.".to_string());
    };
    let already = Command::new(&bin)
        .args(["repo", "list"])
        .output().await
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("non-free"))
        .unwrap_or(false);
    if already { return Ok(()); }
    let cn = Command::new("lsb_release").arg("-sc").output().await.ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "trixie".into());
    let mirror = current_settings().apt_mirror;
    let host = if mirror.trim().is_empty() { "deb.debian.org".to_string() } else { mirror };
    let uri = format!("http://{host}/debian");
    emit_log(app, "info", &format!("Adding non-free repositories for '{cn}' via hammer..."));
    priv_run(app, &[bin.as_str(), "repo", "add", uri.as_str(), cn.as_str(), "main", "contrib", "non-free", "non-free-firmware"]).await?;
    pkgbackend::update_index(app).await?;
    Ok(())
}

// ─── Distrobox / Kali ─────────────────────────────────────────────────────────

async fn ensure_distrobox(app: &tauri::AppHandle) -> Result<(), String> {
    let ok = Command::new("which").arg("distrobox").output().await
        .map(|o| o.status.success()).unwrap_or(false);
    if ok { return Ok(()); }
    emit_log(app, "info", "Installing distrobox...");
    apt_install(app, &["distrobox"]).await
}

async fn kali_exists() -> bool {
    Command::new("distrobox").arg("list").output().await
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("kali-pentest"))
        .unwrap_or(false)
}

async fn ensure_kali(app: &tauri::AppHandle) -> Result<(), String> {
    if kali_exists().await { return Ok(()); }
    emit_log(app, "info", "Creating Kali Linux container (first run ~5 min)...");
    emit_prog(app, "install", "Creating Kali container...", 0.15);
    run_streaming(app, &["distrobox", "create", "--image", "kalilinux/kali-rolling", "--name", "kali-pentest", "--yes"]).await?;
    let _ = run_streaming(app, &["distrobox", "enter", "kali-pentest", "--", "sudo", "apt-get", "update", "-qq"]).await;
    Ok(())
}

// ─── Pentest tool catalog (single source of truth) ─────────────────────────────
//
// Both `in_debian()` (install-strategy decision) and `check_all_installed`
// (installed-status tracking) read from this one table, so they cannot drift
// apart the way the old two-lists-in-two-places design did. `true` means the
// tool is normally packaged in Debian testing/trixie repos and gets
// installed directly via apt; `false` means it is installed inside the
// dedicated Kali Linux distrobox container instead. Verify with
// `apt-cache policy <pkg>` against your target release before relying on
// this for anything security-critical — Debian's archive contents shift
// between releases, and a few of these are educated guesses rather than
// verified facts (this environment has no network access to check live).
// Package-name/repo-availability specifics below (which of these live in
// plain Debian main vs. only in Kali's repos) were classified from general
// packaging knowledge, not verified against a live apt/Kali mirror from
// this offline sandbox — same caveat as the AppImageHub feed schema
// elsewhere in this file. Where in doubt, a tool was classified `false`
// (Kali container) rather than `true`: a wrongly-`false` tool just means
// one extra container round-trip the first time it's installed, while a
// wrongly-`true` tool fails outright with "unable to locate package" on
// plain Debian. Spot-check before shipping if exact accuracy matters more
// than that fallback behavior.
const PENTEST_CATALOG: &[(&str, bool)] = &[
    // ── Network / recon ──
    ("nmap",          true),
    ("masscan",       true),
    ("arp-scan",      true),
    ("netdiscover",   true),
    ("hping3",        true),
    ("netcat",        true),
    ("ncat",          true),
    ("socat",         true),
    ("rustscan",      false),
    ("naabu",         false),
    ("wireshark",     true),
    ("tcpdump",       true),
    ("tshark",        true),
    ("tcpflow",       true),
    ("scapy",         true),
    ("fping",         true),
    ("zmap",          true),
    ("unicornscan",   false),
    ("dnsenum",       false),
    ("fierce",        false),
    ("p0f",           true),
    ("dmitry",        false),
    ("nbtscan",       true),
    // ── Web application testing ──
    ("burpsuite",     false),
    ("zaproxy",       true),
    ("sqlmap",        true),
    ("nikto",         true),
    ("gobuster",      true),
    ("wpscan",        false),
    ("beef-xss",      false),
    ("feroxbuster",   false),
    ("ffuf",          false),
    ("nuclei",        false),
    ("httpx",         false),
    ("katana",        false),
    ("dirb",          true),
    ("dirsearch",     false),
    ("whatweb",       true),
    ("wafw00f",       true),
    ("commix",        false),
    ("xsser",         true),
    ("joomscan",      false),
    ("droopescan",    false),
    ("sslyze",        true),
    ("testssl.sh",    true),
    ("wfuzz",         false),
    ("wapiti",        true),
    ("skipfish",      false),
    ("xsstrike",      false),
    ("dalfox",        false),
    // ── Password / credential attacks ──
    ("john",          true),
    ("hydra",         true),
    ("hashcat",       true),
    ("medusa",        true),
    ("crunch",        true),
    ("cewl",          true),
    ("patator",       true),
    ("ncrack",        true),
    ("hashid",        true),
    ("ophcrack",      true),
    ("fcrackzip",     true),
    ("pdfcrack",      true),
    ("rarcrack",      true),
    ("bruteforce-luks", true),
    // ── Wireless ──
    ("aircrack-ng",   true),
    ("kismet",        true),
    ("reaver",        true),
    ("wifite",        true),
    ("cowpatty",      true),
    ("pixiewps",      true),
    ("hcxdumptool",   true),
    ("hcxtools",      true),
    ("bully",         false),
    ("mdk4",          false),
    ("fern-wifi-cracker", false),
    // ── MITM / network attacks ──
    ("bettercap",     false),
    ("responder",     false),
    ("ettercap",      true),
    ("sslstrip",      false),
    ("mitmproxy",     true),
    ("dsniff",        true),
    ("dnschef",       false),
    ("yersinia",      true),
    ("macchanger",    true),
    ("tcpreplay",     true),
    ("netsniff-ng",   true),
    // ── Exploitation / Windows / AD ──
    ("metasploit",    false),
    ("impacket",      true),
    ("crackmapexec",  false),
    ("evil-winrm",    false),
    ("bloodhound",    false),
    ("enum4linux",    true),
    ("smbclient",     true),
    ("ldap-utils",    true),
    ("smbmap",        false),
    // ── OSINT ──
    ("theharvester",  true),
    ("maltego",       false),
    ("recon-ng",      true),
    ("dnsrecon",      true),
    ("subfinder",     false),
    ("amass",         true),
    ("sherlock",      false),
    ("spiderfoot",    false),
    ("exiftool",      true),
    ("whois",         true),
    ("gitleaks",      false),
    ("h8mail",        false),
    // ── Tunneling / proxy ──
    ("proxychains",   true),
    ("tor",           true),
    ("chisel",        false),
    ("stunnel",       true),
    ("sshuttle",      true),
    ("iodine",        true),
    // ── Vulnerability scanning ──
    ("sslscan",       true),
    ("openvas",       false),
    ("trivy",         false),
    // ── Forensics / reverse engineering / malware ──
    ("volatility",    true),
    ("autopsy",       true),
    ("binwalk",       true),
    ("foremost",      true),
    ("steghide",      true),
    ("radare2",       true),
    ("ghidra",        false),
    ("gdb",           true),
    ("yara",          true),
    ("clamav",        true),
    ("mat2",          true),
    ("testdisk",      true),
    ("photorec",      true),
    ("sleuthkit",     true),
    ("bulk-extractor", true),
    ("hexedit",       true),
    ("upx",           true),
    ("apktool",       false),
    ("jadx",          false),
    // ── System hardening / auditing ──
    ("lynis",         true),
    ("rkhunter",      true),
    ("chkrootkit",    true),
];

/// A handful of tools have a common/binary name that doesn't match their
/// actual Debian package name. This maps catalog id -> real apt package
/// name for those cases, so `apt-get install/remove` and the dpkg-based
/// installed-state check target the correct package while the UI, wrapper
/// script, and desktop file keep using the familiar tool name.
const APT_NAME_OVERRIDES: &[(&str, &str)] = &[
    ("exiftool", "libimage-exiftool-perl"),
    ("impacket", "python3-impacket"),
    ("scapy",    "python3-scapy"),
    ("stunnel",  "stunnel4"),
    ("photorec", "testdisk"),
    ("upx",      "upx-ucl"),
];

fn apt_pkg_name(tool: &str) -> String {
    APT_NAME_OVERRIDES.iter().find(|(t, _)| *t == tool).map(|(_, p)| p.to_string()).unwrap_or_else(|| tool.to_string())
}

fn in_debian(name: &str) -> bool {
    PENTEST_CATALOG.iter().find(|(n, _)| *n == name).map(|(_, d)| *d).unwrap_or(false)
}

fn pentest_tool_names() -> Vec<&'static str> {
    PENTEST_CATALOG.iter().map(|(n, _)| *n).collect()
}

// ─── check_all_installed ─────────────────────────────────────────────────────

#[tauri::command]
async fn check_all_installed() -> Vec<InstalledState> {
    let mut out: Vec<InstalledState> = Vec::new();

    // Flatpak: one call to list all
    let fp_text = Command::new("flatpak")
        .args(["list", "--columns=application,version"])
        .output().await
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let mut fp: std::collections::HashMap<String,String> = std::collections::HashMap::new();
    for line in fp_text.lines() {
        let mut p = line.splitn(2, '\t');
        let id  = p.next().unwrap_or("").trim().to_string();
        let ver = p.next().unwrap_or("").trim().to_string();
        if !id.is_empty() { fp.insert(id, ver); }
    }

    let flatpak_items: &[(&str, &str)] = &[
        ("game_launchers::Steam",            "com.valvesoftware.Steam"),
        ("game_launchers::Lutris",           "net.lutris.Lutris"),
        ("game_launchers::Heroic",           "com.heroicgameslauncher.hgl"),
        ("game_launchers::Epic Games Store", "com.heroicgameslauncher.hgl"),
        ("game_launchers::Bottles",          "com.usebottles.bottles"),
    ];
    for (key, id) in flatpak_items {
        let installed = fp.contains_key(*id);
        let version   = fp.get(*id).cloned();
        out.push(InstalledState { key: key.to_string(), installed, version });
    }

    // apt/dpkg (or hammer, if that's what this system uses — see
    // `pkgbackend.rs`): one bulk lookup for all installed packages.
    let apt = pkgbackend::installed_map().await;

    // Pentest tools that are apt/Debian-native: check via dpkg (fast, always run).
    for name in pentest_tool_names() {
        if !in_debian(name) { continue; }
        let key = format!("pentest_tools::{name}");
        let pkg = apt_pkg_name(name);
        let installed = apt.contains_key(&pkg);
        let version   = apt.get(&pkg).cloned();
        out.push(InstalledState { key, installed, version });
    }

    // Pentest tools that live inside the Kali distrobox container: check
    // `command -v` for all of them in a single batched call so we don't pay
    // a per-tool distrobox-enter cost. Skipped entirely (fast "not
    // installed") if the container doesn't exist yet, and bounded by a
    // timeout so a slow/broken container never blocks the whole UI refresh.
    let container_tools: Vec<&str> = pentest_tool_names().into_iter().filter(|n| !in_debian(n)).collect();
    let mut container_installed: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    if kali_exists().await {
        let names_sh = container_tools.join(" ");
        let script = format!(
            "for t in {names_sh}; do command -v \"$t\" >/dev/null 2>&1 && echo \"$t:yes\" || echo \"$t:no\"; done"
        );
        let fut = Command::new("distrobox")
            .args(["enter", "kali-pentest", "--", "sh", "-c", &script])
            .output();
        if let Ok(Ok(out)) = tokio::time::timeout(std::time::Duration::from_secs(6), fut).await {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some((name, state)) = line.split_once(':') {
                    container_installed.insert(name.to_string(), state == "yes");
                }
            }
        }
    }
    for name in container_tools {
        let key = format!("pentest_tools::{name}");
        let installed = container_installed.get(name).copied().unwrap_or(false);
        out.push(InstalledState { key, installed, version: None });
    }

    let apt_items: &[(&str, &str)] = &[
        ("drivers::NVIDIA Driver",       "nvidia-driver"),
        ("drivers::AMD Driver",          "firmware-amd-graphics"),
        ("drivers::Intel Driver",        "intel-media-va-driver"),
        ("drivers::WiFi — Broadcom",     "broadcom-sta-dkms"),
        ("drivers::WiFi — Realtek",      "rtl8812au-dkms"),
        ("drivers::Firmware (non-free)", "firmware-linux-nonfree"),
    ];
    for (key, pkg) in apt_items {
        let installed = apt.contains_key(*pkg);
        let version   = apt.get(*pkg).cloned();
        out.push(InstalledState { key: key.to_string(), installed, version });
    }

    // HackerOS Ecosystem: installed state is this app's own marker files
    // (see the module doc comment above `install_hackeros_tool`), not
    // anything queried from `hacker` itself.
    let marker_dir = hackeros_ecosystem_marker_dir();
    for (name, slug, _) in HACKEROS_ECOSYSTEM_CATALOG {
        let key = format!("hackeros_ecosystem::{name}");
        let installed = marker_dir.join(slug).is_file();
        out.push(InstalledState { key, installed, version: None });
    }

    // Dev Tools — "Local" variants: check via dpkg (fast, always run;
    // `apt` is the same bulk lookup already built above).
    for (local, _container, pkg, _bin) in DEV_TOOLS_CATALOG {
        let key = format!("dev_tools::{local}");
        let installed = apt.contains_key(*pkg);
        let version   = apt.get(*pkg).cloned();
        out.push(InstalledState { key, installed, version });
    }

    // Dev Tools — "Container" variants: same batched `command -v` probe
    // inside `hackeros-devbox` the pentest tools use for their Kali
    // container above, skipped entirely if that container doesn't exist
    // yet and bounded by a timeout for the same reasons.
    let mut dev_container_installed: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    if devbox_exists().await {
        let bins: Vec<&str> = DEV_TOOLS_CATALOG.iter().map(|(_, _, _, bin)| *bin).collect();
        let names_sh = bins.join(" ");
        let script = format!(
            "for t in {names_sh}; do command -v \"$t\" >/dev/null 2>&1 && echo \"$t:yes\" || echo \"$t:no\"; done"
        );
        let fut = Command::new("distrobox")
            .args(["enter", "hackeros-devbox", "--", "sh", "-c", &script])
            .env("DBX_CONTAINER_MANAGER", "podman")
            .output();
        if let Ok(Ok(out)) = tokio::time::timeout(std::time::Duration::from_secs(6), fut).await {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some((bin, state)) = line.split_once(':') {
                    dev_container_installed.insert(bin.to_string(), state == "yes");
                }
            }
        }
    }
    for (_local, container, _pkg, bin) in DEV_TOOLS_CATALOG {
        let key = format!("dev_tools::{container}");
        let installed = dev_container_installed.get(*bin).copied().unwrap_or(false);
        out.push(InstalledState { key, installed, version: None });
    }
    out
}

// ─── install_package ─────────────────────────────────────────────────────────

#[tauri::command]
async fn install_package(app: tauri::AppHandle, name: String, category: String) -> Result<String, String> {
    // Only `pentest_tools` names are ever interpolated into a raw shell/argv
    // token (apt package name or Kali container command), so only that
    // branch needs the strict `validate_pkg_token` allowlist — it's applied
    // again inside `install_pentest` itself. The other curated categories
    // (game_launchers, drivers, hackeros_ecosystem) only ever use `name` as
    // a match key against a static Rust catalog; a name that doesn't match
    // anything just falls through to "Unknown …" below, so a looser
    // display-name check is enough and doesn't reject legitimate names like
    // "NVIDIA Driver" or "Epic Games Store" that contain a space.
    let name = validate_display_name(&name)?;
    reset_job(&app);
    emit_log(&app, "info", &format!("Starting installation of {}...", name));
    let result = match category.as_str() {
        "game_launchers"     => install_launcher(&app, &name).await,
        "pentest_tools"      => install_pentest(&app, &name).await,
        "drivers"            => install_driver(&app, &name).await,
        "hackeros_ecosystem" => install_hackeros_tool(&app, &name).await,
        "dev_tools"          => install_dev_tool(&app, &name).await,
        _ => Err(format!("Unknown category: {category}")),
    };
    reset_job(&app);
    let pentest_apt_backed = category == "pentest_tools" && in_debian(&name);
    let driver_pkg_list: Option<Vec<&str>> = if category == "drivers" { driver_pkgs(&name).ok().map(|p| p.to_vec()) } else { None };

    if let Some(pkgs) = &driver_pkg_list {
        // Drivers install several apt packages at once (e.g. "NVIDIA Driver"
        // -> nvidia-driver + firmware-misc-nonfree). Record every one of
        // them with its resolved version so a later rollback can re-pin
        // each package individually, instead of only ever being able to
        // roll back single-package curated installs.
        let versions = if result.is_ok() { history::current_apt_versions(pkgs).await } else { vec![] };
        let versions = if versions.is_empty() { None } else { Some(versions) };
        let _ = history::record_multi("install", "apt", &name, &name, None, versions,
            result.is_ok(), result.as_ref().err().cloned());
    } else {
        let (hist_source, hist_pkg_id) = if pentest_apt_backed {
            ("apt".to_string(), apt_pkg_name(&name))
        } else if category == "dev_tools" {
            // Local variants are literally an apt package — piggyback on
            // the same "apt" source/version-pin rollback pentest tools
            // use. Container variants get their own source, same "undo
            // the recorded action" rollback semantics as HackerOS
            // Ecosystem — see history.rs.
            match dev_tool_entry(&name) {
                Some((_, _, pkg, _, false)) => ("apt".to_string(), pkg.to_string()),
                Some((_, _, _, _, true))    => ("dev_tools_container".to_string(), name.clone()),
                None                        => ("curated".to_string(), name.clone()),
            }
        } else if category == "hackeros_ecosystem" {
            // Distinct from plain "curated" (game launchers) so
            // `rollback_entry` can dispatch a `hacker pack`/`hacker unpack`
            // undo specifically for these — see history.rs.
            ("hackeros_ecosystem".to_string(), name.clone())
        } else {
            ("curated".to_string(), name.clone())
        };
        let version = if result.is_ok() && (pentest_apt_backed || hist_source == "apt") {
            history::current_apt_version(&hist_pkg_id).await
        } else { None };
        let _ = history::record("install", &hist_source, &name, &hist_pkg_id, version,
            result.is_ok(), result.as_ref().err().cloned());
    }
    result?;
    emit_log(&app, "success", &format!("{} installed successfully.", name));
    Ok(format!("{name} installed successfully."))
}

// ─── uninstall_package ────────────────────────────────────────────────────────

#[tauri::command]
async fn uninstall_package(app: tauri::AppHandle, name: String, category: String) -> Result<String, String> {
    let name = validate_display_name(&name)?;
    reset_job(&app);
    emit_log(&app, "info", &format!("Removing {}...", name));
    let result = match category.as_str() {
        "game_launchers"     => uninstall_launcher(&app, &name).await,
        "pentest_tools"      => uninstall_pentest(&app, &name).await,
        "drivers"            => uninstall_driver(&app, &name).await,
        "hackeros_ecosystem" => uninstall_hackeros_tool(&app, &name).await,
        "dev_tools"          => uninstall_dev_tool(&app, &name).await,
        _ => Err(format!("Unknown category: {category}")),
    };
    reset_job(&app);
    let hist_source = if (category == "pentest_tools" && in_debian(&name)) || category == "drivers" {
        "apt".to_string()
    } else if category == "dev_tools" {
        match dev_tool_entry(&name) {
            Some((_, _, _, _, false)) => "apt".to_string(),
            Some((_, _, _, _, true))  => "dev_tools_container".to_string(),
            None                      => "curated".to_string(),
        }
    } else if category == "hackeros_ecosystem" {
        "hackeros_ecosystem".to_string()
    } else {
        "curated".to_string()
    };
    let _ = history::record("uninstall", &hist_source, &name, &name, None,
        result.is_ok(), result.as_ref().err().cloned());
    result?;
    emit_log(&app, "success", &format!("{} removed successfully.", name));
    Ok(format!("{name} removed successfully."))
}

// ─── cancel_install ───────────────────────────────────────────────────────────

#[tauri::command]
async fn cancel_install(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<JobState>();
    state.cancel_requested.store(true, Ordering::SeqCst);
    emit_log(&app, "info", "Cancellation requested — stopping current step...");
    let pid = *state.current_pid.lock().await;
    if let Some(pid) = pid {
        let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).output().await;
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let _ = Command::new("kill").args(["-KILL", &pid.to_string()]).output().await;
    }
    Ok(())
}

async fn install_launcher(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    match name {
        "Steam"                       => flatpak_install(app, "com.valvesoftware.Steam").await,
        "Lutris"                      => flatpak_install(app, "net.lutris.Lutris").await,
        "Heroic" | "Epic Games Store" => flatpak_install(app, "com.heroicgameslauncher.hgl").await,
        "Bottles"                     => flatpak_install(app, "com.usebottles.bottles").await,
        "GOG" | "Battle.net" | "EA App" => install_wine_launcher(app, name).await,
        _ => Err(format!("Unknown launcher: {name}")),
    }
}

async fn uninstall_launcher(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    match name {
        "Steam"                       => flatpak_uninstall(app, "com.valvesoftware.Steam").await,
        "Lutris"                      => flatpak_uninstall(app, "net.lutris.Lutris").await,
        "Heroic" | "Epic Games Store" => flatpak_uninstall(app, "com.heroicgameslauncher.hgl").await,
        "Bottles"                     => flatpak_uninstall(app, "com.usebottles.bottles").await,
        "GOG" | "Battle.net" | "EA App" => uninstall_wine_launcher(app, name).await,
        _ => Err(format!("Unknown launcher: {name}")),
    }
}

fn wine_launcher_meta(name: &str) -> (&'static str, &'static str, &'static str) {
    match name {
        "GOG"        => ("gog",       "https://webinstallers.gog.com/galaxy_installer_en.exe",
                         "GOG Galaxy/GalaxyClient.exe"),
        "Battle.net" => ("battlenet", "https://www.battle.net/download/getInstaller?os=win&installer=Battle.net-Setup.exe",
                         "Battle.net/Battle.net.exe"),
        _            => ("ea",        "https://origin-a.akamaihd.net/EA-Desktop-Client-Download/installer-releases/EAappInstaller.exe",
                         "Electronic Arts/EA Desktop/EADesktop.exe"),
    }
}

async fn install_wine_launcher(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    ensure_wine(app).await?;
    check_cancel(app)?;
    let (id, url, exe) = wine_launcher_meta(name);
    let home   = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let dir    = format!("{home}/.hackeros/launchers/{id}");
    let prefix = format!("{dir}/prefix");
    std::fs::create_dir_all(&prefix).ok();
    let installer = format!("{dir}/installer.exe");

    emit_log(app, "info", &format!("Downloading {} installer...", name));
    emit_prog(app, "download", &format!("Downloading {}...", name), 0.1);
    run_streaming(app, &["wget", "-q", "-O", &installer, url]).await?;

    check_cancel(app)?;
    verify_download(app, id, &installer).await?;

    emit_log(app, "info", "Initialising Wine prefix (win32)...");
    emit_prog(app, "wine", "Initialising Wine prefix...", 0.40);
    run_streaming_env(
        app, &["wineboot", "--init"],
        &[("WINEPREFIX", &prefix), ("WINEARCH", "win32"), ("WINEDEBUG", "-all")],
    ).await?;

    check_cancel(app)?;
    emit_log(app, "info", &format!("Running {} installer via Wine...", name));
    emit_prog(app, "wine", &format!("Installing {}...", name), 0.65);
    run_streaming_env(
        app, &["wine", &installer, "/S"],
        &[("WINEPREFIX", &prefix), ("WINEARCH", "win32"), ("WINEDEBUG", "-all")],
    ).await?;

    let ddir     = format!("{home}/.local/share/applications");
    std::fs::create_dir_all(&ddir).ok();
    let exe_path = format!("{prefix}/drive_c/Program Files (x86)/{exe}");
    let desktop  = format!(
        "[Desktop Entry]\nName={name}\nExec=env WINEPREFIX={prefix} WINEARCH=win32 WINEDEBUG=-all wine \"{exe_path}\"\nType=Application\nCategories=Game;\n"
    );
    std::fs::write(format!("{ddir}/{id}.desktop"), desktop).ok();
    emit_prog(app, "done", "Done!", 1.0);
    Ok(())
}

async fn uninstall_wine_launcher(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    let (id, _url, _exe) = wine_launcher_meta(name);
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let dir  = format!("{home}/.hackeros/launchers/{id}");
    emit_prog(app, "uninstall", &format!("Removing {}...", name), 0.3);
    emit_log(app, "info", &format!("Deleting Wine prefix and files for {}...", name));
    let _ = std::fs::remove_dir_all(&dir);
    let desktop = format!("{home}/.local/share/applications/{id}.desktop");
    let _ = std::fs::remove_file(&desktop);
    emit_prog(app, "done", "Removed.", 1.0);
    Ok(())
}

async fn install_pentest(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    let name = validate_pkg_token(name)?;
    let name = name.as_str();
    if in_debian(name) {
        let pkg = apt_pkg_name(name);
        emit_log(app, "info", &format!("Installing {} from Debian repos...", name));
        emit_prog(app, "install", &format!("Installing {}...", name), 0.2);
        apt_install(app, &[pkg.as_str()]).await?;
    } else {
        ensure_distrobox(app).await?;
        check_cancel(app)?;
        ensure_kali(app).await?;
        check_cancel(app)?;
        emit_log(app, "info", &format!("Installing {} in Kali container...", name));
        emit_prog(app, "install", &format!("Installing {} in Kali...", name), 0.5);
        run_streaming(app, &["distrobox", "enter", "kali-pentest", "--", "sudo", "apt-get", "install", "-y", name]).await?;
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        let bin  = format!("{home}/.local/bin");
        std::fs::create_dir_all(&bin).ok();
        let w = format!("{bin}/{name}");
        std::fs::write(&w, format!("#!/bin/sh\ndistrobox enter kali-pentest -- {name} \"$@\"\n")).ok();
        let _ = std::process::Command::new("chmod").args(["755", &w]).output();
        let ddir = format!("{home}/.local/share/applications");
        std::fs::create_dir_all(&ddir).ok();
        std::fs::write(format!("{ddir}/{name}.desktop"),
            format!("[Desktop Entry]\nName={name}\nExec={w}\nType=Application\nCategories=Security;\n")).ok();
    }
    emit_prog(app, "done", "Done!", 1.0);
    Ok(())
}

async fn uninstall_pentest(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    let name = validate_pkg_token(name)?;
    let name = name.as_str();
    if in_debian(name) {
        let pkg = apt_pkg_name(name);
        emit_log(app, "info", &format!("Removing {} (apt)...", name));
        emit_prog(app, "uninstall", &format!("Removing {}...", name), 0.3);
        apt_remove(app, &[pkg.as_str()]).await?;
    } else {
        emit_log(app, "info", &format!("Removing {} from Kali container...", name));
        emit_prog(app, "uninstall", &format!("Removing {}...", name), 0.3);
        if kali_exists().await {
            let _ = run_streaming(app, &["distrobox", "enter", "kali-pentest", "--", "sudo", "apt-get", "remove", "-y", name]).await;
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        let _ = std::fs::remove_file(format!("{home}/.local/bin/{name}"));
        let _ = std::fs::remove_file(format!("{home}/.local/share/applications/{name}.desktop"));
    }
    emit_prog(app, "done", "Removed.", 1.0);
    Ok(())
}

// ─── Dev Tools (Rust/cargo, Node.js/npm, Python, Go, Java, Ruby, PHP, C/C++) ──
//
// Assumes a fresh HackerOS install has none of these toolchains yet (no
// cargo, no npm, ...) and offers two independent ways to get each one,
// chosen per-row rather than as one global setting:
//   - "Local" installs the real apt package straight onto the host —
//     fastest, but its dependencies land directly on the base system.
//   - "Container" keeps the host clean: every container-mode tool
//     installs inside one shared Podman/Distrobox container
//     (`hackeros-devbox`, created on first use, ~2-3 min), the same
//     approach the Kali-only pentest tools above already use, right down
//     to the `~/.local/bin/<tool>` wrapper that forwards a host
//     invocation into the container. `DBX_CONTAINER_MANAGER=podman` is
//     set explicitly on every distrobox call for this container, so it's
//     always Podman doing the work — per the request that started this
//     section — even on a system that also happens to have Docker
//     installed (distrobox otherwise prefers Docker when both exist).
//
// Same host-PATH caveat as the pentest Kali wrappers applies if someone
// installs *both* variants of the same tool: the `~/.local/bin/<tool>`
// wrapper generally shadows the apt-installed binary in `/usr/bin` on
// Debian's default PATH ordering. Neither this nor the pentest container
// tries to detect or warn about that today.
//
// All eight apt package names below (cargo, npm, python3-pip, golang-go,
// default-jdk, ruby-full, php-cli, build-essential) are long-standing,
// widely-mirrored Debian main packages — high confidence these exist in
// Debian testing/trixie without needing non-free or backports, unlike
// some of the pentest catalog's Kali-only picks.
const DEV_TOOLS_CATALOG: &[(&str, &str, &str, &str)] = &[
    // (Local display name, Container display name, apt package, primary binary)
    ("Rust (cargo) — Local",            "Rust (cargo) — Container",            "cargo",           "cargo"),
    ("Node.js (npm) — Local",           "Node.js (npm) — Container",           "npm",             "npm"),
    ("Python (pip) — Local",            "Python (pip) — Container",            "python3-pip",     "pip3"),
    ("Go — Local",                      "Go — Container",                      "golang-go",       "go"),
    ("Java (JDK) — Local",              "Java (JDK) — Container",              "default-jdk",     "javac"),
    ("Ruby (gem) — Local",              "Ruby (gem) — Container",              "ruby-full",       "gem"),
    ("PHP — Local",                     "PHP — Container",                     "php-cli",         "php"),
    ("C/C++ (build-essential) — Local", "C/C++ (build-essential) — Container", "build-essential", "gcc"),
];

/// Looks `name` up against both the local- and container-display-name
/// columns of [`DEV_TOOLS_CATALOG`]. Returns
/// `(local_name, container_name, apt_pkg, primary_bin, is_container)`.
fn dev_tool_entry(name: &str) -> Option<(&'static str, &'static str, &'static str, &'static str, bool)> {
    DEV_TOOLS_CATALOG.iter().find_map(|(local, container, pkg, bin)| {
        if *local == name { Some((*local, *container, *pkg, *bin, false)) }
        else if *container == name { Some((*local, *container, *pkg, *bin, true)) }
        else { None }
    })
}

async fn ensure_podman(app: &tauri::AppHandle) -> Result<(), String> {
    let ok = Command::new("which").arg("podman").output().await
        .map(|o| o.status.success()).unwrap_or(false);
    if ok { return Ok(()); }
    emit_log(app, "info", "Installing podman...");
    apt_install(app, &["podman"]).await
}

async fn devbox_exists() -> bool {
    Command::new("distrobox").arg("list").env("DBX_CONTAINER_MANAGER", "podman")
        .output().await
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("hackeros-devbox"))
        .unwrap_or(false)
}

async fn ensure_devbox(app: &tauri::AppHandle) -> Result<(), String> {
    if devbox_exists().await { return Ok(()); }
    emit_log(app, "info", "Creating the HackerOS Dev Tools container (Podman, first run ~2-3 min)...");
    emit_prog(app, "install", "Creating dev tools container...", 0.15);
    run_streaming_env(
        app,
        &["distrobox", "create", "--image", "debian:trixie-slim", "--name", "hackeros-devbox", "--yes"],
        &[("DBX_CONTAINER_MANAGER", "podman")],
    ).await?;
    let _ = run_streaming_env(
        app,
        &["distrobox", "enter", "hackeros-devbox", "--", "sudo", "apt-get", "update", "-qq"],
        &[("DBX_CONTAINER_MANAGER", "podman")],
    ).await;
    Ok(())
}

async fn install_dev_tool(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    let name = validate_display_name(name)?;
    let (_, _, pkg, bin, container) = dev_tool_entry(&name)
        .ok_or_else(|| format!("Unknown Dev Tools entry: {name}"))?;
    if !container {
        emit_log(app, "info", &format!("Installing {} from Debian repos...", name));
        emit_prog(app, "install", &format!("Installing {}...", name), 0.2);
        apt_install(app, &[pkg]).await?;
    } else {
        ensure_podman(app).await?;
        check_cancel(app)?;
        ensure_distrobox(app).await?;
        check_cancel(app)?;
        ensure_devbox(app).await?;
        check_cancel(app)?;
        emit_log(app, "info", &format!("Installing {} in the dev tools container...", name));
        emit_prog(app, "install", &format!("Installing {} in container...", name), 0.6);
        run_streaming_env(
            app,
            &["distrobox", "enter", "hackeros-devbox", "--", "sudo", "apt-get", "install", "-y", pkg],
            &[("DBX_CONTAINER_MANAGER", "podman")],
        ).await?;
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        let bindir = format!("{home}/.local/bin");
        std::fs::create_dir_all(&bindir).ok();
        let w = format!("{bindir}/{bin}");
        std::fs::write(&w, format!(
            "#!/bin/sh\nDBX_CONTAINER_MANAGER=podman distrobox enter hackeros-devbox -- {bin} \"$@\"\n"
        )).ok();
        let _ = std::process::Command::new("chmod").args(["755", &w]).output();
    }
    emit_prog(app, "done", "Done!", 1.0);
    Ok(())
}

async fn uninstall_dev_tool(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    let name = validate_display_name(name)?;
    let (_, _, pkg, bin, container) = dev_tool_entry(&name)
        .ok_or_else(|| format!("Unknown Dev Tools entry: {name}"))?;
    if !container {
        emit_log(app, "info", &format!("Removing {} (apt)...", name));
        emit_prog(app, "uninstall", &format!("Removing {}...", name), 0.3);
        apt_remove(app, &[pkg]).await?;
    } else {
        emit_log(app, "info", &format!("Removing {} from the dev tools container...", name));
        emit_prog(app, "uninstall", &format!("Removing {}...", name), 0.3);
        if devbox_exists().await {
            let _ = run_streaming_env(
                app,
                &["distrobox", "enter", "hackeros-devbox", "--", "sudo", "apt-get", "remove", "-y", pkg],
                &[("DBX_CONTAINER_MANAGER", "podman")],
            ).await;
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        let _ = std::fs::remove_file(format!("{home}/.local/bin/{bin}"));
    }
    emit_prog(app, "done", "Removed.", 1.0);
    Ok(())
}

/// Flatpak IDs for the game launchers that are Flatpak-based (as opposed to
/// the Wine-based ones handled by `wine_launcher_meta`). Used by
/// `get_package_info` to fetch size/version for the "game_launchers"
/// category.
fn launcher_flatpak_id(name: &str) -> &'static str {
    match name {
        "Steam"                       => "com.valvesoftware.Steam",
        "Lutris"                      => "net.lutris.Lutris",
        "Heroic" | "Epic Games Store" => "com.heroicgameslauncher.hgl",
        "Bottles"                     => "com.usebottles.bottles",
        _ => "",
    }
}

async fn install_driver(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    ensure_nonfree(app).await?;
    check_cancel(app)?;
    let pkgs = driver_pkgs(name)?;
    emit_log(app, "info", &format!("Installing {}...", name));
    emit_prog(app, "install", &format!("Installing {}...", name), 0.3);
    apt_install(app, pkgs).await?;
    emit_prog(app, "done", "Done!", 1.0);
    Ok(())
}

async fn uninstall_driver(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    let pkgs = driver_pkgs(name)?;
    emit_log(app, "info", &format!("Removing {}...", name));
    emit_prog(app, "uninstall", &format!("Removing {}...", name), 0.3);
    apt_remove(app, pkgs).await?;
    emit_prog(app, "done", "Removed.", 1.0);
    Ok(())
}

fn driver_pkgs(name: &str) -> Result<&'static [&'static str], String> {
    Ok(match name {
        "NVIDIA Driver"       => &["nvidia-driver", "firmware-misc-nonfree"],
        "AMD Driver"          => &["firmware-amd-graphics","libgl1-mesa-dri","xserver-xorg-video-amdgpu"],
        "Intel Driver"        => &["firmware-misc-nonfree","intel-media-va-driver","i965-va-driver","libva-drm2"],
        "WiFi — Broadcom"     => &["broadcom-sta-dkms","dkms","linux-headers-amd64"],
        "WiFi — Realtek"      => &["rtl8812au-dkms","dkms","linux-headers-amd64"],
        "Firmware (non-free)" => &["firmware-linux-nonfree","firmware-misc-nonfree","firmware-realtek","firmware-iwlwifi","firmware-atheros"],
        _ => return Err(format!("Unknown driver: {name}")),
    })
}

// ─── HackerOS Ecosystem ──────────────────────────────────────────────────────
//
// A grab-bag of first-party HackerOS tools/add-ons/environments that are
// installed and removed through the system's own `hacker` CLI rather than
// apt/flatpak/snap/etc: `hacker unpack <slug>` / `hacker pack <slug>`.
// Mirrors HACKEROS_ECOSYSTEM in src/data/packages.ts exactly — same names
// in the same order — so the two catalogs can't drift apart. If you add a
// tool here, add the matching row there too.
//
// `hacker` itself does its own state tracking, but exposes no query command
// this app can parse portably, so installed/not-installed here is tracked
// with a small marker file per tool at `~/.hackeros/ecosystem/<slug>` —
// written right after a successful `unpack` and removed right after a
// successful `pack`. That marker is this app's own bookkeeping, not a
// property `hacker` reports back, so it can in principle drift from reality
// if a tool is unpacked/packed some other way outside the Store; the "not
// detected" hint next to the source toggles elsewhere in the app follows
// the same "best effort, not ground truth" philosophy.
const HACKEROS_ECOSYSTEM_CATALOG: &[(&str, &str, bool)] = &[
    // (display name, `hacker` slug, uninstallable)
    ("HackerOS TV",            "hackeros-tv",         true),
    ("Add-ons",                "add-ons",             true),
    ("GS",                     "gs",                  true),
    ("Dev Tools",              "devtools",            true),
    ("Emulators",              "emulators",           true),
    ("Cybersecurity",          "cybersecurity",       true),
    ("Gaming",                 "gaming",              true),
    ("Gaming — Roblox",        "gaming-roblox",       true),
    ("Hacker Mode",            "hacker-mode",         true),
    ("Automatic Updates",      "automatic-updates",   true),
    ("Alacritty Config",       "alacritty-config",    true),
    ("Winboat",                "winboat",             true),
    ("NVIDIA Drivers",         "nvidia-drivers",      true),
    ("HackerOS Containers",    "hackeros-containers", true),
    ("H#",                     "h#",                  true),
    ("H# Utils",               "h#-utils",            true),
    ("HackerOS Builder",       "hackeros-builder",    true),
    ("Isolator",               "isolator",            true),
    // Hydra is install-only — `hacker` offers no way to remove it once
    // unpacked (see the UI's warning next to this row).
    ("Hydra",                  "hydra",               false),
    ("Hammer",                 "hammer",              true),
    ("HackerOS Games",         "hackeros-games",      true),
    ("HexAi",                  "hexai",               true),
    ("HackerDeck",             "hackerdeck",          true),
    ("Blue Environment",       "blue-environment",    true),
    ("HWDE",                   "hwde",                true),
    ("Cybersecurity Mode",     "cybersecurity-mode",  true),
    ("SDE",                    "sde",                 true),
];

fn hackeros_ecosystem_entry(name: &str) -> Option<(&'static str, &'static str, bool)> {
    HACKEROS_ECOSYSTEM_CATALOG.iter()
        .find(|(n, _, _)| *n == name)
        .map(|(n, slug, uninstallable)| (*n, *slug, *uninstallable))
}

fn hackeros_ecosystem_marker_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    std::path::PathBuf::from(format!("{home}/.hackeros/ecosystem"))
}

async fn install_hackeros_tool(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    let (_, slug, _) = hackeros_ecosystem_entry(name).ok_or_else(|| format!("Unknown HackerOS Ecosystem tool: {name}"))?;
    emit_log(app, "info", &format!("Unpacking {} (`hacker unpack {}`)...", name, slug));
    emit_prog(app, "install", &format!("Installing {}...", name), 0.2);
    run_streaming(app, &["hacker", "unpack", slug]).await?;
    let dir = hackeros_ecosystem_marker_dir();
    std::fs::create_dir_all(&dir).ok();
    std::fs::write(dir.join(slug), chrono_now_string()).ok();
    emit_prog(app, "done", "Done!", 1.0);
    Ok(())
}

async fn uninstall_hackeros_tool(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    let (_, slug, uninstallable) = hackeros_ecosystem_entry(name).ok_or_else(|| format!("Unknown HackerOS Ecosystem tool: {name}"))?;
    if !uninstallable {
        return Err(format!("{name} cannot be removed via `hacker pack` — this is a one-way install."));
    }
    emit_log(app, "info", &format!("Packing {} (`hacker pack {}`)...", name, slug));
    emit_prog(app, "uninstall", &format!("Removing {}...", name), 0.3);
    run_streaming(app, &["hacker", "pack", slug]).await?;
    let _ = std::fs::remove_file(hackeros_ecosystem_marker_dir().join(slug));
    emit_prog(app, "done", "Removed.", 1.0);
    Ok(())
}

/// Cheap timestamp for the ecosystem marker files — avoids pulling in the
/// `chrono` / `time` crates just to stamp a file nobody parses back out.
fn chrono_now_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

/// Whether the `hacker` CLI itself is on PATH — surfaced in the Ecosystem
/// view so a missing CLI shows a clear banner instead of every install just
/// silently failing with a "command not found" buried in the log.
#[tauri::command]
async fn is_hacker_available() -> bool {
    Command::new("which").arg("hacker")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status().await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Whether Podman itself is on PATH — surfaced in the Dev Tools view (same
/// idea as `is_hacker_available` for HackerOS Ecosystem) so a person sees
/// a clear heads-up instead of the first Container-mode install just
/// silently taking longer than expected while it installs Podman via apt
/// first (`ensure_podman`, above, does that automatically either way —
/// this is purely informational).
#[tauri::command]
async fn is_podman_available() -> bool {
    Command::new("which").arg("podman")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status().await
        .map(|s| s.success())
        .unwrap_or(false)
}

// ─── update_system ────────────────────────────────────────────────────────────

#[tauri::command]
async fn update_system(app: tauri::AppHandle) -> Result<String, String> {
    reset_job(&app);
    let home   = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let script = format!("{home}/.hackeros/hacker/update-system");
    if !std::path::Path::new(&script).exists() {
        let msg = format!(
            "System update script not found at {script}.\n\
             This feature only works on a HackerOS installation where that script is provided \
             by the base system. On other systems, please update via your normal package manager."
        );
        emit_log(&app, "error", &msg);
        return Err(msg);
    }
    emit_log(&app, "info", "Running system update...");
    emit_prog(&app, "update", "Running system update...", 0.1);
    let result = run_streaming(&app, &[&script]).await;
    reset_job(&app);
    let _ = history::record("update", "system", "HackerOS system update", "system", None,
        result.is_ok(), result.as_ref().err().cloned());
    result?;
    emit_prog(&app, "done", "System updated!", 1.0);
    emit_log(&app, "success", "System updated successfully.");
    Ok("System updated successfully.".into())
}

/// Counts pending APT upgrades using the current (possibly slightly stale)
/// package index — deliberately does *not* run `apt-get update` itself
/// (that needs root and would be surprising to trigger silently just from
/// opening the app). Used to badge the "Update System" nav item and to
/// honour the "check for updates on startup" setting.
#[tauri::command]
async fn check_updates_available() -> u32 {
    pkgbackend::upgradable_count().await
}

// ─── Discover: source search helpers ──────────────────────────────────────────
//
// Every one of these shells out to a real package-manager CLI, and a few of
// them (snap, brew) can hit the network under the hood. Without a bound, one
// slow/hanging source used to stall the *entire* Discover request — this is
// the main reason browsing or searching could feel like it "loads forever".
// Every subprocess call below is now wrapped in a timeout so a single slow
// source degrades to "no results from that source" instead of blocking
// everything else.
async fn run_timeout(mut cmd: Command, secs: u64) -> Option<std::process::Output> {
    tokio::time::timeout(std::time::Duration::from_secs(secs), cmd.output()).await.ok()?.ok()
}

/// Same as `run_timeout`, but keeps *why* it failed instead of collapsing
/// timeout / missing-binary / any other spawn error into the same `None` —
/// used by the Discover source functions so `run_all_sources` can report
/// e.g. "Snap did not respond in time" instead of just quietly returning
/// fewer results with no explanation.
async fn run_timeout_reported(mut cmd: Command, secs: u64, source: &str) -> Result<std::process::Output, SourceIssue> {
    match tokio::time::timeout(std::time::Duration::from_secs(secs), cmd.output()).await {
        Err(_) => Err(SourceIssue {
            source: source.into(), kind: "timeout".into(),
            message: format!("{source} did not respond within {secs}s."),
        }),
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => Err(SourceIssue {
            source: source.into(), kind: "unavailable".into(),
            message: format!("{source} is not installed on this system."),
        }),
        Ok(Err(e)) => Err(SourceIssue {
            source: source.into(), kind: "error".into(),
            message: format!("{source} failed: {e}"),
        }),
        Ok(Ok(out)) => Ok(out),
    }
}

/// Discover "apt" source search. Delegates to [`pkgbackend::search`],
/// which runs `apt-cache search` when real APT is present and falls back
/// to `hammer search`/`hammer oci search` otherwise — see `pkgbackend.rs`.
async fn search_apt(query: String) -> Result<Vec<DiscoverResult>, SourceIssue> {
    pkgbackend::search(query).await
}

async fn search_flatpak(query: String) -> Result<Vec<DiscoverResult>, SourceIssue> {
    let mut cmd = Command::new("flatpak");
    cmd.args(["search","--columns=name,description,application,version",&query]);
    let out = run_timeout_reported(cmd, 5, "flatpak").await?;
    Ok(String::from_utf8_lossy(&out.stdout).lines()
        .filter(|l| !l.starts_with("Name")).take(10).filter_map(|line| {
            let c: Vec<&str> = line.split('\t').collect();
            if c.len() < 3 { return None; }
            let name=c[0].trim().to_string(); if name.is_empty() { return None; }
            let desc=c[1].trim().to_string();
            let id  =c[2].trim().to_string();
            let ver =c.get(3).map(|s|s.trim().to_string()).unwrap_or_default();
            Some(DiscoverResult { name, version:ver, desc, source:"flatpak".into(), package_id:id, size:None, icon:None })
        }).collect())
}

/// Installs a snap, first checking `snap info` for whether it requires
/// classic confinement (VS Code, Android Studio, Slack, and other snaps
/// that need broad filesystem/system access) rather than letting
/// `snap install` fail with a raw "requires classic confinement, aborting"
/// CLI error the person then has to go figure out. Also applies a channel/
/// track (`--channel=`) if one's given, falling back to the configured
/// default (`AppSettings.snap_default_channel`) — "stable" means omit the
/// flag entirely, since that's `snap install`'s own default.
async fn snap_install(app: &tauri::AppHandle, id: &str, channel: Option<&str>) -> Result<(), String> {
    let id = validate_pkg_token(id)?;

    let mut info_cmd = Command::new("snap");
    info_cmd.args(["info", &id]);
    let confinement = run_timeout(info_cmd, 5).await.and_then(|out| {
        String::from_utf8_lossy(&out.stdout).lines()
            .find_map(|l| l.strip_prefix("confinement:").map(|s| s.trim().to_string()))
    });

    let mut args: Vec<String> = vec!["install".into(), id.clone()];
    if confinement.as_deref() == Some("classic") {
        emit_log(app, "info", &format!("{id} requires classic confinement — installing with --classic."));
        args.push("--classic".into());
    } else if confinement.as_deref() == Some("devmode") {
        emit_log(app, "info", &format!("{id} is only published in devmode (development snap)."));
        args.push("--devmode".into());
    }

    let chan = channel.filter(|c| !c.is_empty()).map(|c| c.to_string())
        .unwrap_or_else(|| current_settings().snap_default_channel);
    if !chan.is_empty() && !chan.eq_ignore_ascii_case("stable") {
        let chan = validate_pkg_token(&chan)?;
        emit_log(app, "info", &format!("Installing {id} from channel {chan}..."));
        args.push(format!("--channel={chan}"));
    }

    let argv: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    priv_run(app, &argv).await
}

async fn search_snap(query: String) -> Result<Vec<DiscoverResult>, SourceIssue> {
    let mut cmd = Command::new("snap");
    cmd.args(["find",&query]);
    // `snap find` calls out to the Snap Store over the network — the
    // likeliest source of multi-second (or worse) latency, so it gets the
    // most generous timeout but is still bounded, and reported as a
    // "timeout" issue (not silently fewer results) if it's exceeded.
    let out = run_timeout_reported(cmd, 6, "snap").await?;
    Ok(String::from_utf8_lossy(&out.stdout).lines().skip(1).take(8).filter_map(|line| {
        let c: Vec<&str> = line.split_whitespace().collect();
        if c.is_empty() { return None; }
        let name=c[0].to_string();
        let ver =c.get(1).map(|s|s.to_string()).unwrap_or_default();
        let desc=c.get(3..).map(|s|s.join(" ")).unwrap_or_default();
        Some(DiscoverResult { name:name.clone(), version:ver, desc, source:"snap".into(), package_id:name, size:None, icon:None })
    }).collect())
}

async fn search_brew(query: String) -> Result<Vec<DiscoverResult>, SourceIssue> {
    let mut cmd = Command::new("brew");
    cmd.args(["search",&query]);
    let out = run_timeout_reported(cmd, 6, "brew").await?;
    let mut res = vec![];
    let mut in_sec = false;
    for line in String::from_utf8_lossy(&out.stdout).lines().take(40) {
        if line.starts_with('=') { in_sec=true; continue; }
        if !in_sec { continue; }
        let name=line.trim().to_string(); if name.is_empty() { continue; }
        res.push(DiscoverResult { name:name.clone(), version:String::new(), desc:String::new(),
            source:"brew".into(), package_id:name, size:None, icon:None });
        if res.len()>=8 { break; }
    }
    Ok(res)
}

async fn run_all_sources(app: &tauri::AppHandle, query: String, settings: &AppSettings) -> (Vec<DiscoverResult>, Vec<SourceIssue>) {
    let want = |s: &str| settings.enabled_sources.iter().any(|x| x == s);
    let qlc = query.to_lowercase();

    // See `SourceCacheState`'s doc comment: each source's own raw result is
    // cached independently, keyed by "{source}:{query}", so one slow/
    // erroring/newly-toggled source doesn't force every other source to be
    // re-queried too.
    async fn cached<Fut>(app: &tauri::AppHandle, source: &str, qlc: &str, fetch: Fut) -> Result<Vec<DiscoverResult>, SourceIssue>
    where
        Fut: std::future::Future<Output = Result<Vec<DiscoverResult>, SourceIssue>>,
    {
        let key = format!("{source}:{qlc}");
        if let Some(cached) = source_cache_get(app, &key).await { return cached; }
        let result = fetch.await;
        source_cache_set(app, key, result.clone()).await;
        result
    }

    let (apt, fp, snap, brew, hpm_res, nix_res, appimage_res) = tokio::join!(
        async { if want("apt")      { Some(cached(app, "apt", &qlc, search_apt(query.clone())).await) }      else { None } },
        async { if want("flatpak")  { Some(cached(app, "flatpak", &qlc, search_flatpak(query.clone())).await) }  else { None } },
        async { if want("snap")     { Some(cached(app, "snap", &qlc, search_snap(query.clone())).await) }     else { None } },
        async { if want("brew")     { Some(cached(app, "brew", &qlc, search_brew(query.clone())).await) }     else { None } },
        async { if want("hpm")      { Some(cached(app, "hpm", &qlc, hpm::search(query.clone())).await) }     else { None } },
        async { if want("nix")      { Some(cached(app, "nix", &qlc, hnm::search(query.clone())).await) }     else { None } },
        async { if want("appimage") { Some(cached(app, "appimage", &qlc, appimage::search(query.clone())).await) } else { None } },
    );

    let mut results = vec![];
    let mut issues = vec![];
    for outcome in [apt, fp, snap, brew, hpm_res, nix_res, appimage_res] {
        match outcome {
            None => {} // source not enabled — not an issue, just not queried
            Some(Ok(r)) => results.extend(r),
            Some(Err(issue)) => issues.push(issue),
        }
    }
    (results, issues)
}

// ─── Local icon cache lookups (no network needed for these) ──────────────────
//
// The previous implementation looked up each app's icon one at a time,
// *sequentially*, and — worse — shelled out to the `base64` binary as a
// separate subprocess for every single icon found, plus a fresh directory
// scan of the apt AppStream icon cache per app. For a 20-30 item result
// list that was easily 20-30 sequential process spawns just for icons,
// which is a large share of why Discover could feel like it never
// finished loading. Now: the icon-cache directories are scanned exactly
// once per batch (building an in-memory index), and the actual file reads
// + base64 encoding happen in Rust (no subprocess) and fully in parallel
// across all items via a JoinSet.

async fn b64_file(path: &std::path::Path) -> Option<String> {
    let bytes = tokio::fs::read(path).await.ok()?;
    if bytes.is_empty() { return None; }
    Some(format!("data:image/png;base64,{}", B64.encode(&bytes)))
}

type IconIndex = std::collections::HashMap<String, std::path::PathBuf>;

async fn index_dir_pngs(dir: &std::path::Path, idx: &mut IconIndex) {
    if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("png") { continue; }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                idx.entry(stem.to_string()).or_insert(path);
            }
        }
    }
}

/// Scans Flatpak's own local AppStream icon cache once, building an
/// id -> path index. Populated automatically once a remote's metadata has
/// been fetched (which `ensure_flatpak`/`flatpak search` trigger) — no
/// network access needed here. Apps not yet cached simply aren't in the
/// index, and the frontend falls back to a source-badge icon for those.
async fn build_flatpak_icon_index() -> IconIndex {
    let mut idx = IconIndex::new();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    for base in [
        format!("{home}/.local/share/flatpak/appstream/flathub"),
        "/var/lib/flatpak/appstream/flathub".to_string(),
    ] {
        for arch in ["x86_64", "aarch64"] {
            for size in ["128x128", "64x64"] {
                let dir = std::path::PathBuf::from(format!("{base}/{arch}/active/icons/{size}"));
                index_dir_pngs(&dir, &mut idx).await;
            }
        }
    }
    idx
}

/// Same idea for apt/Debian packages: scans the local AppStream icon cache
/// maintained by the `appstream` package's APT hooks
/// (`/var/cache/app-info/icons/<origin>/<size>/<component>.png`) exactly
/// once per batch instead of once per app.
async fn build_apt_icon_index() -> IconIndex {
    let mut idx = IconIndex::new();
    let base = std::path::Path::new("/var/cache/app-info/icons");
    if let Ok(mut origins) = tokio::fs::read_dir(base).await {
        while let Ok(Some(origin)) = origins.next_entry().await {
            for size in ["128x128", "64x64"] {
                index_dir_pngs(&origin.path().join(size), &mut idx).await;
            }
        }
    }
    idx
}

/// Single-icon lookups for the app-detail view (opened once at a time, so
/// building a whole index just for one lookup is fine perf-wise — the
/// batch path used by browse/search lists uses build_*_icon_index directly
/// instead, to avoid rescanning per item).
async fn single_flatpak_icon(id: &str) -> Option<String> {
    if id.is_empty() { return None; }
    let idx = build_flatpak_icon_index().await;
    match idx.get(id) { Some(p) => b64_file(p).await, None => None }
}

async fn single_apt_icon(pkg: &str) -> Option<String> {
    if pkg.is_empty() { return None; }
    let idx = build_apt_icon_index().await;
    match idx.get(pkg) { Some(p) => b64_file(p).await, None => None }
}

async fn dedupe_and_enrich(items: Vec<DiscoverResult>) -> Vec<DiscoverResult> {
    let mut seen = std::collections::HashSet::new();
    let mut deduped: Vec<DiscoverResult> = vec![];
    for r in items {
        let key = format!("{}::{}", r.source, r.name.to_lowercase());
        if seen.insert(key) {
            deduped.push(r);
            if deduped.len() >= 24 { break; }
        }
    }

    let needs_flatpak = deduped.iter().any(|r| r.source == "flatpak");
    let needs_apt     = deduped.iter().any(|r| r.source == "apt");
    let (fp_idx, apt_idx): (IconIndex, IconIndex) = tokio::join!(
        async { if needs_flatpak { build_flatpak_icon_index().await } else { IconIndex::new() } },
        async { if needs_apt     { build_apt_icon_index().await }     else { IconIndex::new() } },
    );

    let mut set: JoinSet<(usize, Option<String>)> = JoinSet::new();
    for (i, r) in deduped.iter().enumerate() {
        let source = r.source.clone();
        let package_id = r.package_id.clone();
        let local_path = match source.as_str() {
            "flatpak" => fp_idx.get(&r.package_id).cloned(),
            "apt"     => apt_idx.get(&r.package_id).cloned(),
            _ => None,
        };
        set.spawn(async move {
            let icon = match source.as_str() {
                // Local AppStream icon cache — no network needed.
                "flatpak" | "apt" => match local_path { Some(p) => b64_file(&p).await, None => None },
                // No local cache exists for snaps (see `snapcraft_icon`'s
                // doc comment) — this is the one enrichment path in this
                // function that hits the network, bounded by
                // `snapcraft_icon`'s own per-request timeouts and run
                // concurrently with everything else here via the JoinSet,
                // same "best effort, never blocks the whole batch on one
                // slow lookup" shape as the rest of Discover.
                "snap" => snapcraft_icon(&package_id).await,
                _ => None,
            };
            (i, icon)
        });
    }
    let mut icons: std::collections::HashMap<usize, Option<String>> = std::collections::HashMap::new();
    while let Some(res) = set.join_next().await {
        if let Ok((i, icon)) = res { icons.insert(i, icon); }
    }
    for (i, r) in deduped.iter_mut().enumerate() {
        r.icon = icons.remove(&i).flatten();
    }
    deduped
}

// ─── Discover categories ──────────────────────────────────────────────────────
//
// Real app stores (GNOME Software, Plasma Discover) browse by category using
// each app's AppStream <categories> metadata. Fully replicating that here
// would mean parsing AppStream XML/YAML for the entire catalog just to
// build a browse index, which is a lot of machinery for an offline-first
// tool. Instead, each category maps to a small set of representative search
// terms and reuses the exact same live multi-source search used by the
// search box — so browsing a category is still 100% live data from
// apt/flatpak/snap/brew, never a hardcoded app list, just seeded with
// broader terms than a person would necessarily type themselves.
const CATEGORIES: &[(&str, &str, &str, &[&str])] = &[
    ("development", "Development",            "Code",      &["ide", "compiler git", "programming editor"]),
    ("office",      "Office & Productivity",   "FileText",  &["office suite", "pdf reader", "notes app"]),
    ("graphics",    "Graphics & Photography",  "Palette",   &["image editor", "photo editor", "vector graphics"]),
    ("media",       "Audio & Video",           "Music",     &["video editor", "audio editor", "media player"]),
    ("internet",    "Internet & Communication","Globe",     &["web browser", "email client", "chat client"]),
    ("security",    "Security & Privacy",      "Shield",    &["password manager", "vpn client", "encryption"]),
    ("system",      "System Tools",            "Cpu",       &["backup tool", "disk utility", "virtualization"]),
    ("games",       "Games",                   "Gamepad2",  &["game"]),
    ("utilities",   "Utilities",               "Wrench",    &["file manager", "archive manager"]),
];

#[tauri::command]
fn discover_categories() -> Vec<CategoryDef> {
    CATEGORIES.iter().map(|(id, label, icon, _)| CategoryDef {
        id: id.to_string(), label: label.to_string(), icon: icon.to_string(),
    }).collect()
}

#[tauri::command]
async fn discover_browse(app: tauri::AppHandle, category_id: String) -> DiscoverResponse {
    let settings = current_settings();
    let cache_key = format!("browse:{category_id}:{}", settings.enabled_sources.join(","));
    if let Some(cached) = cache_get(&app, &cache_key).await { return cached; }

    let kws: Vec<String> = CATEGORIES.iter().find(|(id, _, _, _)| *id == category_id)
        .map(|(_, _, _, k)| k.iter().take(2).map(|s| s.to_string()).collect())
        .unwrap_or_default();
    // The two seed keywords used to be queried one after another (up to 2x
    // the wall-clock time of a single round of 4-source searches). They're
    // independent, so run them concurrently instead — total latency is now
    // bounded by whichever single keyword/source combination is slowest,
    // not by the sum of all of them.
    let mut kw_iter = kws.into_iter();
    let kw0 = kw_iter.next();
    let kw1 = kw_iter.next();
    let (batch0, batch1) = tokio::join!(
        async { match kw0 { Some(k) => run_all_sources(&app, k, &settings).await, None => (vec![], vec![]) } },
        async { match kw1 { Some(k) => run_all_sources(&app, k, &settings).await, None => (vec![], vec![]) } },
    );
    let mut all = batch0.0;
    all.extend(batch1.0);
    // Same issue can legitimately come from both keyword batches (e.g. snap
    // timing out on both) — de-dup by source so the frontend doesn't show
    // "Snap did not respond" twice.
    let mut issues = batch0.1;
    for issue in batch1.1 {
        if !issues.iter().any(|i: &SourceIssue| i.source == issue.source) { issues.push(issue); }
    }
    let results = dedupe_and_enrich(all).await;
    let response = DiscoverResponse { results, issues };
    cache_set(&app, cache_key, response.clone()).await;
    response
}

#[tauri::command]
async fn discover_search(app: tauri::AppHandle, query: String) -> DiscoverResponse {
    let settings = current_settings();
    let cache_key = format!("search:{}:{}", query.to_lowercase(), settings.enabled_sources.join(","));
    if let Some(cached) = cache_get(&app, &cache_key).await { return cached; }
    let (all, issues) = run_all_sources(&app, query, &settings).await;
    let results = dedupe_and_enrich(all).await;
    let response = DiscoverResponse { results, issues };
    cache_set(&app, cache_key, response.clone()).await;
    response
}

#[tauri::command]
async fn discover_install(
    app: tauri::AppHandle,
    package_id: String,
    source: String,
    name: Option<String>,
    // Flatpak only: which configured remote (by name) to install from.
    // Falls back to `settings.flatpak_default_remote`.
    remote: Option<String>,
    // Flatpak only: branch to install, e.g. "beta". Falls back to
    // `settings.flatpak_default_branch`.
    branch: Option<String>,
    // Snap only: channel/track, e.g. "latest/edge". Falls back to
    // `settings.snap_default_channel`.
    channel: Option<String>,
) -> Result<String, String> {
    let package_id = validate_pkg_token(&package_id)?;
    // AppImage's package_id is "owner/repo" (there's no separate slug to
    // install by) — the display name is only used for the wrapper
    // filename / .desktop entry, so falling back to the repo part of the
    // id keeps install working even if the frontend didn't pass one.
    let display_name = name.unwrap_or_else(|| {
        package_id.rsplit('/').next().unwrap_or(&package_id).to_string()
    });
    reset_job(&app);
    emit_log(&app, "info", &format!("Installing {} via {}...", package_id, source));
    emit_prog(&app, "install", &format!("Installing {}...", package_id), 0.2);
    let result: Result<(), String> = async {
        match source.as_str() {
            "apt"     => apt_install(&app, &[package_id.as_str()]).await,
            "flatpak" => {
                ensure_flatpak(&app).await?;
                let settings = current_settings();
                let remote_name = remote.filter(|r| !r.is_empty()).unwrap_or(settings.flatpak_default_remote);
                let remote_name = validate_pkg_token(&remote_name)?;
                let branch_name = branch.filter(|b| !b.is_empty()).unwrap_or(settings.flatpak_default_branch);
                let ref_arg = flatpak_ref(&package_id, &branch_name);
                if run_streaming(&app, &["flatpak", "install", "-y", "--user", &remote_name, &ref_arg]).await.is_err() {
                    priv_run(&app, &["flatpak", "install", "-y", &remote_name, &ref_arg]).await?;
                }
                Ok(())
            },
            "snap" => snap_install(&app, &package_id, channel.as_deref()).await,
            "brew" => run_streaming(&app, &["brew", "install", &package_id]).await,
            "hpm"  => hpm::install(&app, &package_id).await,
            "nix"  => hnm::install(&app, &package_id).await,
            "appimage" => appimage::install(&app, &package_id, &display_name).await,
            _ => Err(format!("Unknown source: {source}")),
        }
    }.await;
    reset_job(&app);
    let (version, commit, nix_generation) = if result.is_err() {
        (None, None, None)
    } else {
        match source.as_str() {
            "apt" => (history::current_apt_version(&package_id).await, None, None),
            "appimage" => (appimage::is_installed(&package_id).map(|i| i.version), None, None),
            "flatpak" => (None, flatpak_current_commit(&package_id).await, None),
            "nix" => (None, None, hnm::current_generation().await),
            _ => (None, None, None),
        }
    };
    let _ = if commit.is_some() {
        history::record_with_commit("install", &source, &display_name, &package_id, version, commit,
            result.is_ok(), result.as_ref().err().cloned())
    } else if nix_generation.is_some() {
        history::record_with_generation("install", &source, &display_name, &package_id, version, nix_generation,
            result.is_ok(), result.as_ref().err().cloned())
    } else {
        history::record("install", &source, &display_name, &package_id, version,
            result.is_ok(), result.as_ref().err().cloned())
    };
    result?;
    emit_prog(&app, "done", "Done!", 1.0);
    emit_log(&app, "success", &format!("{package_id} installed."));
    Ok(format!("{package_id} installed."))
}

#[tauri::command]
async fn discover_uninstall(app: tauri::AppHandle, package_id: String, source: String) -> Result<String, String> {
    let package_id = validate_pkg_token(&package_id)?;
    reset_job(&app);
    emit_log(&app, "info", &format!("Removing {} ({})...", package_id, source));
    emit_prog(&app, "uninstall", &format!("Removing {}...", package_id), 0.2);
    let result: Result<(), String> = async {
        match source.as_str() {
            "apt"     => apt_remove(&app, &[package_id.as_str()]).await,
            "flatpak" => {
                if run_streaming(&app, &["flatpak", "uninstall", "-y", "--user", &package_id]).await.is_err() {
                    priv_run(&app, &["flatpak", "uninstall", "-y", &package_id]).await?;
                }
                Ok(())
            },
            "snap" => priv_run(&app, &["snap","remove",&package_id]).await,
            "brew" => run_streaming(&app, &["brew", "uninstall", &package_id]).await,
            "hpm"  => hpm::remove(&app, &package_id).await,
            "nix"  => hnm::remove(&app, &package_id).await,
            "appimage" => appimage::uninstall(&app, &package_id).await,
            _ => Err(format!("Unknown source: {source}")),
        }
    }.await;
    reset_job(&app);
    let nix_generation = if result.is_ok() && source == "nix" { hnm::current_generation().await } else { None };
    let _ = if nix_generation.is_some() {
        history::record_with_generation("uninstall", &source, &package_id, &package_id, None, nix_generation,
            result.is_ok(), result.as_ref().err().cloned())
    } else {
        history::record("uninstall", &source, &package_id, &package_id, None,
            result.is_ok(), result.as_ref().err().cloned())
    };
    result?;
    emit_prog(&app, "done", "Removed.", 1.0);
    emit_log(&app, "success", &format!("{package_id} removed."));
    Ok(format!("{package_id} removed."))
}

#[tauri::command]
async fn get_installed_sets() -> InstalledSets {
    // Previously 4 sequential subprocess calls; now run concurrently and
    // each bounded by a timeout, since this fires on every Discover mount
    // and after every install/uninstall action.
    let mut sets = InstalledSets::default();
    let mut fp_cmd  = Command::new("flatpak");    fp_cmd.args(["list", "--columns=application"]);
    let mut snap_cmd = Command::new("snap");       snap_cmd.args(["list"]);
    let mut brew_cmd = Command::new("brew");       brew_cmd.args(["list", "--formula"]);
    // `pkgbackend::installed_names` covers both real apt (dpkg-query) and,
    // when that's absent, hammer (`hammer list --installed` /
    // `hammer oci list`) — see pkgbackend.rs.
    let (apt_names, fp_out, snap_out, brew_out, hpm_names, nix_names) = tokio::join!(
        pkgbackend::installed_names(),
        run_timeout(fp_cmd, 4), run_timeout(snap_cmd, 4), run_timeout(brew_cmd, 4),
        hpm::installed_names(), hnm::installed_names(),
    );
    sets.hpm = hpm_names;
    sets.nix = nix_names;
    sets.appimage = appimage::list_installed().into_iter().map(|e| e.repo).collect();
    sets.apt = apt_names;
    if let Some(out) = fp_out {
        sets.flatpak = String::from_utf8_lossy(&out.stdout).lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    }
    if let Some(out) = snap_out {
        sets.snap = String::from_utf8_lossy(&out.stdout).lines().skip(1)
            .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string())).collect();
    }
    if let Some(out) = brew_out {
        sets.brew = String::from_utf8_lossy(&out.stdout).lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    }
    sets
}

// ─── AppStream-based rich details (best effort) ───────────────────────────────

async fn ensure_appstream(app: &tauri::AppHandle) -> Result<(), String> {
    let has = Command::new("which").arg("appstreamcli").output().await
        .map(|o| o.status.success()).unwrap_or(false);
    if !has {
        emit_log(app, "info", "Installing AppStream metadata tools...");
        apt_install(app, &["appstream"]).await?;
    }
    Ok(())
}

async fn appstream_get(id: &str) -> Option<serde_json::Value> {
    let mut cmd = Command::new("appstreamcli");
    cmd.args(["get", id, "--format=json"]);
    let out = run_timeout(cmd, 5).await?;
    if !out.status.success() || out.stdout.is_empty() { return None; }
    serde_json::from_slice(&out.stdout).ok()
}

// The exact JSON schema `appstreamcli --format=json` emits varies by
// version and wasn't verifiable from this offline sandbox, so the helpers
// below deliberately don't assume fixed key names. Instead they walk the
// JSON tree looking for keys that *contain* a recognisable substring
// (e.g. any key containing "screenshot"), which is tolerant of schema
// differences across appstreamcli/libappstream versions at the cost of
// being a bit more permissive than a strict typed parse. If ratings or
// screenshots stop showing up after an appstreamcli upgrade, this is the
// place to adjust.

fn locale_val_to_string(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(map) => map.get("C").and_then(|x| x.as_str()).map(|s| s.to_string())
            .or_else(|| map.values().next().and_then(|x| x.as_str()).map(|s| s.to_string())),
        _ => None,
    }
}

fn find_locale_string(v: &serde_json::Value, key_substr: &str) -> Option<String> {
    if let serde_json::Value::Object(map) = v {
        for (k, val) in map {
            if k.to_lowercase().contains(key_substr) {
                if let Some(s) = locale_val_to_string(val) { return Some(s); }
            }
        }
        for (_, val) in map {
            if let Some(found) = find_locale_string(val, key_substr) { return Some(found); }
        }
    }
    None
}

fn find_first_string(v: &serde_json::Value, key_substr: &str) -> Option<String> {
    if let serde_json::Value::Object(map) = v {
        for (k, val) in map {
            if k.to_lowercase().contains(key_substr) {
                if let Some(s) = val.as_str() { return Some(s.to_string()); }
            }
        }
        for (_, val) in map {
            if let Some(f) = find_first_string(val, key_substr) { return Some(f); }
        }
    }
    None
}

fn collect_plain_strings(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(arr) => for item in arr { collect_plain_strings(item, out); },
        _ => {}
    }
}

fn collect_all_strings_under(v: &serde_json::Value, key_substr: &str, out: &mut Vec<String>) {
    if let serde_json::Value::Object(map) = v {
        for (k, val) in map {
            if k.to_lowercase().contains(key_substr) {
                collect_plain_strings(val, out);
            } else {
                collect_all_strings_under(val, key_substr, out);
            }
        }
    }
}

fn collect_all_url_strings(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) if s.starts_with("http") => out.push(s.clone()),
        serde_json::Value::Object(map) => { for (_, val) in map { collect_all_url_strings(val, out); } }
        serde_json::Value::Array(arr) => { for item in arr { collect_all_url_strings(item, out); } }
        _ => {}
    }
}

fn collect_urls_under_key(v: &serde_json::Value, key_substr: &str, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(map) => {
            for (k, val) in map {
                if k.to_lowercase().contains(key_substr) {
                    collect_all_url_strings(val, out);
                } else {
                    collect_urls_under_key(val, key_substr, out);
                }
            }
        }
        serde_json::Value::Array(arr) => { for item in arr { collect_urls_under_key(item, key_substr, out); } }
        _ => {}
    }
}

/// AppStream long descriptions are simple HTML-ish markup (`<p>`, `<ul>`,
/// `<li>`). This strips tags into readable plain-text paragraphs since the
/// frontend renders descriptions as plain text, not HTML.
fn strip_simple_markup(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => { in_tag = false; out.push('\n'); }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n\n")
}

fn enrich_from_appstream(d: &mut AppDetails, json: &serde_json::Value) {
    let root = if json.is_array() {
        json.get(0).cloned().unwrap_or(serde_json::Value::Null)
    } else { json.clone() };

    if let Some(s) = find_locale_string(&root, "summary") { d.summary = s; }
    if let Some(s) = find_locale_string(&root, "description") { d.description = strip_simple_markup(&s); }
    if let Some(s) = find_first_string(&root, "homepage") { d.homepage = Some(s); }
    if let Some(s) = find_first_string(&root, "project_license").or_else(|| find_first_string(&root, "license")) {
        d.license = Some(s);
    }
    let mut cats = vec![];
    collect_all_strings_under(&root, "categor", &mut cats);
    cats.sort(); cats.dedup();
    if !cats.is_empty() { d.categories = cats; }

    let mut shots = vec![];
    collect_urls_under_key(&root, "screenshot", &mut shots);
    shots.retain(|u| u.ends_with(".png") || u.ends_with(".jpg") || u.ends_with(".jpeg") || u.contains("screenshot"));
    shots.sort(); shots.dedup();
    if !shots.is_empty() { d.screenshots = shots.into_iter().take(6).collect(); }
}

fn parse_snap_info(d: &mut AppDetails, text: &str) {
    let mut in_desc = false;
    let mut desc_lines = vec![];
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("summary:") { d.summary = rest.trim().to_string(); continue; }
        if let Some(rest) = line.strip_prefix("license:") { d.license = Some(rest.trim().to_string()); continue; }
        if let Some(rest) = line.strip_prefix("publisher:") { d.categories.push(format!("Publisher: {}", rest.trim())); continue; }
        if let Some(rest) = line.strip_prefix("confinement:") { d.confinement = Some(rest.trim().to_string()); continue; }
        if line.starts_with("description:") { in_desc = true; continue; }
        if in_desc {
            if line.starts_with(' ') || line.starts_with('|') {
                desc_lines.push(line.trim_start_matches('|').trim().to_string());
            } else { in_desc = false; }
        }
        if let Some(rest) = line.trim_start().strip_prefix("stable:") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if let Some(v) = parts.first() { d.version = Some(v.to_string()); }
            if let Some(sz) = parts.iter().find(|p| p.ends_with("MB") || p.ends_with("kB") || p.ends_with("GB")) {
                d.size = Some(sz.to_string());
            }
        }
    }
    if !desc_lines.is_empty() { d.description = desc_lines.join(" ").trim().to_string(); }
}

fn parse_brew_info(d: &mut AppDetails, json: &serde_json::Value) {
    let item = json.get("formulae").and_then(|a| a.get(0))
        .or_else(|| json.get("casks").and_then(|a| a.get(0)));
    if let Some(item) = item {
        if let Some(s) = item.get("desc").and_then(|v| v.as_str()) { d.summary = s.to_string(); d.description = s.to_string(); }
        if let Some(s) = item.get("homepage").and_then(|v| v.as_str()) { d.homepage = Some(s.to_string()); }
        if let Some(s) = item.get("license").and_then(|v| v.as_str()) { d.license = Some(s.to_string()); }
        if let Some(s) = item.get("versions").and_then(|v| v.get("stable")).and_then(|v| v.as_str()) {
            d.version = Some(s.to_string());
        }
    }
}

/// Delegates to [`pkgbackend::show_info`] — `apt-cache show` when real
/// APT is present, `hammer info --json` / `hammer oci search` otherwise.
async fn apt_show_info(name: &str) -> serde_json::Value {
    pkgbackend::show_info(name).await
}

/// Fetches community star ratings from the GNOME ODRS service — the same
/// service GNOME Software queries. Best-effort: wrapped in a short timeout
/// and returns `None` on any error (offline machine, service down, schema
/// mismatch), never fails the whole detail view. Gated behind
/// `settings.ratings_enabled` so it's opt-out for anyone who doesn't want
/// the app making outbound requests. Coverage is naturally limited to apps
/// that have an AppStream id ODRS recognises — in practice, Flatpak apps.
/// Snap has no local AppStream-style icon cache the way apt/flatpak do —
/// `snap find`/`snap info`'s plain-text output never carries an icon URL
/// at all (that's why every snap result used to fall back to the generic
/// source-badge icon even when apt/flatpak had a real one for the same
/// app). The actual icon lives in the Snap Store's own JSON API instead.
///
/// The exact shape of `api.snapcraft.io/v2/snaps/info/<name>`'s response
/// wasn't independently verifiable from this offline sandbox, so the media
/// lookup below walks the JSON tolerantly (find a `media` array entry with
/// `type == "icon"`) rather than assuming an exact nested path — same
/// defensive approach as the `appstreamcli --format=json` parsing earlier
/// in this file. Best-effort throughout: any failure just means "no icon
/// for this snap", never a hard error.
async fn snapcraft_icon(name: &str) -> Option<String> {
    if name.is_empty() { return None; }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build().ok()?;
    let url = format!("https://api.snapcraft.io/v2/snaps/info/{name}");
    let resp = client.get(&url).header("Snap-Device-Series", "16").send().await.ok()?;
    if !resp.status().is_success() { return None; }
    let json: serde_json::Value = resp.json().await.ok()?;

    let icon_url = json.get("snap")
        .and_then(|s| s.get("media"))
        .and_then(|m| m.as_array())
        .and_then(|arr| arr.iter().find(|m| m.get("type").and_then(|t| t.as_str()) == Some("icon")))
        .and_then(|m| m.get("url"))
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())?;

    let icon_resp = client.get(&icon_url).send().await.ok()?;
    if !icon_resp.status().is_success() { return None; }
    let bytes = icon_resp.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > 1_500_000 { return None; }
    let mime = if icon_url.ends_with(".svg") { "image/svg+xml" } else { "image/png" };
    Some(format!("data:{mime};base64,{}", B64.encode(&bytes)))
}

async fn fetch_rating(app_id: &str) -> Option<RatingInfo> {
    if app_id.is_empty() { return None; }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build().ok()?;
    let url = format!("https://odrs.gnome.org/api/v2/ratings/{app_id}");
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() { return None; }
    let json: serde_json::Value = resp.json().await.ok()?;
    let mut weighted = 0.0f32;
    let mut count = 0u32;
    for stars in 1..=5 {
        let key = format!("star{stars}");
        if let Some(n) = json.get(&key).and_then(|v| v.as_u64()) {
            weighted += stars as f32 * n as f32;
            count += n as u32;
        }
    }
    if count == 0 { return None; }
    Some(RatingInfo { average: weighted / count as f32, count })
}

#[tauri::command]
async fn get_app_details(app: tauri::AppHandle, package_id: String, source: String, name: Option<String>) -> AppDetails {
    let display_name = name.unwrap_or_else(|| package_id.clone());
    let mut d = AppDetails {
        id: package_id.clone(), name: display_name, source: source.clone(),
        package_id: package_id.clone(), summary: String::new(), description: String::new(),
        icon: None, screenshots: vec![], version: None, license: None, homepage: None,
        categories: vec![], size: None, rating: None, local_rating: None, confinement: None,
    };

    if let Err(e) = validate_pkg_token(&package_id) {
        d.summary = e.clone();
        d.description = e;
        return d;
    }

    match source.as_str() {
        "flatpak" => {
            let info = flatpak_remote_info(&package_id).await;
            d.version = info["version"].as_str().map(|s| s.to_string());
            d.size    = info["size"].as_str().map(|s| s.to_string());
            d.icon    = single_flatpak_icon(&package_id).await;
            if ensure_appstream(&app).await.is_ok() {
                if let Some(json) = appstream_get(&package_id).await {
                    enrich_from_appstream(&mut d, &json);
                }
            }
        }
        "apt" => {
            let info = apt_show_info(&package_id).await;
            d.version = info["version"].as_str().map(|s| s.to_string());
            d.size    = info["size"].as_str().map(|s| s.to_string());
            d.icon    = single_apt_icon(&package_id).await;
            if ensure_appstream(&app).await.is_ok() {
                for cand in [package_id.clone(), format!("{package_id}.desktop")] {
                    if let Some(json) = appstream_get(&cand).await {
                        enrich_from_appstream(&mut d, &json);
                        break;
                    }
                }
            }
            if d.summary.is_empty() {
                // `info` already carries "description" (from apt-cache's
                // Description[-en]: field, or hammer's own package
                // metadata when apt-get isn't present — see
                // `pkgbackend::show_info`), so reuse it instead of a
                // second apt-cache/hammer call for the same package.
                if let Some(desc) = info["description"].as_str() {
                    d.summary = desc.trim().to_string();
                    d.description = desc.trim().to_string();
                }
            }
        }
        "snap" => {
            let mut cmd = Command::new("snap");
            cmd.args(["info", &package_id]);
            if let Some(out) = run_timeout(cmd, 6).await {
                parse_snap_info(&mut d, &String::from_utf8_lossy(&out.stdout));
            }
            d.icon = snapcraft_icon(&package_id).await;
        }
        "brew" => {
            let mut cmd = Command::new("brew");
            cmd.args(["info","--json=v2",&package_id]);
            if let Some(out) = run_timeout(cmd, 6).await {
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                    parse_brew_info(&mut d, &json);
                }
            }
        }
        "hpm" => {
            if let Some(info) = hpm::info(&package_id).await {
                d.version = info.version;
                d.summary = info.summary;
                d.description = info.description;
                d.license = info.license;
                d.categories = info.tags;
                // hpm doesn't ship icons of its own; the frontend falls
                // back to the source badge for this source, same as it
                // already does for any apt/flatpak app with no AppStream
                // icon indexed.
            }
        }
        "nix" => {
            if let Some(info) = hnm::info(&package_id).await {
                d.version = info.version;
                d.summary = info.summary;
                d.description = info.description;
                d.homepage = info.homepage;
                d.license = info.license;
                // Like hpm, nixpkgs attributes have no per-app icon of
                // their own here — frontend falls back to the source
                // badge, same as any apt/flatpak app with no AppStream
                // icon indexed.
            }
        }
        "appimage" => {
            if let Some(entry) = appimage::feed_entry(&package_id).await {
                d.summary = entry.description.chars().take(140).collect();
                d.description = entry.description;
                d.homepage = entry.homepage;
                d.categories = entry.categories;
            }
            d.homepage = d.homepage.or_else(|| Some(format!("https://github.com/{package_id}")));
            if let Some(installed) = appimage::is_installed(&package_id) {
                d.version = Some(installed.version);
                d.icon = installed.icon_path.as_deref().and_then(|p| {
                    std::fs::read(p).ok().map(|b| format!("data:image/png;base64,{}", B64.encode(&b)))
                });
            }
        }
        _ => {}
    }

    if d.summary.is_empty() && !d.description.is_empty() {
        d.summary = d.description.chars().take(140).collect();
    }
    if d.description.is_empty() && !d.summary.is_empty() {
        d.description = d.summary.clone();
    }

    if current_settings().ratings_enabled && source == "flatpak" {
        d.rating = fetch_rating(&package_id).await;
    }
    // Local rating covers every source (apt/flatpak/snap/brew), unlike the
    // ODRS-backed `rating` field above which only exists for Flatpak.
    d.local_rating = ratings::get_local_rating(&source, &package_id);
    d
}

#[tauri::command]
fn submit_rating(source: String, package_id: String, stars: u8, comment: Option<String>) -> Result<RatingInfo, String> {
    ratings::submit_rating(&source, &package_id, stars, comment)
}

#[tauri::command]
fn get_local_rating(source: String, package_id: String) -> Option<RatingInfo> {
    ratings::get_local_rating(&source, &package_id)
}

#[tauri::command]
fn get_reviews(source: String, package_id: String) -> Vec<ratings::LocalReview> {
    ratings::get_reviews(&source, &package_id)
}

#[tauri::command]
fn get_install_history() -> Vec<history::HistoryEntry> {
    history::get_all()
}

#[tauri::command]
fn clear_install_history() -> Result<(), String> {
    history::clear()
}

#[tauri::command]
async fn rollback_history_entry(app: tauri::AppHandle, entry_id: String) -> Result<String, String> {
    history::rollback_entry(&app, &entry_id).await
}

/// Lets the Settings UI show whether Homebrew/Linuxbrew was actually
/// detected on this machine, rather than presenting it as an always-on
/// source that silently errors on every search/install for anyone who
/// doesn't have it installed (see README: "Homebrew support" for the
/// full set of caveats around this source on Linux).
#[tauri::command]
async fn is_brew_available() -> bool {
    Command::new("which").arg("brew").output().await
        .map(|o| o.status.success()).unwrap_or(false)
}

/// Same status check as `is_brew_available`, generalized to every source —
/// this is what used to only exist for brew (Settings could show "brew not
/// detected" but flatpak/snap just silently returned zero Discover results
/// with no explanation if `flatpak`/`snapd` weren't installed). One command
/// covering all of them means Settings can show the same "not detected"
/// treatment consistently instead of brew being a special case.
#[tauri::command]
async fn is_flatpak_available() -> bool {
    Command::new("which").arg("flatpak").output().await
        .map(|o| o.status.success()).unwrap_or(false)
}

#[tauri::command]
async fn is_snap_available() -> bool {
    // `snap` the CLI can be present without `snapd` actually running (a
    // partial/broken install) — checking the CLI binary is still the
    // right signal here though: `snap find`/`install` fail with a clear
    // "no snapd" error from the CLI itself in that case, whereas a
    // missing CLI binary is what previously looked identical to "found
    // nothing" in Discover.
    Command::new("which").arg("snap").output().await
        .map(|o| o.status.success()).unwrap_or(false)
}

/// Same idea as `is_brew_available`, for the HackerOS Community Repository
/// source: lets Settings show "hpm not detected" instead of Discover
/// silently returning zero hpm results with no explanation.
#[tauri::command]
async fn is_hpm_available() -> bool {
    hpm::is_available().await
}

/// Same idea as `is_hpm_available`, for the Nix/nixpkgs source (via
/// `hnm`): lets Settings show "hnm not detected" instead of Discover
/// silently returning zero nix results with no explanation.
#[tauri::command]
async fn is_nix_available() -> bool {
    hnm::is_available().await
}

/// Same idea as `is_hpm_available`/`is_nix_available`, for the "apt"
/// Discover source — `true` if either real apt-get or its hammer
/// fallback is actually usable (see `pkgbackend.rs`), so Settings can
/// show "not detected" in the rare case neither is present (e.g. a
/// stripped-down dev container) instead of just assuming apt exists
/// because "this is a Debian-based OS".
#[tauri::command]
async fn is_apt_available() -> bool {
    pkgbackend::is_available().await
}

// ─── Nix panel (see hnm.rs's module doc comment + NixView.tsx) ──────────────
//
// Read-only/informational commands return their result directly (no job
// state, no log terminal — the panel just renders them). Mutating ones
// follow the same `reset_job` → run → `reset_job` shape as
// `discover_install`/`discover_uninstall` above, so Cancel and the shared
// TerminalLog work on them exactly the same way.

#[tauri::command]
async fn nix_list_generations() -> Result<Vec<hnm::NixGeneration>, String> {
    hnm::list_generations().await
}

#[tauri::command]
async fn nix_store_size() -> String {
    hnm::store_size().await
}

#[tauri::command]
async fn nix_list_installed() -> Vec<hnm::NixInstalledPkg> {
    hnm::list_installed().await
}

#[tauri::command]
async fn nix_env_status() -> Result<String, String> {
    hnm::env_status().await
}

#[tauri::command]
async fn nix_doctor() -> Result<String, String> {
    hnm::doctor().await
}

#[tauri::command]
async fn nix_check() -> Result<String, String> {
    hnm::check().await
}

#[tauri::command]
async fn nix_which(package: String) -> Option<String> {
    hnm::which(&package).await
}

#[tauri::command]
async fn nix_rollback(app: tauri::AppHandle, generation: u32) -> Result<String, String> {
    reset_job(&app);
    let result = hnm::rollback(&app, generation).await;
    reset_job(&app);
    result
}

#[tauri::command]
async fn nix_pin(app: tauri::AppHandle, package: String, version: Option<String>) -> Result<String, String> {
    reset_job(&app);
    let result = hnm::pin(&app, &package, version.as_deref()).await;
    reset_job(&app);
    result
}

#[tauri::command]
async fn nix_unpin(app: tauri::AppHandle, package: String) -> Result<String, String> {
    reset_job(&app);
    let result = hnm::unpin(&app, &package).await;
    reset_job(&app);
    result
}

#[tauri::command]
async fn nix_gc(app: tauri::AppHandle) -> Result<String, String> {
    reset_job(&app);
    let result = hnm::gc(&app).await;
    reset_job(&app);
    result
}

#[tauri::command]
async fn nix_clean(app: tauri::AppHandle) -> Result<String, String> {
    reset_job(&app);
    let result = hnm::clean(&app).await;
    reset_job(&app);
    result
}

/// Backs both the Nix panel's own "Rebuild index" button and Settings'
/// "Build Nix index" quick action (see `SettingsView.tsx`) — same command
/// either way, there's no cheaper partial variant `hnm update` exposes.
#[tauri::command]
async fn nix_update_index(app: tauri::AppHandle) -> Result<String, String> {
    reset_job(&app);
    let result = hnm::update_index(&app).await;
    reset_job(&app);
    // The whole point of rebuilding the index is to make search results
    // change — without this, cached "index not built" / stale nix results
    // could stick around in Discover for up to `CACHE_TTL` even though the
    // index is now fresh (see `invalidate_source_cache`'s doc comment).
    if result.is_ok() { invalidate_source_cache(&app, "nix").await; }
    result
}

#[tauri::command]
async fn nix_env_activate(app: tauri::AppHandle) -> Result<String, String> {
    reset_job(&app);
    let result = hnm::env_activate(&app).await;
    reset_job(&app);
    result
}

#[tauri::command]
async fn nix_env_deactivate(app: tauri::AppHandle) -> Result<String, String> {
    reset_job(&app);
    let result = hnm::env_deactivate(&app).await;
    reset_job(&app);
    result
}

/// Manual "refresh catalog" action for the AppImageHub feed, so a person
/// isn't stuck waiting up to 24h (`FEED_TTL_SECS`) for a newly-published
/// AppImage to show up in search after this app's local cache was last
/// updated.
#[tauri::command]
async fn refresh_appimage_feed(app: tauri::AppHandle) -> Result<usize, String> {
    let result = appimage::refresh_feed().await;
    // Same reasoning as `nix_update_index` above: a manual feed refresh
    // is pointless if Discover keeps serving cached pre-refresh AppImage
    // results/issues for up to `CACHE_TTL` afterwards.
    if result.is_ok() { invalidate_source_cache(&app, "appimage").await; }
    result
}

#[tauri::command]
fn get_persisted_queue() -> Vec<queue_store::PersistedJob> {
    queue_store::load()
}

#[tauri::command]
fn save_persisted_queue(jobs: Vec<queue_store::PersistedJob>) -> Result<(), String> {
    queue_store::save(&jobs)
}

#[tauri::command]
async fn get_package_info(name: String, category: String) -> serde_json::Value {
    // Same split as `install_package`/`uninstall_package`: only
    // `pentest_tools` names are ever fed to apt/dpkg as a raw token here
    // (via `apt_pkg_name`), so only that branch needs the strict allowlist.
    // The others just key into a static Rust catalog, so a rejection there
    // would incorrectly turn e.g. "NVIDIA Driver"'s info button into a
    // permanent error.
    let name = match validate_display_name(&name) {
        Ok(n) => n,
        Err(e) => return serde_json::json!({"size": null, "version": null, "note": e}),
    };
    match category.as_str() {
        "game_launchers" => {
            let id = launcher_flatpak_id(&name);
            if !id.is_empty() {
                flatpak_remote_info(id).await
            } else {
                serde_json::json!({"size": null, "version": null, "note": "Downloaded via Wine — size known only after install."})
            }
        },
        "pentest_tools" => {
            if in_debian(&name) {
                match validate_pkg_token(&apt_pkg_name(&name)) {
                    Ok(pkg) => apt_show_info(&pkg).await,
                    Err(e) => serde_json::json!({"size": null, "version": null, "note": e}),
                }
            } else { serde_json::json!({"size": null, "version": null, "note": "Installed inside the Kali container — size not tracked by apt."}) }
        },
        "drivers" => {
            // `driver_pkgs` returns every apt package a driver entry
            // installs (e.g. "NVIDIA Driver" -> nvidia-driver +
            // firmware-misc-nonfree) — show info for the first/main one
            // rather than `apt show`-ing the display name itself, which
            // isn't a real package and would always fail.
            match driver_pkgs(&name).ok().and_then(|p| p.first().copied()) {
                Some(pkg) => apt_show_info(pkg).await,
                None => serde_json::json!({"size": null, "version": null, "note": format!("Unknown driver: {name}")}),
            }
        },
        "hackeros_ecosystem" => serde_json::json!({
            "size": null, "version": null,
            "note": "Managed by the `hacker` CLI (unpack/pack) — size and version aren't tracked here.",
        }),
        "dev_tools" => match dev_tool_entry(&name) {
            Some((_, _, pkg, _, false)) => apt_show_info(pkg).await,
            Some((_, _, _, _, true)) => serde_json::json!({
                "size": null, "version": null,
                "note": "Installed inside the hackeros-devbox Podman container — size not tracked by apt on the host.",
            }),
            None => serde_json::json!({"size": null, "version": null, "note": format!("Unknown Dev Tools entry: {name}")}),
        },
        _ => serde_json::json!({"size":null,"version":null}),
    }
}

// ─── Settings ─────────────────────────────────────────────────────────────────

fn settings_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    std::path::PathBuf::from(format!("{home}/.hackeros/store/settings.json"))
}

fn current_settings() -> AppSettings {
    let mut settings: AppSettings = std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    // Migration: settings.json written before `flatpak_remotes` existed
    // only has the old single `flatpak_remote_url` string. If that's the
    // situation we're in (remotes list still empty after deserializing —
    // `default_flatpak_remotes` only kicks in when the key is *absent*,
    // and an old file never has it), fold the legacy URL into a "flathub"
    // remote entry instead of silently discarding a custom mirror.
    if settings.flatpak_remotes.is_empty() {
        settings.flatpak_remotes = vec![FlatpakRemote {
            name: "flathub".into(),
            url: settings.flatpak_remote_url.clone(),
        }];
    }
    settings
}

#[tauri::command]
fn get_settings() -> AppSettings {
    current_settings()
}

#[tauri::command]
fn save_settings(settings: AppSettings) -> Result<(), String> {
    let path = settings_path();
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir).map_err(|e| e.to_string())?; }
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn reset_settings() -> AppSettings {
    let _ = std::fs::remove_file(settings_path());
    AppSettings::default()
}

/// Expands a leading `~/` to the user's home directory — the only shell-ism
/// worth supporting here, since a person typing a backup path by hand will
/// naturally reach for it. Anything else (absolute or relative paths) is
/// passed through untouched.
fn expand_home_path(path: &str) -> std::path::PathBuf {
    let path = path.trim();
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        std::path::PathBuf::from(format!("{home}/{rest}"))
    } else if path.is_empty() {
        default_settings_backup_path()
    } else {
        std::path::PathBuf::from(path)
    }
}

fn default_settings_backup_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    std::path::PathBuf::from(format!("{home}/hackeros-store-settings.json"))
}

/// Writes the current settings to a standalone JSON file so a person can
/// carry their configuration (enabled sources, flatpak remotes, language,
/// theme, etc.) to a reinstalled system or a second machine — `save_settings`
/// only ever writes to the app's own fixed `settings.json`, this is the
/// export half of that. `path` defaults to `~/hackeros-store-settings.json`
/// when empty. Returns the path actually written to, so the UI can show it.
#[tauri::command]
fn export_settings_snapshot(path: Option<String>) -> Result<String, String> {
    let settings = current_settings();
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    let dest = expand_home_path(&path.unwrap_or_default());
    if let Some(dir) = dest.parent() { std::fs::create_dir_all(dir).map_err(|e| e.to_string())?; }
    std::fs::write(&dest, json).map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().to_string())
}

/// Reads a previously-exported settings JSON file and makes it the app's
/// active settings (via the same `save_settings` path a normal Settings
/// save uses) — the counterpart to `export_settings_snapshot`. Rejects a
/// file that isn't valid `AppSettings` JSON rather than partially applying
/// it, so a typo'd or corrupted backup can't silently wipe out working
/// configuration.
#[tauri::command]
fn import_settings_snapshot(path: Option<String>) -> Result<AppSettings, String> {
    let src = expand_home_path(&path.unwrap_or_default());
    let text = std::fs::read_to_string(&src)
        .map_err(|e| format!("Couldn't read {}: {e}", src.display()))?;
    let settings: AppSettings = serde_json::from_str(&text)
        .map_err(|e| format!("That doesn't look like a HackerOS Store settings file: {e}"))?;
    save_settings(settings.clone())?;
    Ok(settings)
}

#[tauri::command]
async fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "HackerOS Store".to_string(),
        target_release: "Debian trixie (testing) and forks".to_string(),
        pkg_backend: pkgbackend::backend_display_label().await,
    }
}

#[tauri::command]
async fn clear_cache(app: tauri::AppHandle) -> Result<String, String> {
    emit_log(&app, "info", "Clearing package manager caches...");
    emit_prog(&app, "cache", "Clearing caches...", 0.2);

    let _ = pkgbackend::clean(&app).await;
    emit_prog(&app, "cache", &format!("Cleared {} cache...", pkgbackend::backend_label().await), 0.45);

    let _ = run_streaming(&app, &["flatpak", "uninstall", "-y", "--user", "--unused"]).await;
    let _ = priv_run(&app, &["flatpak", "uninstall", "-y", "--unused"]).await;
    emit_prog(&app, "cache", "Cleared unused Flatpak runtimes...", 0.7);

    // Native directory walk instead of a shell glob (`rm -f .../*/installer.exe`),
    // so no shell is invoked at all for this cleanup step.
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let launchers_dir = format!("{home}/.hackeros/launchers");
    if let Ok(entries) = std::fs::read_dir(&launchers_dir) {
        for entry in entries.flatten() {
            let installer = entry.path().join("installer.exe");
            if installer.is_file() {
                let _ = std::fs::remove_file(&installer);
            }
        }
    }
    emit_prog(&app, "cache", "Cleared downloaded Wine installers...", 0.9);

    emit_prog(&app, "done", "Cache cleared.", 1.0);
    emit_log(&app, "success", "Caches cleared successfully.");
    Ok("Caches cleared successfully.".into())
}

// ─── run ──────────────────────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(JobState::default())
        .manage(DiscoverCacheState::default())
        .manage(SourceCacheState::default())
        .invoke_handler(tauri::generate_handler![
            install_package,
            uninstall_package,
            cancel_install,
            update_system,
            check_updates_available,
            check_all_installed,
            discover_categories,
            discover_browse,
            discover_search,
            discover_install,
            discover_uninstall,
            get_app_details,
            get_installed_sets,
            get_package_info,
            get_settings,
            save_settings,
            reset_settings,
            export_settings_snapshot,
            import_settings_snapshot,
            get_app_info,
            clear_cache,
            submit_rating,
            get_local_rating,
            get_reviews,
            get_install_history,
            clear_install_history,
            rollback_history_entry,
            is_brew_available,
            is_flatpak_available,
            is_snap_available,
            is_hpm_available,
            is_nix_available,
            is_apt_available,
            is_hacker_available,
            is_podman_available,
            nix_list_generations,
            nix_store_size,
            nix_list_installed,
            nix_env_status,
            nix_doctor,
            nix_check,
            nix_which,
            nix_rollback,
            nix_pin,
            nix_unpin,
            nix_gc,
            nix_clean,
            nix_update_index,
            nix_env_activate,
            nix_env_deactivate,
            refresh_appimage_feed,
            get_persisted_queue,
            save_persisted_queue,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
