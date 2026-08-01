use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::security::validate_pkg_token;
use crate::DiscoverResult;

const FEED_URL: &str = "https://appimage.github.io/feed.json";
const FEED_TTL_SECS: u64 = 86_400; // 24h — see module docs.
const GITHUB_API: &str = "https://api.github.com";

fn store_root() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    PathBuf::from(format!("{home}/.hackeros/store"))
}

fn appimages_dir() -> PathBuf { store_root().join("appimages") }
fn feed_cache_path() -> PathBuf { store_root().join("appimage_feed_cache.json") }
fn installed_path() -> PathBuf { store_root().join("appimage_installed.json") }

fn home_dir() -> String { std::env::var("HOME").unwrap_or_else(|_| "/root".into()) }
fn bin_dir() -> PathBuf { PathBuf::from(format!("{}/.local/bin", home_dir())) }
fn desktop_dir() -> PathBuf { PathBuf::from(format!("{}/.local/share/applications", home_dir())) }
fn icon_dir() -> PathBuf { PathBuf::from(format!("{}/.local/share/icons/hicolor/128x128/apps", home_dir())) }

/// "owner/repo" -> a filesystem-safe slug. `repo` has already been through
/// [`validate_pkg_token`] by every caller before this is used, so it can
/// only contain the allow-listed charset (alnum, `. - _ + : @ /`) — this
/// just additionally removes the `/` so it's safe as a single path
/// component (never re-introduces `..` or a fresh `/`).
fn slugify(repo: &str) -> String {
    repo.replace('/', "__")
}

// ─── Feed entry + cache ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppImageEntry {
    pub name: String,
    pub description: String,
    /// "owner/repo" — the only thing actually required to install.
    pub repo: String,
    pub homepage: Option<String>,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct FeedCache {
    fetched_at: u64,
    apps: Vec<AppImageEntry>,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn load_cache() -> Option<FeedCache> {
    let s = std::fs::read_to_string(feed_cache_path()).ok()?;
    serde_json::from_str(&s).ok()
}

fn save_cache(cache: &FeedCache) {
    if let Some(dir) = feed_cache_path().parent() { let _ = std::fs::create_dir_all(dir); }
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(feed_cache_path(), json);
    }
}

// ── Tolerant JSON walking (schema not verifiable offline — see module docs) ──

fn as_str(v: &serde_json::Value) -> Option<String> { v.as_str().map(|s| s.to_string()) }

fn find_str(v: &serde_json::Value, key_substr: &str) -> Option<String> {
    if let serde_json::Value::Object(map) = v {
        for (k, val) in map {
            if k.to_lowercase().contains(key_substr) {
                if let Some(s) = as_str(val) { return Some(s); }
            }
        }
        for (_, val) in map {
            if let Some(f) = find_str(val, key_substr) { return Some(f); }
        }
    }
    None
}

/// Looks for a GitHub repo reference anywhere in the entry: a dedicated
/// "github"/"repo" key holding either a bare `owner/repo` or a full
/// `https://github.com/owner/repo` URL, falling back to scanning every
/// string in the object for a github.com link if no such key exists.
fn find_github_repo(v: &serde_json::Value) -> Option<String> {
    for key in ["github", "repo", "repository"] {
        if let Some(s) = find_str(v, key) {
            if let Some(r) = extract_owner_repo(&s) { return Some(r); }
        }
    }
    let mut strings = vec![];
    collect_strings(v, &mut strings);
    strings.iter().find_map(|s| extract_owner_repo(s))
}

fn extract_owner_repo(s: &str) -> Option<String> {
    let cleaned = s.trim().trim_end_matches(".git").trim_end_matches('/');
    let rest = cleaned.strip_prefix("https://github.com/").or_else(|| cleaned.strip_prefix("http://github.com/"))?;
    let mut parts = rest.splitn(3, '/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() { return None; }
    Some(format!("{owner}/{repo}"))
}

fn collect_strings(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Object(map) => for (_, val) in map { collect_strings(val, out); },
        serde_json::Value::Array(arr) => for item in arr { collect_strings(item, out); },
        _ => {}
    }
}

