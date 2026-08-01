use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;

use crate::security::validate_pkg_token;

/// One apt package + the version it was at right after an install, used to
/// support rollback for multi-package curated installs (drivers).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackageVersion {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: String,
    /// "install" | "uninstall" | "update" | "rollback"
    pub action: String,
    /// "apt" | "flatpak" | "snap" | "brew" | "curated" | "system"
    pub source: String,
    pub name: String,
    pub package_id: String,
    /// Populated for single-package entries (Discover installs, apt-backed
    /// pentest tools). `None` when `packages` is used instead.
    pub version: Option<String>,
    /// Populated for multi-package entries (drivers). When present, this
    /// takes precedence over `version` for display and rollback purposes.
    #[serde(default)]
    pub packages: Option<Vec<PackageVersion>>,
    pub success: bool,
    pub message: Option<String>,
    /// Flatpak ostree commit hash active right after this install/update,
    /// used by `rollback_entry` to pin back to it later. `None` for every
    /// non-Flatpak entry, and for Flatpak entries recorded before this
    /// field existed (those fall back to the "not supported" message).
    #[serde(default)]
    pub commit: Option<String>,
    /// Nix profile generation number active right after this install/
    /// remove (via `hnm`), used the same way `commit` is for Flatpak —
    /// see this module's doc comment for why nix rollback is
    /// generation-wide rather than per-package. `None` for every
    /// non-nix entry and for nix entries recorded before this field
    /// existed.
    #[serde(default)]
    pub nix_generation: Option<u32>,
}

const MAX_HISTORY_ENTRIES: usize = 500;

fn history_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    PathBuf::from(format!("{home}/.hackeros/store/history.json"))
}

fn load() -> Vec<HistoryEntry> {
    std::fs::read_to_string(history_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(entries: &[HistoryEntry]) -> Result<(), String> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch:{secs}")
}

fn new_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("h{nanos}")
}

/// Appends one entry to the history log. Best-effort: a failure to persist
/// history should never fail the install/uninstall action itself, so
/// callers just ignore the `Result` (or log it) rather than propagating it.
pub fn record(
    action: &str,
    source: &str,
    name: &str,
    package_id: &str,
    version: Option<String>,
    success: bool,
    message: Option<String>,
) -> Result<(), String> {
    record_multi(action, source, name, package_id, version, None, success, message)
}

/// Same as [`record`], but also accepts a list of individual apt package
/// versions — used for driver installs, which touch several apt packages
/// at once (e.g. "NVIDIA Driver" -> `nvidia-driver` + `firmware-misc-nonfree`).
pub fn record_multi(
    action: &str,
    source: &str,
    name: &str,
    package_id: &str,
    version: Option<String>,
    packages: Option<Vec<PackageVersion>>,
    success: bool,
    message: Option<String>,
) -> Result<(), String> {
    push_entry(HistoryEntry {
        id: new_id(),
        timestamp: now_iso(),
        action: action.to_string(),
        source: source.to_string(),
        name: name.to_string(),
        package_id: package_id.to_string(),
        version,
        packages,
        success,
        message,
        commit: None,
        nix_generation: None,
    })
}

/// Same as [`record`], but also records the Flatpak ostree commit active
/// right after the action — the only source-specific extra a rollback
/// actually needs for Flatpak (see `flatpak_current_commit` in `lib.rs`).
pub fn record_with_commit(
    action: &str,
    source: &str,
    name: &str,
    package_id: &str,
    version: Option<String>,
    commit: Option<String>,
    success: bool,
    message: Option<String>,
) -> Result<(), String> {
    push_entry(HistoryEntry {
        id: new_id(),
        timestamp: now_iso(),
        action: action.to_string(),
        source: source.to_string(),
        name: name.to_string(),
        package_id: package_id.to_string(),
        version,
        packages: None,
        success,
        message,
        commit,
        nix_generation: None,
    })
}

/// Same as [`record`], but also records the Nix profile generation active
/// right after the action — the nix/hnm equivalent of
/// [`record_with_commit`]'s Flatpak commit, and just as necessary: without
/// it, `rollback_entry`'s `"nix"` branch has no generation number to roll
/// back to.
pub fn record_with_generation(
    action: &str,
    source: &str,
    name: &str,
    package_id: &str,
    version: Option<String>,
    nix_generation: Option<u32>,
    success: bool,
    message: Option<String>,
) -> Result<(), String> {
    push_entry(HistoryEntry {
        id: new_id(),
        timestamp: now_iso(),
        action: action.to_string(),
        source: source.to_string(),
        name: name.to_string(),
        package_id: package_id.to_string(),
        version,
        packages: None,
        success,
        message,
        commit: None,
        nix_generation,
    })
}

fn push_entry(entry: HistoryEntry) -> Result<(), String> {
    let mut entries = load();
    entries.push(entry);
    if entries.len() > MAX_HISTORY_ENTRIES {
        let drop = entries.len() - MAX_HISTORY_ENTRIES;
        entries.drain(0..drop);
    }
    save(&entries)
}

/// Returns history, most recent first.
pub fn get_all() -> Vec<HistoryEntry> {
    let mut entries = load();
    entries.reverse();
    entries
}

pub fn clear() -> Result<(), String> {
    save(&[])
}

