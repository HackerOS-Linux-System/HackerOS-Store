use std::process::Stdio;
use tokio::process::Command;

use crate::security::validate_pkg_token;
use crate::{DiscoverResult, SourceIssue};

/// Prefers the well-known system path (`/usr/bin/hnm`), falling back to
/// whatever `hnm` resolves to on `$PATH` for a dev machine / non-standard
/// install — same resolution strategy as `hpm_bin()`. Cached after the
/// first successful resolution for the life of the process.
async fn hnm_bin() -> Option<&'static str> {
    static RESOLVED: tokio::sync::OnceCell<Option<String>> = tokio::sync::OnceCell::const_new();
    let resolved = RESOLVED.get_or_init(|| async {
        if std::path::Path::new("/usr/bin/hnm").is_file() {
            return Some("/usr/bin/hnm".to_string());
        }
        let ok = Command::new("which").arg("hnm").output().await
            .map(|o| o.status.success()).unwrap_or(false);
        if ok { Some("hnm".to_string()) } else { None }
    }).await;
    resolved.as_deref()
}

pub async fn is_available() -> bool { hnm_bin().await.is_some() }

fn unavailable_msg() -> String {
    "hnm is not installed on this system (expected /usr/bin/hnm).".to_string()
}

async fn run_timeout(mut cmd: Command, secs: u64) -> Option<std::process::Output> {
    cmd.stdin(Stdio::null());
    tokio::time::timeout(std::time::Duration::from_secs(secs), cmd.output()).await.ok()?.ok()
}

// ─── search ─────────────────────────────────────────────────────────────────

/// Mirrors `hnm`'s own `nix::Pkg` struct (see hnm's `src/nix.rs`) — only
/// the fields Discover actually shows are pulled out.
#[derive(serde::Deserialize)]
struct HnmPkg {
    name: String,
    version: String,
    description: String,
}

pub async fn search(query: String) -> Result<Vec<DiscoverResult>, SourceIssue> {
    let issue = |kind: &str, message: String| SourceIssue { source: "nix".into(), kind: kind.into(), message };
    let Some(bin) = hnm_bin().await else {
        return Err(issue("unavailable", "hnm is not installed on this system (expected /usr/bin/hnm).".into()));
    };
    let mut cmd = Command::new(bin);
    cmd.args(["search", &query, "--json"]);
    // A first-ever search can trigger `hnm unpack` (bootstrapping Nix
    // itself) under the hood — same generous timeout hpm gets for its
    // first-run index fetch, for the same reason.
    let Some(out) = run_timeout(cmd, 20).await else {
        return Err(issue("timeout", "hnm did not respond within 20s (first-run Nix bootstrap can be slow).".into()));
    };

    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        return Err(issue("error", format!("hnm search failed: {}", stderr.trim())));
    }

    // `hnm search --json` prints nothing on stdout at all (just a stderr
    // warning) in two distinct cases: the local package index hasn't been
    // built yet (`hnm update` was never run), or the query genuinely
    // matched nothing. Only the first one is worth surfacing as an issue
    // rather than a plain empty result set.
    if out.stdout.iter().all(|b| b.is_ascii_whitespace()) {
        if stderr.contains("package index not built") {
            return Err(issue(
                "error",
                "Nix package index not built yet — run `hnm update` once (Settings → Maintenance, or a terminal) to enable nix search.".into(),
            ));
        }
        return Ok(vec![]);
    }

    let pkgs: Vec<HnmPkg> = serde_json::from_slice(&out.stdout)
        .map_err(|e| issue("error", format!("could not parse hnm output: {e}")))?;

    Ok(pkgs.into_iter().take(30).map(|p| DiscoverResult {
        name: p.name.clone(),
        version: p.version,
        desc: p.description,
        source: "nix".into(),
        package_id: p.name,
        size: None,
        icon: None,
    }).collect())
}

// ─── info ───────────────────────────────────────────────────────────────────

pub struct HnmInfo {
    pub version: Option<String>,
    pub summary: String,
    pub description: String,
    pub homepage: Option<String>,
    pub license: Option<String>,
}

