use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

use crate::history::PackageVersion;
use crate::{DiscoverResult, SourceIssue};

// ─────────────────────────────────────────────────────────────────────────
//  Backend / mode detection
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Apt,
    Hammer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HammerMode {
    /// Classic CLI: `hammer install/remove/search/...` — behaves like a
    /// drop-in apt-get/apt-cache replacement.
    Normal,
    /// Image-based/atomic CLI: `hammer oci install/uninstall/...` —
    /// changes are layered and require a reboot to activate.
    Oci,
}

async fn on_path(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Resolves + caches the `hammer` binary path, the same way `hpm.rs`
/// resolves `hpm`: prefers the well-known `/usr/bin/hammer` HackerOS
/// install path, falls back to `$PATH` (dev machines / non-standard
/// installs).
async fn hammer_bin() -> Option<&'static str> {
    static RESOLVED: tokio::sync::OnceCell<Option<String>> = tokio::sync::OnceCell::const_new();
    let resolved = RESOLVED
        .get_or_init(|| async {
            if std::path::Path::new("/usr/bin/hammer").is_file() {
                return Some("/usr/bin/hammer".to_string());
            }
            if on_path("hammer").await {
                return Some("hammer".to_string());
            }
            None
        })
        .await;
    resolved.as_deref()
}

/// Which backend actually manages Debian packages on this system.
/// Resolved once and cached for the process lifetime:
///   - `apt-get` present on `$PATH` -> [`Backend::Apt`] (unchanged
///     behaviour from before hammer support existed).
///   - otherwise, if hammer is found (`/usr/bin/hammer` or `$PATH`) ->
///     [`Backend::Hammer`].
///   - otherwise -> [`Backend::Apt`], so every existing "apt-get is not
///     installed" error message still fires exactly as it did before.
pub async fn backend() -> Backend {
    static RESOLVED: tokio::sync::OnceCell<Backend> = tokio::sync::OnceCell::const_new();
    *RESOLVED
        .get_or_init(|| async {
            if on_path("apt-get").await {
                Backend::Apt
            } else if hammer_bin().await.is_some() {
                Backend::Hammer
            } else {
                Backend::Apt
            }
        })
        .await
}

/// Public accessor for the resolved `hammer` binary path, for the rare
/// caller outside this module that needs to shell out to a hammer
/// subcommand this module doesn't itself wrap yet (e.g. `hammer repo add`
/// in `ensure_nonfree_hammer`). Returns `None` if hammer isn't installed.
pub async fn hammer_bin_pub() -> Option<String> {
    hammer_bin().await.map(|s| s.to_string())
}

/// `true` if either apt or hammer is actually usable right now — lets a
/// caller short-circuit with a clean error instead of shelling out to a
/// binary that isn't there.
pub async fn is_available() -> bool {
    match backend().await {
        Backend::Apt => on_path("apt-get").await,
        Backend::Hammer => hammer_bin().await.is_some(),
    }
}

/// Human-readable label for the active backend, surfaced in the UI
/// (About/Settings panel via `get_app_info`) so a person can tell which
/// package manager Discover's "apt" source is actually talking to.
pub async fn backend_label() -> &'static str {
    match backend().await {
        Backend::Apt => "apt",
        Backend::Hammer => "hammer",
    }
}

/// Same as [`backend_label`], but also folds in the hammer mode
/// ("hammer (normal)" / "hammer (oci)") when relevant.
pub async fn backend_display_label() -> String {
    match backend().await {
        Backend::Apt => "apt".to_string(),
        Backend::Hammer => match hammer_mode().await {
            HammerMode::Normal => "hammer (normal)".to_string(),
            HammerMode::Oci => "hammer (oci)".to_string(),
        },
    }
}