fn find_icon_url(v: &serde_json::Value) -> Option<String> {
    for key in ["icon", "logo"] {
        if let Some(s) = find_str(v, key) {
            if s.starts_with("http") { return Some(s); }
        }
    }
    None
}

fn find_categories(v: &serde_json::Value) -> Vec<String> {
    if let serde_json::Value::Object(map) = v {
        for (k, val) in map {
            if k.to_lowercase().contains("categor") {
                let mut out = vec![];
                collect_strings(val, &mut out);
                return out;
            }
        }
    }
    vec![]
}

fn parse_feed(json: &serde_json::Value) -> Vec<AppImageEntry> {
    // The feed is expected to be a JSON array of app objects, but some
    // mirrors/exports wrap it in `{"items": [...]}` or `{"apps": [...]}` —
    // tolerate both.
    let arr = json.as_array().cloned().unwrap_or_else(|| {
        ["items", "apps", "data"].iter()
            .find_map(|k| json.get(k).and_then(|v| v.as_array()).cloned())
            .unwrap_or_default()
    });

    arr.iter().filter_map(|item| {
        let name = find_str(item, "name")?;
        let repo = find_github_repo(item)?;
        Some(AppImageEntry {
            name,
            description: find_str(item, "description").unwrap_or_default(),
            repo,
            homepage: find_str(item, "homepage").or_else(|| find_str(item, "website")),
            icon_url: find_icon_url(item),
            categories: find_categories(item),
        })
    }).collect()
}

fn http_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("hackeros-store")
        .build().ok()
}

/// Loads the feed from the local cache if it's fresh enough, otherwise
/// fetches and re-caches it. Best-effort: any network failure just falls
/// back to whatever's in the (possibly stale, possibly empty) cache rather
/// than failing the whole Discover request.
async fn ensure_feed(force_refresh: bool) -> Vec<AppImageEntry> {
    if !force_refresh {
        if let Some(cache) = load_cache() {
            if unix_now().saturating_sub(cache.fetched_at) < FEED_TTL_SECS {
                return cache.apps;
            }
        }
    }

    let Some(client) = http_client() else { return load_cache().map(|c| c.apps).unwrap_or_default(); };
    let fetched = async {
        let resp = client.get(FEED_URL).send().await.ok()?;
        if !resp.status().is_success() { return None; }
        let json: serde_json::Value = resp.json().await.ok()?;
        Some(parse_feed(&json))
    }.await;

    match fetched {
        Some(apps) if !apps.is_empty() => {
            save_cache(&FeedCache { fetched_at: unix_now(), apps: apps.clone() });
            apps
        }
        // Network reachable but parse/schema mismatch, or genuinely empty —
        // don't overwrite a possibly-good existing cache with nothing.
        _ => load_cache().map(|c| c.apps).unwrap_or_default(),
    }
}

/// Explicit cache refresh, exposed to the frontend (e.g. a "Refresh
/// catalog" button in Settings) so a person isn't stuck waiting up to 24h
/// for a newly-published AppImage to show up.
pub async fn refresh_feed() -> Result<usize, String> {
    let apps = ensure_feed(true).await;
    if apps.is_empty() {
        return Err("Could not reach the AppImageHub feed and no local cache exists yet.".to_string());
    }
    Ok(apps.len())
}

pub async fn search(query: String) -> Result<Vec<DiscoverResult>, crate::SourceIssue> {
    let q = query.to_lowercase();
    let apps = ensure_feed(false).await;
    if apps.is_empty() {
        return Err(crate::SourceIssue {
            source: "appimage".into(),
            kind: "unavailable".into(),
            message: "Could not reach the AppImageHub feed and no local cache exists yet.".into(),
        });
    }
    let mut set: tokio::task::JoinSet<DiscoverResult> = tokio::task::JoinSet::new();
    let mut n = 0;
    for entry in apps.into_iter() {
        if n >= 10 { break; }
        let hay = format!("{} {}", entry.name.to_lowercase(), entry.description.to_lowercase());
        if !hay.contains(&q) { continue; }
        n += 1;
        set.spawn(async move {
            let icon = match &entry.icon_url {
                Some(url) => fetch_icon_b64(url).await,
                None => None,
            };
            DiscoverResult {
                name: entry.name,
                version: String::new(),
                desc: entry.description,
                source: "appimage".into(),
                package_id: entry.repo,
                size: None,
                icon,
            }
        });
    }
    let mut out = vec![];
    while let Some(r) = set.join_next().await { if let Ok(r) = r { out.push(r); } }
    Ok(out)
}