/// Parses `hnm info <pkg>`'s `"  {:>16}  {}"` label rows (see hnm's
/// `output::label`). Trimming the leading whitespace of the *whole* line
/// consumes both the fixed 2-space margin and the right-alignment padding
/// of the key column in one step, leaving `"{key}  {value}"` — so a plain
/// prefix-match on the bare key (guarded by a following whitespace char,
/// so e.g. "license" doesn't accidentally match a longer key starting
/// the same way) is enough; no fixed column offsets to keep in sync with
/// hnm's own formatting width.
fn strip_label<'a>(line: &'a str, label: &str) -> Option<String> {
    let rest = line.strip_prefix(label)?;
    if !rest.starts_with(|c: char| c.is_whitespace()) { return None; }
    Some(rest.trim().to_string())
}

pub async fn info(package: &str) -> Option<HnmInfo> {
    let bin = hnm_bin().await?;
    let mut cmd = Command::new(bin);
    cmd.args(["info", package]);
    let out = run_timeout(cmd, 10).await?;
    let text = String::from_utf8_lossy(&out.stdout);

    let mut d = HnmInfo {
        version: None, summary: String::new(), description: String::new(),
        homepage: None, license: None,
    };
    for raw in text.lines() {
        let line = raw.trim_start();
        if let Some(v) = strip_label(line, "description") { d.description = v; continue; }
        if let Some(v) = strip_label(line, "homepage")    { d.homepage = Some(v); continue; }
        if let Some(v) = strip_label(line, "license")     { d.license = Some(v); continue; }
        if let Some(v) = strip_label(line, "version")     { d.version = Some(v); continue; }
    }
    d.summary = d.description.chars().take(160).collect();
    Some(d)
}

// ─── list / installed set ───────────────────────────────────────────────────

/// Mirrors hnm's own `state::InstalledPkg` (see hnm's `src/state.rs`) —
/// `hnm list -i --json` dumps a `Vec<state::InstalledPkg>` verbatim, so
/// this struct's field names/types have to match that one exactly for
/// `serde_json` to pick everything up. Re-`Serialize`d as-is to the
/// frontend for the Nix panel's installed-packages table.
#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct NixInstalledPkg {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub attr_path: String,
    /// RFC3339 timestamp string (hnm serializes `chrono::DateTime<Utc>`
    /// this way) — passed straight through as a string rather than
    /// parsed, since the frontend only ever displays it.
    #[serde(default)]
    pub installed_at: String,
    pub pinned: Option<String>,
    pub description: Option<String>,
}

pub async fn list_installed() -> Vec<NixInstalledPkg> {
    let Some(bin) = hnm_bin().await else { return vec![]; };
    let mut cmd = Command::new(bin);
    cmd.args(["list", "-i", "--json"]);
    let Some(out) = run_timeout(cmd, 8).await else { return vec![]; };
    if !out.status.success() || out.stdout.iter().all(|b| b.is_ascii_whitespace()) { return vec![]; }
    serde_json::from_slice(&out.stdout).unwrap_or_default()
}

pub async fn installed_names() -> Vec<String> {
    list_installed().await.into_iter().map(|p| p.name).collect()
}

// ─── generations / rollback ─────────────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
pub struct NixGeneration {
    pub generation: u32,
    pub date: String,
    pub current: bool,
}

/// `~/.hnm/profile` — the fixed nix-env profile hnm installs into (see
/// hnm's `config::profile_dir()`; hnm's `config.hk` has a `profile_dir`
/// setting too, but nothing in hnm actually reads it back for this path,
/// so the real, always-in-effect location is this hardcoded one).
fn hnm_profile_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    format!("{home}/.hnm/profile")
}

/// A minimal version of hnm's own `pub_nix_env_vars()`-style PATH
/// augmentation, just enough for a bare read-only `nix-env` invocation to
/// resolve — `nix-env` itself isn't necessarily on this GUI process's
/// PATH unless hnm's shell integration has been activated (see
/// `env_activate` below).
fn augmented_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let current = std::env::var("PATH").unwrap_or_default();
    format!(
        "{home}/.hnm/profile/bin:{home}/.nix-profile/bin:/nix/var/nix/profiles/default/bin:{current}"
    )
}