/// Best-effort apt version pin used by rollback. Only ever called after
/// `source == "apt"` and a validated package_id + version have been
/// confirmed by the caller.
async fn apt_downgrade_to(app: &tauri::AppHandle, package_id: &str, version: &str) -> Result<(), String> {
    let spec = format!("{package_id}={version}");
    crate::priv_run(app, &["apt-get", "install", "-y", "--allow-downgrades", &spec]).await
}

/// Attempts to roll a single history entry back to the version(s) it
/// recorded. Returns a human-readable result message on success, or a clear
/// error (including the "not supported for this source" case) on failure.
pub async fn rollback_entry(app: &tauri::AppHandle, entry_id: &str) -> Result<String, String> {
    let entries = load();
    let entry = entries
        .iter()
        .find(|e| e.id == entry_id)
        .ok_or_else(|| "History entry not found.".to_string())?
        .clone();

    if entry.source != "apt" {
        return match entry.source.as_str() {
            "hpm" => crate::hpm::rollback(app, &entry.package_id).await,
            "nix" => {
                let Some(generation) = entry.nix_generation else {
                    return Err(
                        "Rollback isn't available for this Nix entry — no generation number was \
                         recorded at install/remove time (it predates generation tracking). \
                         Open the Nix panel to roll back to a specific generation directly instead."
                            .to_string(),
                    );
                };
                let base_msg = crate::hnm::rollback(app, generation).await?;
                let msg = format!(
                    "{base_msg} Note: this reverts the *entire* Nix profile to right after '{}' — \
                     every nix/hnm package's state at that point, not just this one — the same way \
                     `hnm rollback` itself works (it has no per-package undo).",
                    entry.name
                );
                let _ = record_with_generation("rollback", "nix", &entry.name, &entry.package_id, None, Some(generation), true, Some(msg.clone()));
                Ok(msg)
            }
            "appimage" => crate::appimage::rollback(app, &entry.package_id).await,
            "flatpak" => {
                let Some(commit) = entry.commit.clone() else {
                    return Err(
                        "Rollback isn't available for this Flatpak entry — no commit was recorded \
                         at install time (it predates commit tracking, or the `flatpak info` lookup \
                         failed right after install). Re-install it once to enable rollback for \
                         future updates."
                            .to_string(),
                    );
                };
                crate::flatpak_rollback_to_commit(app, &entry.package_id, &commit).await?;
                let short = &commit[..commit.len().min(12)];
                let msg = format!("Rolled {} back to commit {short}.", entry.name);
                let _ = record_with_commit("rollback", "flatpak", &entry.name, &entry.package_id, None, Some(commit), true, Some(msg.clone()));
                Ok(msg)
            }
            other => Err(format!(
                "Rollback isn't supported for source '{other}' — no pinned version/revision is tracked for it."
            )),
        };
    }

    // Multi-package entry (drivers): roll back every recorded package,
    // stopping at the first failure so we can report exactly which
    // package couldn't be downgraded rather than leaving an ambiguous
    // partial state.
    if let Some(packages) = entry.packages.clone() {
        if packages.is_empty() {
            return Err("No package versions were recorded for this action, so it can't be rolled back.".to_string());
        }
        for pv in &packages {
            let pkg = validate_pkg_token(&pv.name)?;
            let version = validate_pkg_token(&pv.version)?;
            apt_downgrade_to(app, &pkg, &version).await.map_err(|e| {
                format!("Rollback stopped at '{}': {e}", pv.name)
            })?;
        }
        let msg = format!(
            "Rolled {} back to its previously recorded versions ({}).",
            entry.name,
            packages.iter().map(|p| format!("{}={}", p.name, p.version)).collect::<Vec<_>>().join(", "),
        );
        let _ = record_multi("rollback", "apt", &entry.name, &entry.package_id, None, Some(packages), true, Some(msg.clone()));
        return Ok(msg);
    }

    let Some(version) = entry.version.clone() else {
        return Err(
            "No version was recorded for this action, so it can't be rolled back.".to_string(),
        );
    };
    let package_id = validate_pkg_token(&entry.package_id)?;
    let version = validate_pkg_token(&version)?;
    apt_downgrade_to(app, &package_id, &version).await?;
    let msg = format!("Rolled {} back to version {}.", entry.name, version);
    let _ = record("rollback", "apt", &entry.name, &package_id, Some(version), true, Some(msg.clone()));
    Ok(msg)
}

/// Best-effort lookup of the currently-installed apt/dpkg version of a
/// package, used to populate `HistoryEntry.version` right after an
/// install/uninstall so a later rollback has something to target.
pub async fn current_apt_version(package_id: &str) -> Option<String> {
    let out = Command::new("dpkg-query")
        .args(["-W", "-f=${Version}", package_id])
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if v.is_empty() { None } else { Some(v) }
}

/// Same as [`current_apt_version`], but for several packages at once (used
/// by driver installs, which touch multiple apt packages). Packages that
/// aren't actually installed (e.g. a virtual/meta package with no direct
/// dpkg entry) are silently omitted rather than failing the whole lookup.
pub async fn current_apt_versions(package_ids: &[&str]) -> Vec<PackageVersion> {
    let mut out = Vec::new();
    for &pkg in package_ids {
        if let Some(version) = current_apt_version(pkg).await {
            out.push(PackageVersion { name: pkg.to_string(), version });
        }
    }
    out
}
