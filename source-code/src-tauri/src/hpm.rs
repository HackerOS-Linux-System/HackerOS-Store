use std::process::Stdio;
use tokio::process::Command;

use crate::security::validate_pkg_token;
use crate::DiscoverResult;

/// Prefers the well-known system path (`/usr/bin/hpm`, matching how it's
/// installed on HackerOS), falling back to whatever `hpm` resolves to on
/// `$PATH` for anyone running the Store on a dev machine / non-standard
/// install. Cached after the first successful resolution for the life of
/// the process.
async fn hpm_bin() -> Option<&'static str> {
    static RESOLVED: tokio::sync::OnceCell<Option<String>> = tokio::sync::OnceCell::const_new();
    let resolved = RESOLVED.get_or_init(|| async {
        if std::path::Path::new("/usr/bin/hpm").is_file() {
            return Some("/usr/bin/hpm".to_string());
        }
        let ok = Command::new("which").arg("hpm").output().await
            .map(|o| o.status.success()).unwrap_or(false);
        if ok { Some("hpm".to_string()) } else { None }
    }).await;
    resolved.as_deref()
}

pub async fn is_available() -> bool { hpm_bin().await.is_some() }

async fn run_timeout(mut cmd: Command, secs: u64) -> Option<std::process::Output> {
    cmd.stdin(Stdio::null());
    tokio::time::timeout(std::time::Duration::from_secs(secs), cmd.output()).await.ok()?.ok()
}

// ─── search ─────────────────────────────────────────────────────────────────

/// Parses one row of `hpm search`'s results table:
/// `"  {:<22} {:<12} {:<32} {}"` -> name, version, description, tags.
/// Tags are `@word` tokens and are always last, so they're stripped off
/// the tail of the line first; whatever's left after the name/version
/// tokens is the description (which may itself have been re-wrapped by
/// terminal width in rare cases — best-effort).
fn parse_search_row(line: &str) -> Option<DiscoverResult> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() { return None; }
    let mut tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.len() < 2 { return None; }

    let mut tags = vec![];
    while let Some(last) = tokens.last() {
        if last.starts_with('@') { tags.push(*last); tokens.pop(); } else { break; }
    }

    let name = tokens.first()?.to_string();
    if name.is_empty() || name == "Package" { return None; } // header row
    let version = tokens.get(1).copied().unwrap_or("").to_string();
    let mut desc = tokens.get(2..).map(|t| t.join(" ")).unwrap_or_default();
    if desc.ends_with('…') { desc.pop(); }
    if !tags.is_empty() {
        if !desc.is_empty() { desc.push(' '); }
        desc.push_str(&tags.join(" "));
    }

    Some(DiscoverResult {
        name: name.clone(),
        version,
        desc,
        source: "hpm".into(),
        package_id: name,
        size: None,
        icon: None,
    })
}

pub async fn search(query: String) -> Result<Vec<DiscoverResult>, crate::SourceIssue> {
    let issue = |kind: &str, message: String| crate::SourceIssue { source: "hpm".into(), kind: kind.into(), message };
    let Some(bin) = hpm_bin().await else {
        return Err(issue("unavailable", "hpm is not installed on this system (expected /usr/bin/hpm).".into()));
    };
    let mut cmd = Command::new(bin);
    cmd.args(["search", &query]);
    // First-ever run can involve `hpm refresh`-equivalent index fetching
    // (git clones of every package's metadata repo) under the hood, so
    // this gets a much longer timeout than the local apt/flatpak/snap
    // lookups — it's reported as a "timeout" issue rather than silently
    // returning nothing either way (see `run_all_sources` in lib.rs).
    let Some(out) = run_timeout(cmd, 15).await else {
        return Err(issue("timeout", "hpm did not respond within 15s (first-run index fetch can be slow).".into()));
    };
    let text = String::from_utf8_lossy(&out.stdout);
    if text.contains("No results for") || text.contains("No packages found") { return Ok(vec![]); }

    let mut results = vec![];
    let mut in_table = false;
    for line in text.lines() {
        if line.trim_start().starts_with('─') { in_table = true; continue; }
        if !in_table { continue; }
        if line.trim().is_empty() { break; }
        if line.contains("Run ") || line.contains("Search by tag") || line.starts_with("Page ") { break; }
        if let Some(r) = parse_search_row(line) {
            results.push(r);
            if results.len() >= 12 { break; }
        }
    }
    Ok(results)
}

// ─── info ───────────────────────────────────────────────────────────────────

pub struct HpmInfo {
    pub version: Option<String>,
    pub summary: String,
    pub description: String,
    pub license: Option<String>,
    pub authors: Option<String>,
    pub tags: Vec<String>,
    pub installed_version: Option<String>,
}