/// Detects whether the active `hammer` install is running in OCI
/// (image-based/atomic) mode or normal (classic) mode.
///
///  - `HACKEROS_HAMMER_MODE=oci|normal` env var always wins, for anyone
///    (operators, CI, packaging scripts) who wants to force one
///    explicitly instead of relying on auto-detection.
///  - Otherwise: `hammer oci status` is run and its exit status used as
///    the signal. It opens a real OSTree sysroot at `/`
///    (`ostree_sysroot_load` under the hood, see hammer's
///    `oci/sysroot.rs`), which only succeeds on an actual OCI/atomic
///    rootfs and fails cleanly (non-zero exit) on a normal filesystem —
///    the same distinction hammer's own CLI guard
///    (`build_mode::NORMAL_MODE`) cares about at a different level.
pub async fn hammer_mode() -> HammerMode {
    static RESOLVED: tokio::sync::OnceCell<HammerMode> = tokio::sync::OnceCell::const_new();
    *RESOLVED
        .get_or_init(|| async {
            if let Ok(v) = std::env::var("HACKEROS_HAMMER_MODE") {
                match v.trim().to_ascii_lowercase().as_str() {
                    "oci" => return HammerMode::Oci,
                    "normal" | "classic" => return HammerMode::Normal,
                    _ => {}
                }
            }
            let Some(bin) = hammer_bin().await else {
                return HammerMode::Normal;
            };
            let ok = Command::new(bin)
                .args(["oci", "status"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                HammerMode::Oci
            } else {
                HammerMode::Normal
            }
        })
        .await
}

async fn run_timeout(mut cmd: Command, secs: u64) -> Option<std::process::Output> {
    tokio::time::timeout(std::time::Duration::from_secs(secs), cmd.output())
        .await
        .ok()?
        .ok()
}

async fn run_timeout_reported(
    mut cmd: Command,
    secs: u64,
    source: &str,
) -> Result<std::process::Output, SourceIssue> {
    match tokio::time::timeout(std::time::Duration::from_secs(secs), cmd.output()).await {
        Err(_) => Err(SourceIssue {
            source: source.into(),
            kind: "timeout".into(),
            message: format!("{source} did not respond within {secs}s."),
        }),
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => Err(SourceIssue {
            source: source.into(),
            kind: "unavailable".into(),
            message: format!("{source} is not installed on this system."),
        }),
        Ok(Err(e)) => Err(SourceIssue {
            source: source.into(),
            kind: "error".into(),
            message: format!("{source} failed: {e}"),
        }),
        Ok(Ok(out)) => Ok(out),
    }
}

// ─────────────────────────────────────────────────────────────────────────
//  Mutating actions: install / remove / update index / clean / downgrade
// ─────────────────────────────────────────────────────────────────────────

/// Installs one or more packages.
///  - apt: `apt-get install -y --no-install-recommends <pkgs>` (unchanged).
///  - hammer/normal: `hammer install -y --no-recommends <pkgs>` — same
///    flags, same semantics.
///  - hammer/oci: `hammer oci install <pkgs>` — layers the package onto a
///    new OSTree commit; the person is told a reboot is needed before it
///    takes effect (oci installs/uninstalls never happen "live" the way
///    apt/normal-hammer ones do).
///
/// All three still need root, exactly like the old direct `apt-get`
/// calls did, so this goes through `priv_run` (pkexec/sudo) either way.
pub async fn install(app: &tauri::AppHandle, pkgs: &[&str]) -> Result<(), String> {
    if pkgs.is_empty() {
        return Ok(());
    }
    match backend().await {
        Backend::Apt => {
            let mut args = vec!["apt-get", "install", "-y", "--no-install-recommends"];
            args.extend_from_slice(pkgs);
            crate::priv_run(app, &args).await
        }
        Backend::Hammer => {
            let Some(bin) = hammer_bin().await else {
                return Err("Neither apt-get nor hammer is installed on this system.".to_string());
            };
            match hammer_mode().await {
                HammerMode::Normal => {
                    let mut args = vec![bin, "install", "-y", "--no-recommends"];
                    args.extend_from_slice(pkgs);
                    crate::priv_run(app, &args).await
                }
                HammerMode::Oci => {
                    let mut args = vec![bin, "oci", "install"];
                    args.extend_from_slice(pkgs);
                    crate::priv_run(app, &args).await?;
                    crate::emit_log(
                        app,
                        "info",
                        "hammer oci: package layered onto a new deployment — reboot to activate it.",
                    );
                    Ok(())
                }
            }
        }
    }
}

/// Removes one or more packages — mirror of [`install`] for the removal
/// side (`apt-get remove -y` / `hammer remove -y` / `hammer oci
/// uninstall`).
pub async fn remove(app: &tauri::AppHandle, pkgs: &[&str]) -> Result<(), String> {
    if pkgs.is_empty() {
        return Ok(());
    }
    match backend().await {
        Backend::Apt => {
            let mut args = vec!["apt-get", "remove", "-y"];
            args.extend_from_slice(pkgs);
            crate::priv_run(app, &args).await
        }
        Backend::Hammer => {
            let Some(bin) = hammer_bin().await else {
                return Err("Neither apt-get nor hammer is installed on this system.".to_string());
            };
            match hammer_mode().await {
                HammerMode::Normal => {
                    let mut args = vec![bin, "remove", "-y"];
                    args.extend_from_slice(pkgs);
                    crate::priv_run(app, &args).await
                }
                HammerMode::Oci => {
                    let mut args = vec![bin, "oci", "uninstall"];
                    args.extend_from_slice(pkgs);
                    crate::priv_run(app, &args).await?;
                    crate::emit_log(
                        app,
                        "info",
                        "hammer oci: package removed from a new deployment — reboot to activate it.",
                    );
                    Ok(())
                }
            }
        }
    }
}

/// Refreshes the package index. Apt: `apt-get update -qq`. Hammer
/// normal: `hammer sync`. Hammer oci: `hammer oci update`.
pub async fn update_index(app: &tauri::AppHandle) -> Result<(), String> {
    match backend().await {
        Backend::Apt => crate::priv_run(app, &["apt-get", "update", "-qq"]).await,
        Backend::Hammer => {
            let Some(bin) = hammer_bin().await else {
                return Err("Neither apt-get nor hammer is installed on this system.".to_string());
            };
            match hammer_mode().await {
                HammerMode::Normal => crate::priv_run(app, &[bin, "sync"]).await,
                HammerMode::Oci => crate::priv_run(app, &[bin, "oci", "update"]).await,
            }
        }
    }
}

/// Clears the package manager's local cache. Apt: `apt-get clean`.
/// Hammer normal: `hammer clean`. Hammer oci: `hammer oci cleanup`
/// (drops old, unpinned deployments — the closest oci-mode equivalent).
pub async fn clean(app: &tauri::AppHandle) -> Result<(), String> {
    match backend().await {
        Backend::Apt => crate::priv_run(app, &["apt-get", "clean"]).await,
        Backend::Hammer => {
            let Some(bin) = hammer_bin().await else {
                return Ok(()); // best-effort, same as the old apt path ignoring errors
            };
            let res = match hammer_mode().await {
                HammerMode::Normal => crate::priv_run(app, &[bin, "clean"]).await,
                HammerMode::Oci => crate::priv_run(app, &[bin, "oci", "cleanup"]).await,
            };
            // Cache-clearing is best-effort everywhere this is called from
            // (see `clear_cache` in lib.rs, which already ignores the
            // Result of the old apt-get-clean call) — don't fail the whole
            // "clear cache" action just because this one step didn't work.
            let _ = &res;
            Ok(())
        }
    }
}

/// Re-installs a package pinned to a specific (older) version — used by
/// history rollback.
///  - apt: `apt-get install -y --allow-downgrades <pkg>=<version>`
///    (unchanged, best-effort as before).
///  - hammer/normal: `hammer install -y <pkg>=<version>` — best-effort in
///    the same spirit; hammer's solver may or may not accept the
///    `name=version` pin syntax for a given package, in which case this
///    surfaces hammer's own error message to the person, same as an apt
///    pin failure would.
///  - hammer/oci: there's no per-package downgrade in image-based mode —
///    the unit of rollback is a whole deployment
///    (`hammer oci rollback`/`hammer oci status`), not one layered
///    package. Returns a clear explanatory error instead of silently
///    doing the wrong thing.
pub async fn downgrade_to(
    app: &tauri::AppHandle,
    package_id: &str,
    version: &str,
) -> Result<(), String> {
    let spec = format!("{package_id}={version}");
    match backend().await {
        Backend::Apt => {
            crate::priv_run(app, &["apt-get", "install", "-y", "--allow-downgrades", &spec]).await
        }
        Backend::Hammer => {
            let Some(bin) = hammer_bin().await else {
                return Err("Neither apt-get nor hammer is installed on this system.".to_string());
            };
            match hammer_mode().await {
                HammerMode::Normal => crate::priv_run(app, &[bin, "install", "-y", &spec]).await,
                HammerMode::Oci => Err(format!(
                    "hammer oci mode doesn't support pinning a single package ({package_id}) back \
                     to an older version — packages are layered onto whole deployments instead. \
                     Use 'hammer oci rollback' to return to the previous deployment as a whole, \
                     or 'hammer oci status' to see what's available."
                )),
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
//  Query: search / info / installed versions
// ─────────────────────────────────────────────────────────────────────────

/// Discover "apt" source search. Apt: `apt-cache search` + a bulk
/// `dpkg-query` pass for installed versions (unchanged from the original
/// `search_apt`). Hammer normal: `hammer search --json`, which already
/// carries install state per result. Hammer oci: `hammer oci search`
/// (plain text — oci mode has no `--json` output for this), installed
/// state left unknown (oci mode has no cheap "is this installed"
/// lookup outside of `hammer oci list`, which only enumerates *layered*
/// packages, not the whole base image).
pub async fn search(query: String) -> Result<Vec<DiscoverResult>, SourceIssue> {
    match backend().await {
        Backend::Apt => search_apt(query).await,
        Backend::Hammer => match hammer_mode().await {
            HammerMode::Normal => search_hammer_normal(query).await,
            HammerMode::Oci => search_hammer_oci(query).await,
        },
    }
}

async fn search_apt(query: String) -> Result<Vec<DiscoverResult>, SourceIssue> {
    let mut cmd = Command::new("apt-cache");
    cmd.args(["search", "--names-only", &query]);
    let out = run_timeout_reported(cmd, 4, "apt").await?;
    let items: Vec<(String, String)> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .take(14)
        .filter_map(|l| {
            let mut p = l.splitn(2, " - ");
            let n = p.next()?.trim().to_string();
            let d = p.next().unwrap_or("").trim().to_string();
            if n.is_empty() {
                return None;
            }
            Some((n, d))
        })
        .collect();
    if items.is_empty() {
        return Ok(vec![]);
    }
    let names: Vec<&str> = items.iter().map(|(n, _)| n.as_str()).collect();
    let mut dpkg_cmd = Command::new("dpkg-query");
    dpkg_cmd.arg("-W").arg("-f=${Package} ${Version}\n").args(&names);
    let dpkg = run_timeout(dpkg_cmd, 3)
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let mut vm: HashMap<String, String> = HashMap::new();
    for line in dpkg.lines() {
        let mut p = line.splitn(2, ' ');
        let pkg = p.next().unwrap_or("").to_string();
        let ver = p.next().unwrap_or("").trim().to_string();
        vm.insert(pkg, ver);
    }
    Ok(items
        .into_iter()
        .map(|(name, desc)| {
            let ver = vm.get(&name).cloned().unwrap_or_default();
            DiscoverResult {
                name: name.clone(),
                version: ver,
                desc,
                source: "apt".into(),
                package_id: name,
                size: None,
                icon: None,
            }
        })
        .collect())
}

async fn search_hammer_normal(query: String) -> Result<Vec<DiscoverResult>, SourceIssue> {
    let Some(bin) = hammer_bin().await else {
        return Err(SourceIssue {
            source: "apt".into(),
            kind: "unavailable".into(),
            message: "hammer is not installed on this system (expected /usr/bin/hammer).".into(),
        });
    };
    let mut cmd = Command::new(bin);
    cmd.args(["search", &query, "--json"]);
    let out = run_timeout_reported(cmd, 8, "apt").await?;
    let text = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let arr = parsed.as_array().cloned().unwrap_or_default();
    Ok(arr
        .into_iter()
        .take(14)
        .filter_map(|v| {
            let name = v.get("name")?.as_str()?.to_string();
            let version = v.get("version").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let desc = v
                .get("summary")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            Some(DiscoverResult {
                name: name.clone(),
                version,
                desc,
                source: "apt".into(),
                package_id: name,
                size: None,
                icon: None,
            })
        })
        .collect())
}

/// `hammer oci search`'s output is plain text (`  name version — desc`) —
/// there's no `--json` mode for it. Parsed defensively; any line that
/// doesn't match the expected shape is just skipped rather than failing
/// the whole search.
async fn search_hammer_oci(query: String) -> Result<Vec<DiscoverResult>, SourceIssue> {
    let Some(bin) = hammer_bin().await else {
        return Err(SourceIssue {
            source: "apt".into(),
            kind: "unavailable".into(),
            message: "hammer is not installed on this system (expected /usr/bin/hammer).".into(),
        });
    };
    let mut cmd = Command::new(bin);
    cmd.args(["oci", "search", &query]);
    let out = run_timeout_reported(cmd, 10, "apt").await?;
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text
        .lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.is_empty() || t.starts_with("No packages") {
                return None;
            }
            let (head, desc) = match t.split_once('—') {
                Some((h, d)) => (h.trim(), d.trim().to_string()),
                None => (t, String::new()),
            };
            let mut parts = head.split_whitespace();
            let name = parts.next()?.to_string();
            let version = parts.next().unwrap_or("").to_string();
            Some((name, version, desc))
        })
        .take(14)
        .map(|(name, version, desc)| DiscoverResult {
            name: name.clone(),
            version,
            desc,
            source: "apt".into(),
            package_id: name,
            size: None,
            icon: None,
        })
        .collect())
}

/// App-detail-view info lookup (version/size/description). Apt:
/// `apt-cache show`. Hammer normal: `hammer info --json`. Hammer oci:
/// falls back to an exact-name `hammer oci search` match (oci mode has
/// no per-package "show full info" command).
pub async fn show_info(name: &str) -> serde_json::Value {
    match backend().await {
        Backend::Apt => show_info_apt(name).await,
        Backend::Hammer => match hammer_mode().await {
            HammerMode::Normal => show_info_hammer_normal(name).await,
            HammerMode::Oci => show_info_hammer_oci(name).await,
        },
    }
}

async fn show_info_apt(name: &str) -> serde_json::Value {
    let mut info = serde_json::json!({"size": null, "version": null, "description": null});
    let mut cmd = Command::new("apt-cache");
    cmd.args(["show", "--no-all-versions", name]);
    if let Some(out) = run_timeout(cmd, 4).await {
        let s = String::from_utf8_lossy(&out.stdout).to_string();
        for line in s.lines() {
            if line.starts_with("Version:") {
                info["version"] = serde_json::json!(line.trim_start_matches("Version:").trim());
            }
            if line.starts_with("Size:") || line.starts_with("Installed-Size:") {
                let kb: u64 = line.split_whitespace().last().unwrap_or("0").parse().unwrap_or(0);
                let sz = if kb > 1024 {
                    format!("{:.1} MB", kb as f64 / 1024.0)
                } else {
                    format!("{kb} KB")
                };
                info["size"] = serde_json::json!(sz);
            }
            if let Some(rest) = line
                .strip_prefix("Description-en:")
                .or_else(|| line.strip_prefix("Description:"))
            {
                info["description"] = serde_json::json!(rest.trim());
            }
        }
    }
    info
}

async fn show_info_hammer_normal(name: &str) -> serde_json::Value {
    let mut info = serde_json::json!({"size": null, "version": null, "description": null});
    let Some(bin) = hammer_bin().await else {
        return info;
    };
    let mut cmd = Command::new(bin);
    cmd.args(["info", name, "--json"]);
    if let Some(out) = run_timeout(cmd, 6).await {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
            if let Some(ver) = v.get("version").and_then(|x| x.as_str()) {
                info["version"] = serde_json::json!(ver);
            }
            if let Some(desc) = v.get("description").and_then(|x| x.as_str()) {
                info["description"] = serde_json::json!(desc);
            }
            // hammer's `info --json` doesn't carry a download/installed
            // size the way `apt-cache show`'s "Installed-Size:" field
            // does — left null, same as every other source that doesn't
            // expose one (matches the existing "size known only after
            // install" pattern used elsewhere in this crate).
        }
    }
    info
}

async fn show_info_hammer_oci(name: &str) -> serde_json::Value {
    let mut info = serde_json::json!({"size": null, "version": null, "description": null});
    let Some(bin) = hammer_bin().await else {
        return info;
    };
    let mut cmd = Command::new(bin);
    cmd.args(["oci", "search", name]);
    if let Some(out) = run_timeout(cmd, 8).await {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let t = line.trim();
            let Some((head, desc)) = t.split_once('—') else { continue };
            let mut parts = head.trim().split_whitespace();
            let Some(pkg_name) = parts.next() else { continue };
            if pkg_name != name {
                continue;
            }
            if let Some(ver) = parts.next() {
                info["version"] = serde_json::json!(ver);
            }
            info["description"] = serde_json::json!(desc.trim());
            break;
        }
    }
    info
}