async fn fetch_icon_b64(url: &str) -> Option<String> {
    let client = http_client()?;
    let resp = tokio::time::timeout(std::time::Duration::from_secs(5), client.get(url).send()).await.ok()?.ok()?;
    if !resp.status().is_success() { return None; }
    let bytes = resp.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > 2_000_000 { return None; }
    let mime = if url.ends_with(".svg") { "image/svg+xml" } else { "image/png" };
    Some(format!("data:{mime};base64,{}", B64.encode(&bytes)))
}

/// Looks a single repo up in the cached feed (used by `get_app_details`).
/// Falls back to a bare entry (repo only) if it isn't in the feed — a
/// person can still install by owner/repo even if AppImageHub hasn't
/// indexed it, they just won't get a description/icon from the feed.
pub async fn feed_entry(repo: &str) -> Option<AppImageEntry> {
    ensure_feed(false).await.into_iter().find(|e| e.repo.eq_ignore_ascii_case(repo))
}

// ─── GitHub Releases ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GhAsset { name: String, browser_download_url: String, size: Option<u64> }

#[derive(Debug, Deserialize)]
struct GhRelease { tag_name: String, #[serde(default)] assets: Vec<GhAsset> }

struct ReleaseAsset { tag: String, url: String, filename: String, size: Option<u64> }

async fn latest_appimage_asset(repo: &str) -> Result<ReleaseAsset, String> {
    let (owner, name) = repo.split_once('/').ok_or_else(|| "Invalid repo — expected 'owner/repo'.".to_string())?;
    let client = http_client().ok_or("Could not build HTTP client.")?;
    let url = format!("{GITHUB_API}/repos/{owner}/{name}/releases/latest");
    let resp = client.get(&url)
        .header("Accept", "application/vnd.github+json")
        .send().await
        .map_err(|e| format!("Failed to reach GitHub: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(format!("{repo} has no GitHub Releases — nothing to install."));
    }
    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {} for {repo}.", resp.status()));
    }
    let release: GhRelease = resp.json().await.map_err(|e| format!("Unexpected GitHub API response: {e}"))?;
    let asset = release.assets.iter()
        .find(|a| a.name.to_lowercase().ends_with(".appimage"))
        .ok_or_else(|| format!("The latest release of {repo} ({}) has no .AppImage asset attached.", release.tag_name))?;
    Ok(ReleaseAsset {
        tag: release.tag_name,
        url: asset.browser_download_url.clone(),
        filename: asset.name.clone(),
        size: asset.size,
    })
}

/// Latest tag available on GitHub vs. the installed tag, for update checks.
pub async fn check_update(repo: &str, installed_version: &str) -> Option<String> {
    let asset = latest_appimage_asset(repo).await.ok()?;
    if asset.tag != installed_version { Some(asset.tag) } else { None }
}

// ─── Installed-state store ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppImageInstalled {
    pub repo: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub previous_version: Option<String>,
    pub exec_path: String,
    pub desktop_path: String,
    pub icon_path: Option<String>,
}