/// Lists Nix profile generations directly via `nix-env --list-generations`
/// — deliberately *not* `hnm rollback` with no arguments, because that
/// command doesn't have a read-only "just list them" mode: given no
/// generation, it immediately computes the previous one and switches to
/// it (see hnm's `commands/rollback.rs`). Calling it just to populate this
/// panel would perform a real rollback as a side effect. `nix-env
/// --list-generations` is the same read-only query hnm's own `rollback`
/// command runs internally before deciding a target, so this mirrors it
/// safely instead.
pub async fn list_generations() -> Result<Vec<NixGeneration>, String> {
    let mut cmd = Command::new("nix-env");
    cmd.args(["--profile", &hnm_profile_dir(), "--list-generations"]);
    cmd.env("PATH", augmented_path());
    let Some(out) = run_timeout(cmd, 10).await else {
        return Err("nix-env did not respond within 10s.".into());
    };
    // No profile yet (nothing ever installed via hnm) is the common case
    // for a fresh install — treat it as "no generations" rather than an
    // error the panel has to show scarily.
    if !out.status.success() { return Ok(vec![]); }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut gens = vec![];
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 { continue; }
        let Ok(generation) = parts[0].parse::<u32>() else { continue; };
        let current = line.contains("(current)");
        let date = parts[1..].iter().filter(|s| **s != "(current)")
            .copied().collect::<Vec<_>>().join(" ");
        gens.push(NixGeneration { generation, date, current });
    }
    Ok(gens)
}

/// Total `/nix/store` size for display (e.g. in the Nix panel's header
/// stats, and before/after a `gc`). Read directly with `du`, same as
/// hnm's own `nix::store_du()` — this app never links hnm's crate, so it
/// can't call that function itself, only reproduce the equivalent shell
/// call.
pub async fn store_size() -> String {
    let mut cmd = Command::new("du");
    cmd.args(["-sh", "/nix/store"]);
    match run_timeout(cmd, 10).await {
        Some(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout)
                .split_whitespace().next().unwrap_or("?").to_string()
        }
        _ => "?".to_string(),
    }
}

/// The generation `list_generations()` marks `current: true` right now —
/// used to snapshot "which nix generation was active right after this
/// install/remove" for history/rollback (see `history.rs`'s
/// `record_with_generation` and its `"nix"` branch in `rollback_entry`).
pub async fn current_generation() -> Option<u32> {
    list_generations().await.ok()?.into_iter().find(|g| g.current).map(|g| g.generation)
}

/// Explicit-generation rollback only — never calls `hnm rollback` with no
/// argument (see `list_generations`'s doc comment for why that would be
/// unsafe to trigger from a button whose whole point is "let me pick
/// which generation").
pub async fn rollback(app: &tauri::AppHandle, generation: u32) -> Result<String, String> {
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    crate::emit_log(app, "info", &format!("Rolling back to Nix generation {generation} via hnm..."));
    crate::emit_prog(app, "rollback", &format!("Switching to generation {generation}..."), 0.2);
    let gen_str = generation.to_string();
    crate::run_streaming(app, &[bin, "rollback", &gen_str]).await?;
    crate::emit_prog(app, "done", "Rolled back.", 1.0);
    Ok(format!("Rolled back to generation {generation}."))
}

// ─── pin / unpin ─────────────────────────────────────────────────────────────

pub async fn pin(app: &tauri::AppHandle, package: &str, version: Option<&str>) -> Result<String, String> {
    let package = validate_pkg_token(package)?;
    let version = match version {
        Some(v) if !v.trim().is_empty() => Some(validate_pkg_token(v.trim())?),
        _ => None,
    };
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    crate::emit_log(app, "info", &format!("Pinning {package} via hnm..."));
    crate::emit_prog(app, "pin", &format!("Pinning {package}..."), 0.3);
    let mut argv: Vec<&str> = vec![bin, "pin", package.as_str()];
    if let Some(v) = &version { argv.push(v.as_str()); }
    crate::run_streaming(app, &argv).await?;
    crate::emit_prog(app, "done", "Pinned.", 1.0);
    Ok(format!("Pinned {package}{}.", version.map(|v| format!(" to {v}")).unwrap_or_default()))
}