// ─────────────────────────────────────────────────────────────────────────
//  Query: installed package names / versions (dpkg-query replacement)
// ─────────────────────────────────────────────────────────────────────────

/// Every installed package name — replacement for
/// `dpkg-query -W -f='${Package}\n'`. Hammer oci mode can only ever
/// report the *layered* packages on the booted deployment (there's no
/// cheap way to enumerate every package baked into the base image), so
/// results there are a subset of the true installed set — same
/// "best-effort, not exhaustive" caveat every other soft-fail source in
/// this crate already carries.
pub async fn installed_names() -> Vec<String> {
    match backend().await {
        Backend::Apt => {
            let mut cmd = Command::new("dpkg-query");
            cmd.args(["-W", "-f=${Package}\n"]);
            run_timeout(cmd, 4)
                .await
                .map(|out| {
                    String::from_utf8_lossy(&out.stdout)
                        .lines()
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default()
        }
        Backend::Hammer => installed_map().await.into_keys().collect(),
    }
}

/// Every installed package name -> version. Replacement for
/// `dpkg-query -W -f='${Package} ${Version}\n'`.
pub async fn installed_map() -> HashMap<String, String> {
    let mut vm = HashMap::new();
    match backend().await {
        Backend::Apt => {
            let mut cmd = Command::new("dpkg-query");
            cmd.args(["-W", "-f=${Package} ${Version}\n"]);
            if let Some(out) = run_timeout(cmd, 4).await {
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    let mut p = line.splitn(2, ' ');
                    let pkg = p.next().unwrap_or("").to_string();
                    let ver = p.next().unwrap_or("").trim().to_string();
                    if !pkg.is_empty() {
                        vm.insert(pkg, ver);
                    }
                }
            }
        }
        Backend::Hammer => {
            let Some(bin) = hammer_bin().await else {
                return vm;
            };
            match hammer_mode().await {
                HammerMode::Normal => {
                    let mut cmd = Command::new(bin);
                    cmd.args(["list", "--installed", "--json"]);
                    if let Some(out) = run_timeout(cmd, 6).await {
                        if let Ok(arr) = serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout) {
                            for p in arr {
                                let (Some(name), Some(ver)) = (
                                    p.get("name").and_then(|x| x.as_str()),
                                    p.get("version").and_then(|x| x.as_str()),
                                ) else {
                                    continue;
                                };
                                vm.insert(name.to_string(), ver.to_string());
                            }
                        }
                    }
                }
                HammerMode::Oci => {
                    // Only layered packages (see doc comment on
                    // `installed_names`) — plain-text "  ● name version".
                    let mut cmd = Command::new(bin);
                    cmd.args(["oci", "list"]);
                    if let Some(out) = run_timeout(cmd, 6).await {
                        for line in String::from_utf8_lossy(&out.stdout).lines() {
                            let t = line.trim_start_matches(['●', '○']).trim();
                            let mut parts = t.split_whitespace();
                            let (Some(name), Some(ver)) = (parts.next(), parts.next()) else {
                                continue;
                            };
                            vm.insert(name.to_string(), ver.to_string());
                        }
                    }
                }
            }
        }
    }
    vm
}