fn load_installed() -> Vec<AppImageInstalled> {
    std::fs::read_to_string(installed_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_installed(list: &[AppImageInstalled]) -> Result<(), String> {
    if let Some(dir) = installed_path().parent() { std::fs::create_dir_all(dir).map_err(|e| e.to_string())?; }
    let json = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    std::fs::write(installed_path(), json).map_err(|e| e.to_string())
}

pub fn list_installed() -> Vec<AppImageInstalled> { load_installed() }

pub fn is_installed(repo: &str) -> Option<AppImageInstalled> {
    load_installed().into_iter().find(|e| e.repo.eq_ignore_ascii_case(repo))
}

// ─── Install / uninstall / rollback ────────────────────────────────────────

fn wrapper_path(name: &str) -> PathBuf { bin_dir().join(name) }
fn desktop_path(slug: &str) -> PathBuf { desktop_dir().join(format!("appimage-{slug}.desktop")) }

fn write_desktop(name: &str, exec: &std::path::Path, icon: Option<&str>) -> std::io::Result<PathBuf> {
    let slug = slugify(name);
    std::fs::create_dir_all(desktop_dir())?;
    let icon_line = icon.map(|i| format!("Icon={i}\n")).unwrap_or_default();
    let content = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec=\"{}\" %U\n{icon_line}Categories=Utility;\nTerminal=false\nX-AppImage-Source=appimagehub\n",
        exec.display()
    );
    let path = desktop_path(&slug);
    std::fs::write(&path, content)?;
    Ok(path)
}

/// Downloads the latest release's `.AppImage`, integrates it, and records
/// it in the installed store. `app`/`emit_*` come from `lib.rs` — kept as
/// plain callback closures so this module doesn't need to depend on
/// `tauri::AppHandle`'s full surface directly.
pub async fn install(
    app: &tauri::AppHandle,
    repo: &str,
    display_name: &str,
) -> Result<(), String> {
    let repo = validate_pkg_token(repo)?;
    let slug = slugify(&repo);
    let name_slug = validate_pkg_token(display_name).unwrap_or_else(|_| slug.clone());
    // Filesystem-safe wrapper/binary name: reuse the repo slug if the
    // display name doesn't survive validation cleanly (e.g. contains
    // spaces), so install never fails on the name alone.
    let bin_name = if name_slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        name_slug
    } else {
        slug.clone()
    };

    crate::emit_log(app, "info", &format!("Looking up latest release for {repo}..."));
    let asset = latest_appimage_asset(&repo).await?;
    crate::check_cancel(app)?;

    let version_dir = appimages_dir().join(&slug).join(&asset.tag);
    std::fs::create_dir_all(&version_dir).map_err(|e| format!("Failed to create {}: {e}", version_dir.display()))?;
    let target = version_dir.join(&asset.filename);

    crate::emit_log(app, "info", &format!("Downloading {} ({})...", asset.filename, asset.size.map(|s| format!("{:.1} MB", s as f64 / 1_048_576.0)).unwrap_or_default()));
    crate::emit_prog(app, "download", &format!("Downloading {}...", asset.filename), 0.2);
    let target_str = target.to_string_lossy().to_string();
    crate::run_streaming(app, &["wget", "-q", "-O", target_str.as_str(), asset.url.as_str()]).await?;
    crate::check_cancel(app)?;

    let _ = std::process::Command::new("chmod").args(["755", &target.to_string_lossy()]).output();

    crate::emit_log(app, "info", "Integrating with the desktop (wrapper + .desktop entry)...");
    crate::emit_prog(app, "install", "Setting up desktop integration...", 0.7);

    std::fs::create_dir_all(bin_dir()).map_err(|e| e.to_string())?;
    let wrapper = wrapper_path(&bin_name);
    let _ = std::fs::remove_file(&wrapper); // symlink_file fails if the target already exists
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &wrapper).map_err(|e| format!("Failed to link {}: {e}", wrapper.display()))?;

    let entry = feed_entry(&repo).await;
    let icon_path = match entry.as_ref().and_then(|e| e.icon_url.clone()) {
        Some(url) => download_icon_to_disk(&url, &slug).await,
        None => None,
    };
    let desktop = write_desktop(display_name, &wrapper, icon_path.as_deref())
        .map_err(|e| format!("Failed to write .desktop entry: {e}"))?;

    let mut list = load_installed();
    let previous_version = list.iter().find(|e| e.repo.eq_ignore_ascii_case(&repo)).map(|e| e.version.clone());
    list.retain(|e| !e.repo.eq_ignore_ascii_case(&repo));
    list.push(AppImageInstalled {
        repo: repo.clone(),
        name: display_name.to_string(),
        version: asset.tag.clone(),
        previous_version,
        exec_path: wrapper.to_string_lossy().to_string(),
        desktop_path: desktop.to_string_lossy().to_string(),
        icon_path,
    });
    save_installed(&list)?;

    crate::emit_prog(app, "done", "Done!", 1.0);
    crate::emit_log(app, "success", &format!("{display_name} {} installed.", asset.tag));
    Ok(())
}