pub async fn unpin(app: &tauri::AppHandle, package: &str) -> Result<String, String> {
    let package = validate_pkg_token(package)?;
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    crate::emit_log(app, "info", &format!("Unpinning {package} via hnm..."));
    crate::emit_prog(app, "unpin", &format!("Unpinning {package}..."), 0.3);
    crate::run_streaming(app, &[bin, "unpin", package.as_str()]).await?;
    crate::emit_prog(app, "done", "Unpinned.", 1.0);
    Ok(format!("Unpinned {package} — it will be updated by `hnm update` again."))
}

// ─── maintenance: gc / clean / update (index) ───────────────────────────────

pub async fn gc(app: &tauri::AppHandle) -> Result<String, String> {
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    crate::emit_log(app, "info", "Running Nix garbage collection via hnm (`nix-store --gc`)...");
    crate::emit_prog(app, "gc", "Collecting garbage...", 0.2);
    crate::run_streaming(app, &[bin, "gc"]).await?;
    crate::emit_prog(app, "done", "Garbage collection complete.", 1.0);
    Ok("Nix garbage collection complete — check the log for how much store space was freed.".into())
}

pub async fn clean(app: &tauri::AppHandle) -> Result<String, String> {
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    crate::emit_log(app, "info", "Cleaning hnm's download/eval cache...");
    crate::emit_prog(app, "clean", "Cleaning cache...", 0.3);
    crate::run_streaming(app, &[bin, "clean"]).await?;
    crate::emit_prog(app, "done", "Cache cleaned.", 1.0);
    Ok("hnm cache cleaned. Run Garbage Collection too if you want to reclaim /nix/store space.".into())
}

/// This is the "Build/refresh Nix index" action (both the Settings quick
/// action and the Nix panel's own button call this): `hnm update` doesn't
/// just refresh the local search index, it also refreshes the nixpkgs
/// channel and upgrades already-installed (non-pinned) packages — exactly
/// what a person running `hnm update` from a terminal would get, so the
/// GUI button does the same rather than a partial "index-only" variant
/// hnm doesn't actually expose.
pub async fn update_index(app: &tauri::AppHandle) -> Result<String, String> {
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    crate::emit_log(app, "info", "Refreshing the nixpkgs channel and rebuilding the local Nix package index (`hnm update`). This can take 1-2 minutes on the first run...");
    crate::emit_prog(app, "index", "Refreshing nixpkgs channel and package index...", 0.1);
    crate::run_streaming(app, &[bin, "update"]).await?;
    crate::emit_prog(app, "done", "Nix package index is up to date.", 1.0);
    Ok("Nix package index refreshed — Discover's nix search will now find nixpkgs packages.".into())
}

// ─── shell integration (env) ─────────────────────────────────────────────────

pub async fn env_activate(app: &tauri::AppHandle) -> Result<String, String> {
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    crate::emit_log(app, "info", "Activating hnm shell integration (patching ~/.bashrc, ~/.zshrc, ~/.profile)...");
    crate::emit_prog(app, "env", "Patching shell profile files...", 0.3);
    // `--yes`: without it, `hnm env activate` only *prints* the lines to
    // add — harmless either way, but pointless from a GUI button whose
    // whole purpose is to just do it.
    crate::run_streaming(app, &[bin, "env", "activate", "--yes"]).await?;
    crate::emit_prog(app, "done", "Shell integration activated.", 1.0);
    Ok("hnm shell integration activated — open a new terminal (or reload your shell) to use installed nix packages there.".into())
}