/// Best-effort lookup of a single installed package's version — used to
/// populate `HistoryEntry.version` right after an install/uninstall so a
/// later rollback has something to target.
pub async fn installed_version(package_id: &str) -> Option<String> {
    match backend().await {
        Backend::Apt => {
            let out = Command::new("dpkg-query")
                .args(["-W", "-f=${Version}", package_id])
                .output()
                .await
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        }
        Backend::Hammer => installed_map().await.get(package_id).cloned(),
    }
}

/// Same as [`installed_version`], but for several packages at once (used
/// by driver installs, which touch multiple packages). Packages that
/// aren't actually installed are silently omitted rather than failing
/// the whole lookup.
pub async fn installed_versions(package_ids: &[&str]) -> Vec<PackageVersion> {
    let map = installed_map().await;
    package_ids
        .iter()
        .filter_map(|&pkg| {
            map.get(pkg).map(|version| PackageVersion {
                name: pkg.to_string(),
                version: version.clone(),
            })
        })
        .collect()
}

/// Count of pending upgrades, used to badge the "Update System" nav item.
/// Apt: `apt list --upgradable`. Hammer normal: `hammer list --upgrades
/// --json`, counting the array. Hammer oci: upgrades are a whole-image
/// rebase rather than a per-package count in oci mode, so this returns 0
/// there (matches "not applicable" rather than a wrong-looking count) —
/// use `hammer oci status`/`hammer oci upgrade` directly for that flow.
pub async fn upgradable_count() -> u32 {
    match backend().await {
        Backend::Apt => {
            if let Ok(out) = Command::new("sh")
                .arg("-c")
                .arg("apt list --upgradable 2>/dev/null | grep -c upgradable")
                .output()
                .await
            {
                String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0)
            } else {
                0
            }
        }
        Backend::Hammer => match hammer_mode().await {
            HammerMode::Normal => {
                let Some(bin) = hammer_bin().await else { return 0 };
                let mut cmd = Command::new(bin);
                cmd.args(["list", "--upgrades", "--json"]);
                run_timeout(cmd, 6)
                    .await
                    .and_then(|out| serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout).ok())
                    .map(|arr| arr.len() as u32)
                    .unwrap_or(0)
            }
            HammerMode::Oci => 0,
        },
    }
}

/// `dpkg --add-architecture <arch>` replacement (used only by the Wine
/// i386 bootstrap). Apt: unchanged `dpkg --add-architecture`. Hammer
/// normal: `hammer dpkg-arch add <arch>`, its documented drop-in for
/// exactly this (see `hammer help` -> "Multi-arch"). Hammer oci: foreign
/// architectures aren't a supported concept for a single-arch OSTree
/// base image — returns a clear error instead of silently doing nothing.
pub async fn add_foreign_arch(app: &tauri::AppHandle, arch: &str) -> Result<(), String> {
    match backend().await {
        Backend::Apt => crate::priv_run(app, &["dpkg", "--add-architecture", arch]).await,
        Backend::Hammer => {
            let Some(bin) = hammer_bin().await else {
                return Err("Neither apt-get nor hammer is installed on this system.".to_string());
            };
            match hammer_mode().await {
                HammerMode::Normal => crate::priv_run(app, &[bin, "dpkg-arch", "add", arch]).await,
                HammerMode::Oci => Err(format!(
                    "hammer oci mode doesn't support adding a foreign architecture ({arch}) — the \
                     OSTree base image is single-arch by design."
                )),
            }
        }
    }
}