async fn download_icon_to_disk(url: &str, slug: &str) -> Option<String> {
    let b64 = fetch_icon_b64(url).await?;
    let comma = b64.find(',')?;
    let data = &b64[comma + 1..];
    let bytes = B64.decode(data).ok()?;
    let _ = std::fs::create_dir_all(icon_dir());
    let path = icon_dir().join(format!("{slug}.png"));
    std::fs::write(&path, bytes).ok()?;
    Some(path.to_string_lossy().to_string())
}

/// Removes the wrapper/desktop/icon and the installed-store entry, but
/// deliberately leaves the downloaded version directories under
/// `store/appimages/<slug>/` on disk — matching `hpm`'s own store model —
/// so a later `install` of the same repo, or a rollback issued before the
/// next `clear_cache`, doesn't need to re-download anything.
pub async fn uninstall(app: &tauri::AppHandle, repo: &str) -> Result<(), String> {
    let repo = validate_pkg_token(repo)?;
    let mut list = load_installed();
    let Some(entry) = list.iter().find(|e| e.repo.eq_ignore_ascii_case(&repo)).cloned() else {
        return Err(format!("{repo} is not installed via AppImage."));
    };

    crate::emit_log(app, "info", &format!("Removing {}...", entry.name));
    crate::emit_prog(app, "uninstall", &format!("Removing {}...", entry.name), 0.4);

    let _ = std::fs::remove_file(&entry.exec_path);
    let _ = std::fs::remove_file(&entry.desktop_path);
    if let Some(icon) = &entry.icon_path { let _ = std::fs::remove_file(icon); }

    list.retain(|e| !e.repo.eq_ignore_ascii_case(&repo));
    save_installed(&list)?;

    crate::emit_prog(app, "done", "Removed.", 1.0);
    crate::emit_log(app, "success", &format!("{} removed.", entry.name));
    Ok(())
}

/// Re-points the wrapper/desktop entry at the previous version's stored
/// `.AppImage`, if that version's directory is still on disk (it wasn't
/// pruned). Never re-downloads — if the old version isn't cached locally
/// any more, this fails with a clear message rather than silently
/// installing the *latest* version under the guise of a rollback.
pub async fn rollback(app: &tauri::AppHandle, repo: &str) -> Result<String, String> {
    let repo = validate_pkg_token(repo)?;
    let mut list = load_installed();
    let idx = list.iter().position(|e| e.repo.eq_ignore_ascii_case(&repo))
        .ok_or_else(|| format!("{repo} is not installed via AppImage."))?;
    let Some(prev) = list[idx].previous_version.clone() else {
        return Err("No previous version is recorded for this AppImage — nothing to roll back to.".to_string());
    };

    let slug = slugify(&repo);
    let version_dir = appimages_dir().join(&slug).join(&prev);
    let appimage_file = std::fs::read_dir(&version_dir).ok()
        .and_then(|mut rd| rd.find_map(|e| e.ok()).map(|e| e.path()))
        .ok_or_else(|| format!(
            "Version {prev} of {repo} is no longer cached on disk under {} — can't roll back without re-downloading it.",
            version_dir.display()
        ))?;

    let bin_name = std::path::Path::new(&list[idx].exec_path).file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| slug.clone());
    let wrapper = wrapper_path(&bin_name);
    let _ = std::fs::remove_file(&wrapper);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&appimage_file, &wrapper).map_err(|e| format!("Failed to relink {}: {e}", wrapper.display()))?;

    let name = list[idx].name.clone();
    let current = list[idx].version.clone();
    list[idx].previous_version = Some(current);
    list[idx].version = prev.clone();
    save_installed(&list)?;

    let msg = format!("Rolled {name} back from {} to {prev}.", list[idx].previous_version.clone().unwrap_or_default());
    crate::emit_log(app, "success", &msg);
    Ok(msg)
}