pub async fn env_deactivate(app: &tauri::AppHandle) -> Result<String, String> {
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    crate::emit_log(app, "info", "Deactivating hnm shell integration...");
    crate::emit_prog(app, "env", "Removing shell profile patch...", 0.3);
    crate::run_streaming(app, &[bin, "env", "deactivate", "--yes"]).await?;
    crate::emit_prog(app, "done", "Shell integration deactivated.", 1.0);
    Ok("hnm shell integration deactivated.".into())
}

/// Read-only, so unlike the actions above this doesn't go through
/// `run_streaming`/the log terminal — it just returns hnm's own report
/// text for the panel to show directly (same treatment as `doctor`/`check`
/// below).
pub async fn env_status() -> Result<String, String> {
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    let mut cmd = Command::new(bin);
    cmd.args(["env", "status"]);
    let Some(out) = run_timeout(cmd, 8).await else {
        return Err("hnm env status did not respond within 8s.".into());
    };
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// ─── diagnostics ─────────────────────────────────────────────────────────────

/// `hnm doctor` writes its "✓ ok" / "dim" lines to stdout and its warnings
/// to stderr (see hnm's `output.rs`) — both are concatenated (stdout
/// first) so a warning-only run doesn't come back looking empty.
async fn capture_report(bin: &str, args: &[&str], timeout_secs: u64, timeout_msg: &str) -> Result<String, String> {
    let mut cmd = Command::new(bin);
    cmd.args(args);
    let Some(out) = run_timeout(cmd, timeout_secs).await else {
        return Err(timeout_msg.to_string());
    };
    let mut text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let err = String::from_utf8_lossy(&out.stderr);
    let err = err.trim();
    if !err.is_empty() {
        if !text.is_empty() { text.push_str("\n\n"); }
        text.push_str(err);
    }
    Ok(text)
}

pub async fn doctor() -> Result<String, String> {
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    capture_report(bin, &["doctor"], 20, "hnm doctor did not respond within 20s.").await
}

pub async fn check() -> Result<String, String> {
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    capture_report(bin, &["check"], 10, "hnm check did not respond within 10s.").await
}

/// Quick "where is this binary" lookup for a row in the panel's installed
/// list — optional, best-effort (returns `None` on any failure rather
/// than surfacing an error for what's just a convenience detail).
pub async fn which(package: &str) -> Option<String> {
    let package = validate_pkg_token(package).ok()?;
    let bin = hnm_bin().await?;
    let mut cmd = Command::new(bin);
    cmd.args(["which", package.as_str()]);
    let out = run_timeout(cmd, 6).await?;
    if !out.status.success() { return None; }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

// ─── mutating actions ───────────────────────────────────────────────────────

pub async fn install(app: &tauri::AppHandle, spec: &str) -> Result<(), String> {
    let spec = validate_pkg_token(spec)?;
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    crate::emit_log(app, "info", &format!("Installing {spec} via hnm (Nix/nixpkgs)..."));
    crate::emit_prog(app, "install", &format!("Installing {spec}..."), 0.2);
    // `--no-env`: skip hnm's own "add this to your shell rc" PATH hint —
    // it's advice for an interactive terminal session, not useful in a
    // piped GUI log. The Nix panel's own "Activate shell integration"
    // button (`env_activate` above) covers the same need on demand.
    crate::run_streaming(app, &[bin, "install", spec.as_str(), "--no-env"]).await?;
    crate::emit_prog(app, "done", "Done!", 1.0);
    Ok(())
}

pub async fn remove(app: &tauri::AppHandle, spec: &str) -> Result<(), String> {
    let spec = validate_pkg_token(spec)?;
    let Some(bin) = hnm_bin().await else { return Err(unavailable_msg()); };
    crate::emit_log(app, "info", &format!("Removing {spec} via hnm..."));
    crate::emit_prog(app, "uninstall", &format!("Removing {spec}..."), 0.3);
    // `--force`: see the module doc comment above — without it, hnm's
    // `[y/N]` prompt reads stdin, sees EOF, and aborts every time when
    // driven from this GUI subprocess.
    crate::run_streaming(app, &[bin, "remove", spec.as_str(), "--force"]).await?;
    crate::emit_prog(app, "done", "Removed.", 1.0);
    Ok(())
}