pub async fn info(package: &str) -> Option<HpmInfo> {
    let bin = hpm_bin().await?;
    let mut cmd = Command::new(bin);
    cmd.args(["info", package]);
    let out = run_timeout(cmd, 8).await?;
    let text = String::from_utf8_lossy(&out.stdout);

    let mut d = HpmInfo {
        version: None, summary: String::new(), description: String::new(),
        license: None, authors: None, tags: vec![], installed_version: None,
    };
    let mut in_desc = false;
    let mut desc_lines = vec![];
    for raw in text.lines() {
        let line = raw.trim_start();
        if let Some(v) = strip_label(line, "Version:") { d.version = Some(v); in_desc = false; continue; }
        if let Some(v) = strip_label(line, "Author:") { d.authors = Some(v); in_desc = false; continue; }
        if let Some(v) = strip_label(line, "License:") { d.license = Some(v); in_desc = false; continue; }
        if let Some(v) = strip_label(line, "Tags:") {
            d.tags = v.split_whitespace().map(|s| s.trim_start_matches('@').to_string()).collect();
            in_desc = false; continue;
        }
        if let Some(v) = strip_label(line, "Installed:") {
            let v = v.trim();
            if v != "No" { d.installed_version = v.split_whitespace().next().map(|s| s.to_string()); }
            in_desc = false; continue;
        }
        if line == "Description:" { in_desc = true; continue; }
        if in_desc {
            if line.is_empty() { in_desc = false; continue; }
            desc_lines.push(line.to_string());
        }
    }
    d.description = desc_lines.join(" ");
    d.summary = d.description.chars().take(160).collect();
    Some(d)
}

fn strip_label<'a>(line: &'a str, label: &str) -> Option<String> {
    line.strip_prefix(label).map(|rest| rest.trim().to_string())
}

// ─── list / installed set ──────────────────────────────────────────────────

/// Parses `hpm list`'s `"  {:<20} {:<15} {:<8} {} {}"` rows into a plain
/// set of installed package names (the Discover "is this installed?"
/// check only needs the name, not version/pinned/tags).
pub async fn installed_names() -> Vec<String> {
    let Some(bin) = hpm_bin().await else { return vec![]; };
    let mut cmd = Command::new(bin);
    cmd.arg("list");
    let Some(out) = run_timeout(cmd, 6).await else { return vec![]; };
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().skip(2) // "→ Installed packages:" + column header
        .filter_map(|l| {
            let t = l.trim_start();
            if t.is_empty() || t.starts_with("Package") { return None; }
            t.split_whitespace().next().map(|s| s.to_string())
        }).collect()
}

// ─── mutating actions ───────────────────────────────────────────────────────

pub async fn install(app: &tauri::AppHandle, spec: &str) -> Result<(), String> {
    let spec = validate_pkg_token(spec)?;
    let Some(bin) = hpm_bin().await else {
        return Err("hpm is not installed on this system (expected /usr/bin/hpm).".to_string());
    };
    crate::emit_log(app, "info", &format!("Installing {spec} via hpm (HackerOS Community Repository)..."));
    crate::emit_prog(app, "install", &format!("Installing {spec}..."), 0.2);
    crate::run_streaming(app, &[bin, "install", spec.as_str()]).await?;
    crate::emit_prog(app, "done", "Done!", 1.0);
    Ok(())
}

pub async fn remove(app: &tauri::AppHandle, spec: &str) -> Result<(), String> {
    let spec = validate_pkg_token(spec)?;
    let Some(bin) = hpm_bin().await else {
        return Err("hpm is not installed on this system (expected /usr/bin/hpm).".to_string());
    };
    crate::emit_log(app, "info", &format!("Removing {spec} via hpm..."));
    crate::emit_prog(app, "uninstall", &format!("Removing {spec}..."), 0.3);
    crate::run_streaming(app, &[bin, "remove", spec.as_str()]).await?;
    crate::emit_prog(app, "done", "Removed.", 1.0);
    Ok(())
}

/// `hpm` has its own native rollback (`hpm rollback <pkg>`, restoring the
/// package-state snapshot taken right before the most recent install/
/// remove of it) — no need to re-implement version pinning ourselves the
/// way the apt-backed rollback in `history.rs` has to.
pub async fn rollback(app: &tauri::AppHandle, package: &str) -> Result<String, String> {
    let package = validate_pkg_token(package)?;
    let Some(bin) = hpm_bin().await else {
        return Err("hpm is not installed on this system (expected /usr/bin/hpm).".to_string());
    };
    crate::emit_log(app, "info", &format!("Rolling back {package} via hpm..."));
    crate::run_streaming(app, &[bin, "rollback", package.as_str()]).await?;
    Ok(format!("Rolled {package} back to its previous state via hpm."))
}
