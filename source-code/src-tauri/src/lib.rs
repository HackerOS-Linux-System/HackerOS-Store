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
/// against the enabled package sources (apt/flatpak/snap/brew) — there is no
/// static catalog backing this type any more.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscoverResult {
    pub name: String,
    pub version: String,
    pub desc: String,
    pub source: String,        // "apt" | "flatpak" | "snap" | "brew"
    pub package_id: String,    // id used to install/uninstall/query details
    pub size: Option<String>,
    pub icon: Option<String>,  // "data:image/png;base64,..." | None (frontend falls back to a source badge icon)
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
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct InstalledSets {
    pub apt: Vec<String>,
    pub flatpak: Vec<String>,
    pub snap: Vec<String>,
    pub brew: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    /// UI language. Supported: "en", "pl". Frontend owns the actual string
    /// tables; the backend only persists the chosen code.
    #[serde(default = "default_language")]
    pub language: String,
    /// Flatpak remote URL used when (re-)adding the Flathub remote.
    /// Lets the user switch to a regional mirror if Flathub is slow/blocked.
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
    /// ["apt","flatpak","snap","brew"]. Lets a person turn off a source
    /// they don't have installed (or don't trust) instead of always
    /// paying the query cost / seeing errors for it.
    /// `#[serde(default = ...)]` here (and on the fields below) means a
    /// settings.json written by an older version of this app — before these
    /// fields existed — still deserializes successfully instead of falling
    /// back to full factory defaults and silently discarding the person's
    /// saved mirror/language/etc. preferences.
    #[serde(default = "default_sources")]
    pub enabled_sources: Vec<String>,
    /// Whether to fetch community star ratings from the GNOME ODRS service
    /// for Flatpak apps in the detail view. Off by default for anyone who
    /// doesn't want the app phoning home at all.
    #[serde(default = "default_true")]
    pub ratings_enabled: bool,
    /// Which section the app opens on when launched.
    #[serde(default = "default_section")]
    pub default_section: String,
}

fn default_language() -> String { "en".into() }
fn default_flatpak_remote() -> String { "https://dl.flathub.org/repo/flathub.flatpakrepo".into() }
fn default_true() -> bool { true }
fn default_sources() -> Vec<String> { vec!["apt".into(), "flatpak".into(), "snap".into(), "brew".into()] }
fn default_section() -> String { "discover".into() }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            flatpak_remote_url: "https://dl.flathub.org/repo/flathub.flatpakrepo".into(),
            apt_mirror: String::new(),
            check_updates_on_startup: true,
            enabled_sources: vec!["apt".into(), "flatpak".into(), "snap".into(), "brew".into()],
            ratings_enabled: true,
            default_section: "discover".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppInfo {
    pub version: String,
    pub name: String,
    pub target_release: String,
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
// Every category click or search keystroke used to re-run 1-2 rounds of
// apt/flatpak/snap/brew subprocess calls from scratch, even for a category
// the person just looked at 5 seconds ago. A short-TTL in-memory cache
// makes flipping back to a recently-viewed category or re-typing a recent
// search near-instant, without risking showing very stale data (entries
// expire after CACHE_TTL regardless).
const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(120);

#[derive(Default)]
pub struct DiscoverCacheState {
    pub entries: AsyncMutex<std::collections::HashMap<String, (std::time::Instant, Vec<DiscoverResult>)>>,
}

async fn cache_get(app: &tauri::AppHandle, key: &str) -> Option<Vec<DiscoverResult>> {
    let state = app.state::<DiscoverCacheState>();
    let map = state.entries.lock().await;
    map.get(key).and_then(|(t, v)| if t.elapsed() < CACHE_TTL { Some(v.clone()) } else { None })
}

async fn cache_set(app: &tauri::AppHandle, key: String, val: Vec<DiscoverResult>) {
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
    check_cancel(app)?;

    let mut cmd = Command::new(argv[0]);
    for a in &argv[1..] { cmd.arg(a); }
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

async fn apt_install(app: &tauri::AppHandle, pkgs: &[&str]) -> Result<(), String> {
    let mut args = vec!["apt-get", "install", "-y", "--no-install-recommends"];
    args.extend_from_slice(pkgs);
    priv_run(app, &args).await
}

async fn apt_remove(app: &tauri::AppHandle, pkgs: &[&str]) -> Result<(), String> {
    let mut args = vec!["apt-get", "remove", "-y"];
    args.extend_from_slice(pkgs);
    priv_run(app, &args).await
}

// ─── Flatpak ──────────────────────────────────────────────────────────────────

async fn ensure_flatpak(app: &tauri::AppHandle) -> Result<(), String> {
    let has = Command::new("which").arg("flatpak").output().await
        .map(|o| o.status.success()).unwrap_or(false);
    if !has {
        emit_log(app, "info", "Installing Flatpak...");
        apt_install(app, &["flatpak"]).await?;
    }
    let remote = current_settings().flatpak_remote_url;
    let _ = run_sh(app, &format!("flatpak remote-add --if-not-exists --user flathub '{remote}' 2>/dev/null")).await;
    let _ = run_sh(app, &format!("sudo flatpak remote-add --if-not-exists flathub '{remote}' 2>/dev/null")).await;
    Ok(())
}

async fn flatpak_install(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    ensure_flatpak(app).await?;
    check_cancel(app)?;
    emit_log(app, "info", &format!("Installing {} from Flathub...", id));
    emit_prog(app, "install", &format!("Installing {}...", id), 0.3);
    if run_sh(app, &format!("flatpak install -y --user flathub '{id}'")).await.is_err() {
        check_cancel(app)?;
        run_sh(app, &format!("sudo flatpak install -y flathub '{id}'")).await?;
    }
    emit_prog(app, "done", "Done!", 1.0);
    emit_log(app, "success", "Installation complete.");
    Ok(())
}

async fn flatpak_uninstall(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    emit_log(app, "info", &format!("Removing {}...", id));
    emit_prog(app, "uninstall", &format!("Removing {}...", id), 0.3);
    if run_sh(app, &format!("flatpak uninstall -y --user '{id}'")).await.is_err() {
        check_cancel(app)?;
        run_sh(app, &format!("sudo flatpak uninstall -y '{id}'")).await?;
    }
    emit_prog(app, "done", "Removed.", 1.0);
    emit_log(app, "success", "Removed successfully.");
    Ok(())
}

async fn flatpak_remote_info(id: &str) -> serde_json::Value {
    let mut info = serde_json::json!({"size":null,"version":null});
    if id.is_empty() { return info; }
    let mut cmd = Command::new("flatpak");
    cmd.args(["remote-info","--user","flathub",id]);
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

// ─── Wine ─────────────────────────────────────────────────────────────────────

async fn ensure_wine(app: &tauri::AppHandle) -> Result<(), String> {
    let has = Command::new("which").arg("wine").output().await
        .map(|o| o.status.success()).unwrap_or(false);
    if !has {
        emit_log(app, "info", "Wine not found — installing...");
        priv_run(app, &["dpkg", "--add-architecture", "i386"]).await?;
        priv_run(app, &["apt-get", "update", "-qq"]).await?;
        apt_install(app, &["wine", "wine32", "wine64", "winetricks", "libgl1"]).await?;
        return Ok(());
    }
    let wine32_ok = Command::new("dpkg-query")
        .args(["-W", "-f=${Status}", "wine32"])
        .output().await
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("install ok"))
        .unwrap_or(false);
    if !wine32_ok {
        emit_log(app, "info", "wine32 not found — installing...");
        priv_run(app, &["dpkg", "--add-architecture", "i386"]).await?;
        priv_run(app, &["apt-get", "update", "-qq"]).await?;
        apt_install(app, &["wine32", "libgl1"]).await
            .map_err(|_| "Failed to install wine32. Please run manually:\n  sudo dpkg --add-architecture i386\n  sudo apt-get update\n  sudo apt-get install wine32".to_string())?;
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
    let line = format!("deb http://{host}/debian {cn} main contrib non-free non-free-firmware");
    emit_log(app, "info", &format!("Adding non-free repositories for '{cn}'..."));
    run_sh(app, &format!("echo '{line}' | sudo tee /etc/apt/sources.list.d/hackeros-nonfree.list > /dev/null")).await?;
    priv_run(app, &["apt-get", "update", "-qq"]).await?;
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
    run_sh(app, "distrobox create --image kalilinux/kali-rolling --name kali-pentest --yes").await?;
    let _ = run_sh(app, "distrobox enter kali-pentest -- sudo apt-get update -qq").await;
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
    // ── Wireless ──
    ("aircrack-ng",   true),
    ("kismet",        true),
    ("reaver",        true),
    ("wifite",        true),
    ("cowpatty",      true),
    ("pixiewps",      true),
    ("hcxdumptool",   true),
    ("hcxtools",      true),
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
    // ── Exploitation / Windows / AD ──
    ("metasploit",    false),
    ("impacket",      true),
    ("crackmapexec",  false),
    ("evil-winrm",    false),
    ("bloodhound",    false),
    ("enum4linux",    true),
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
    // ── Tunneling / proxy ──
    ("proxychains",   true),
    ("tor",           true),
    ("chisel",        false),
    ("stunnel",       true),
    // ── Vulnerability scanning ──
    ("sslscan",       true),
    ("openvas",       false),
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

    // apt/dpkg: one call for all
    let dpkg_text = Command::new("dpkg-query")
        .args(["-W", "-f=${Package} ${Version}\n"])
        .output().await
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let mut apt: std::collections::HashMap<String,String> = std::collections::HashMap::new();
    for line in dpkg_text.lines() {
        let mut p = line.splitn(2, ' ');
        let pkg = p.next().unwrap_or("").to_string();
        let ver = p.next().unwrap_or("").trim().to_string();
        apt.insert(pkg, ver);
    }

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
    out
}

// ─── install_package ─────────────────────────────────────────────────────────

#[tauri::command]
async fn install_package(app: tauri::AppHandle, name: String, category: String) -> Result<String, String> {
    reset_job(&app);
    emit_log(&app, "info", &format!("Starting installation of {}...", name));
    let result = match category.as_str() {
        "game_launchers" => install_launcher(&app, &name).await,
        "pentest_tools"  => install_pentest(&app, &name).await,
        "drivers"        => install_driver(&app, &name).await,
        _ => Err(format!("Unknown category: {category}")),
    };
    reset_job(&app);
    result?;
    emit_log(&app, "success", &format!("{} installed successfully.", name));
    Ok(format!("{name} installed successfully."))
}

// ─── uninstall_package ────────────────────────────────────────────────────────

#[tauri::command]
async fn uninstall_package(app: tauri::AppHandle, name: String, category: String) -> Result<String, String> {
    reset_job(&app);
    emit_log(&app, "info", &format!("Removing {}...", name));
    let result = match category.as_str() {
        "game_launchers" => uninstall_launcher(&app, &name).await,
        "pentest_tools"  => uninstall_pentest(&app, &name).await,
        "drivers"        => uninstall_driver(&app, &name).await,
        _ => Err(format!("Unknown category: {category}")),
    };
    reset_job(&app);
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
    run_sh(app, &format!("wget -q --show-progress -O '{installer}' '{url}' 2>&1")).await?;

    check_cancel(app)?;
    verify_download(app, id, &installer).await?;

    emit_log(app, "info", "Initialising Wine prefix (win32)...");
    emit_prog(app, "wine", "Initialising Wine prefix...", 0.40);
    run_sh(app, &format!(
        "WINEPREFIX='{prefix}' WINEARCH=win32 WINEDEBUG=-all wineboot --init 2>&1"
    )).await?;

    check_cancel(app)?;
    emit_log(app, "info", &format!("Running {} installer via Wine...", name));
    emit_prog(app, "wine", &format!("Installing {}...", name), 0.65);
    run_sh(app, &format!(
        "WINEPREFIX='{prefix}' WINEARCH=win32 WINEDEBUG=-all wine '{installer}' /S 2>&1"
    )).await?;

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
        run_sh(app, &format!("distrobox enter kali-pentest -- sudo apt-get install -y {name}")).await?;
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
    if in_debian(name) {
        let pkg = apt_pkg_name(name);
        emit_log(app, "info", &format!("Removing {} (apt)...", name));
        emit_prog(app, "uninstall", &format!("Removing {}...", name), 0.3);
        apt_remove(app, &[pkg.as_str()]).await?;
    } else {
        emit_log(app, "info", &format!("Removing {} from Kali container...", name));
        emit_prog(app, "uninstall", &format!("Removing {}...", name), 0.3);
        if kali_exists().await {
            let _ = run_sh(app, &format!("distrobox enter kali-pentest -- sudo apt-get remove -y {name}")).await;
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        let _ = std::fs::remove_file(format!("{home}/.local/bin/{name}"));
        let _ = std::fs::remove_file(format!("{home}/.local/share/applications/{name}.desktop"));
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
    if let Ok(out) = Command::new("sh").arg("-c")
        .arg("apt list --upgradable 2>/dev/null | grep -c upgradable")
        .output().await {
        String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0)
    } else { 0 }
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

async fn search_apt(query: String) -> Vec<DiscoverResult> {
    let mut cmd = Command::new("apt-cache");
    cmd.args(["search", "--names-only", &query]);
    let Some(out) = run_timeout(cmd, 4).await else { return vec![]; };
    let items: Vec<(String,String)> = String::from_utf8_lossy(&out.stdout)
        .lines().take(14).filter_map(|l| {
            let mut p = l.splitn(2, " - ");
            let n = p.next()?.trim().to_string();
            let d = p.next().unwrap_or("").trim().to_string();
            if n.is_empty() { return None; }
            Some((n, d))
        }).collect();
    if items.is_empty() { return vec![]; }
    let names: Vec<&str> = items.iter().map(|(n,_)| n.as_str()).collect();
    let mut dpkg_cmd = Command::new("dpkg-query");
    dpkg_cmd.arg("-W").arg("-f=${Package} ${Version}\n").args(&names);
    let dpkg = run_timeout(dpkg_cmd, 3).await
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let mut vm: std::collections::HashMap<String,String> = std::collections::HashMap::new();
    for line in dpkg.lines() {
        let mut p = line.splitn(2,' ');
        let pkg=p.next().unwrap_or("").to_string();
        let ver=p.next().unwrap_or("").trim().to_string();
        vm.insert(pkg,ver);
    }
    items.into_iter().map(|(name,desc)| {
        let ver = vm.get(&name).cloned().unwrap_or_default();
        DiscoverResult { name:name.clone(), version:ver, desc, source:"apt".into(), package_id:name, size:None, icon:None }
    }).collect()
}

async fn search_flatpak(query: String) -> Vec<DiscoverResult> {
    let mut cmd = Command::new("flatpak");
    cmd.args(["search","--columns=name,description,application,version",&query]);
    let Some(out) = run_timeout(cmd, 5).await else { return vec![]; };
    String::from_utf8_lossy(&out.stdout).lines()
        .filter(|l| !l.starts_with("Name")).take(10).filter_map(|line| {
            let c: Vec<&str> = line.split('\t').collect();
            if c.len() < 3 { return None; }
            let name=c[0].trim().to_string(); if name.is_empty() { return None; }
            let desc=c[1].trim().to_string();
            let id  =c[2].trim().to_string();
            let ver =c.get(3).map(|s|s.trim().to_string()).unwrap_or_default();
            Some(DiscoverResult { name, version:ver, desc, source:"flatpak".into(), package_id:id, size:None, icon:None })
        }).collect()
}

async fn search_snap(query: String) -> Vec<DiscoverResult> {
    let mut cmd = Command::new("snap");
    cmd.args(["find",&query]);
    // `snap find` calls out to the Snap Store over the network — the
    // likeliest source of multi-second (or worse) latency, so it gets the
    // most generous timeout but is still bounded.
    let Some(out) = run_timeout(cmd, 6).await else { return vec![]; };
    String::from_utf8_lossy(&out.stdout).lines().skip(1).take(8).filter_map(|line| {
        let c: Vec<&str> = line.split_whitespace().collect();
        if c.is_empty() { return None; }
        let name=c[0].to_string();
        let ver =c.get(1).map(|s|s.to_string()).unwrap_or_default();
        let desc=c.get(3..).map(|s|s.join(" ")).unwrap_or_default();
        Some(DiscoverResult { name:name.clone(), version:ver, desc, source:"snap".into(), package_id:name, size:None, icon:None })
    }).collect()
}

async fn search_brew(query: String) -> Vec<DiscoverResult> {
    let mut cmd = Command::new("brew");
    cmd.args(["search",&query]);
    let Some(out) = run_timeout(cmd, 6).await else { return vec![]; };
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
    res
}

async fn run_all_sources(query: String, settings: &AppSettings) -> Vec<DiscoverResult> {
    let want = |s: &str| settings.enabled_sources.iter().any(|x| x == s);
    let (apt, fp, snap, brew) = tokio::join!(
        async { if want("apt")     { search_apt(query.clone()).await }     else { vec![] } },
        async { if want("flatpak") { search_flatpak(query.clone()).await } else { vec![] } },
        async { if want("snap")    { search_snap(query.clone()).await }    else { vec![] } },
        async { if want("brew")    { search_brew(query.clone()).await }    else { vec![] } },
    );
    apt.into_iter().chain(fp).chain(snap).chain(brew).collect()
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
        let path = match r.source.as_str() {
            "flatpak" => fp_idx.get(&r.package_id).cloned(),
            "apt"     => apt_idx.get(&r.package_id).cloned(),
            _ => None,
        };
        set.spawn(async move {
            let icon = match path { Some(p) => b64_file(&p).await, None => None };
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
async fn discover_browse(app: tauri::AppHandle, category_id: String) -> Vec<DiscoverResult> {
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
        async { match kw0 { Some(k) => run_all_sources(k, &settings).await, None => vec![] } },
        async { match kw1 { Some(k) => run_all_sources(k, &settings).await, None => vec![] } },
    );
    let mut all = batch0;
    all.extend(batch1);
    let result = dedupe_and_enrich(all).await;
    cache_set(&app, cache_key, result.clone()).await;
    result
}

#[tauri::command]
async fn discover_search(app: tauri::AppHandle, query: String) -> Vec<DiscoverResult> {
    let settings = current_settings();
    let cache_key = format!("search:{}:{}", query.to_lowercase(), settings.enabled_sources.join(","));
    if let Some(cached) = cache_get(&app, &cache_key).await { return cached; }
    let all = run_all_sources(query, &settings).await;
    let result = dedupe_and_enrich(all).await;
    cache_set(&app, cache_key, result.clone()).await;
    result
}

#[tauri::command]
async fn discover_install(app: tauri::AppHandle, package_id: String, source: String) -> Result<String, String> {
    reset_job(&app);
    emit_log(&app, "info", &format!("Installing {} via {}...", package_id, source));
    emit_prog(&app, "install", &format!("Installing {}...", package_id), 0.2);
    let result: Result<(), String> = async {
        match source.as_str() {
            "apt"     => apt_install(&app, &[package_id.as_str()]).await,
            "flatpak" => {
                ensure_flatpak(&app).await?;
                if run_sh(&app, &format!("flatpak install -y --user flathub '{package_id}'")).await.is_err() {
                    run_sh(&app, &format!("sudo flatpak install -y flathub '{package_id}'")).await?;
                }
                Ok(())
            },
            "snap" => priv_run(&app, &["snap","install",&package_id]).await,
            "brew" => run_sh(&app, &format!("brew install '{package_id}'")).await,
            _ => Err(format!("Unknown source: {source}")),
        }
    }.await;
    reset_job(&app);
    result?;
    emit_prog(&app, "done", "Done!", 1.0);
    emit_log(&app, "success", &format!("{package_id} installed."));
    Ok(format!("{package_id} installed."))
}

#[tauri::command]
async fn discover_uninstall(app: tauri::AppHandle, package_id: String, source: String) -> Result<String, String> {
    reset_job(&app);
    emit_log(&app, "info", &format!("Removing {} ({})...", package_id, source));
    emit_prog(&app, "uninstall", &format!("Removing {}...", package_id), 0.2);
    let result: Result<(), String> = async {
        match source.as_str() {
            "apt"     => apt_remove(&app, &[package_id.as_str()]).await,
            "flatpak" => {
                if run_sh(&app, &format!("flatpak uninstall -y --user '{package_id}'")).await.is_err() {
                    run_sh(&app, &format!("sudo flatpak uninstall -y '{package_id}'")).await?;
                }
                Ok(())
            },
            "snap" => priv_run(&app, &["snap","remove",&package_id]).await,
            "brew" => run_sh(&app, &format!("brew uninstall '{package_id}'")).await,
            _ => Err(format!("Unknown source: {source}")),
        }
    }.await;
    reset_job(&app);
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
    let mut apt_cmd = Command::new("dpkg-query"); apt_cmd.args(["-W", "-f=${Package}\n"]);
    let mut fp_cmd  = Command::new("flatpak");    fp_cmd.args(["list", "--columns=application"]);
    let mut snap_cmd = Command::new("snap");       snap_cmd.args(["list"]);
    let mut brew_cmd = Command::new("brew");       brew_cmd.args(["list", "--formula"]);
    let (apt_out, fp_out, snap_out, brew_out) = tokio::join!(
        run_timeout(apt_cmd, 4), run_timeout(fp_cmd, 4), run_timeout(snap_cmd, 4), run_timeout(brew_cmd, 4),
    );
    if let Some(out) = apt_out {
        sets.apt = String::from_utf8_lossy(&out.stdout).lines().map(|s| s.to_string()).collect();
    }
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

async fn apt_show_info(name: &str) -> serde_json::Value {
    let mut info = serde_json::json!({"size":null,"version":null});
    let mut cmd = Command::new("apt-cache");
    cmd.args(["show","--no-all-versions",name]);
    if let Some(out) = run_timeout(cmd, 4).await {
        let s = String::from_utf8_lossy(&out.stdout).to_string();
        for line in s.lines() {
            if line.starts_with("Version:") {
                info["version"]=serde_json::json!(line.trim_start_matches("Version:").trim());
            }
            if line.starts_with("Size:") || line.starts_with("Installed-Size:") {
                let kb: u64 = line.split_whitespace().last().unwrap_or("0").parse().unwrap_or(0);
                let sz = if kb>1024 { format!("{:.1} MB",kb as f64/1024.0) } else { format!("{} KB",kb) };
                info["size"]=serde_json::json!(sz);
            }
        }
    }
    info
}

/// Fetches community star ratings from the GNOME ODRS service — the same
/// service GNOME Software queries. Best-effort: wrapped in a short timeout
/// and returns `None` on any error (offline machine, service down, schema
/// mismatch), never fails the whole detail view. Gated behind
/// `settings.ratings_enabled` so it's opt-out for anyone who doesn't want
/// the app making outbound requests. Coverage is naturally limited to apps
/// that have an AppStream id ODRS recognises — in practice, Flatpak apps.
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
        categories: vec![], size: None, rating: None,
    };

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
                let mut cmd = Command::new("apt-cache");
                cmd.args(["show","--no-all-versions",&package_id]);
                if let Some(out) = run_timeout(cmd, 4).await {
                    let s = String::from_utf8_lossy(&out.stdout);
                    for line in s.lines() {
                        if let Some(rest) = line.strip_prefix("Description-en:").or_else(|| line.strip_prefix("Description:")) {
                            d.summary = rest.trim().to_string();
                            d.description = rest.trim().to_string();
                        }
                    }
                }
            }
        }
        "snap" => {
            let mut cmd = Command::new("snap");
            cmd.args(["info", &package_id]);
            if let Some(out) = run_timeout(cmd, 6).await {
                parse_snap_info(&mut d, &String::from_utf8_lossy(&out.stdout));
            }
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
    d
}

#[tauri::command]
async fn get_package_info(name: String, category: String) -> serde_json::Value {
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
            if in_debian(&name) { apt_show_info(&apt_pkg_name(&name)).await }
            else { serde_json::json!({"size": null, "version": null, "note": "Installed inside the Kali container — size not tracked by apt."}) }
        },
        "drivers" => apt_show_info(&name).await,
        _ => serde_json::json!({"size":null,"version":null}),
    }
}

// ─── Settings ─────────────────────────────────────────────────────────────────

fn settings_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    std::path::PathBuf::from(format!("{home}/.hackeros/store/settings.json"))
}

fn current_settings() -> AppSettings {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
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

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "HackerOS Store".to_string(),
        target_release: "Debian trixie (testing) and forks".to_string(),
    }
}

#[tauri::command]
async fn clear_cache(app: tauri::AppHandle) -> Result<String, String> {
    emit_log(&app, "info", "Clearing package manager caches...");
    emit_prog(&app, "cache", "Clearing caches...", 0.2);

    let _ = priv_run(&app, &["apt-get", "clean"]).await;
    emit_prog(&app, "cache", "Cleared apt cache...", 0.45);

    let _ = run_sh(&app, "flatpak uninstall -y --user --unused 2>/dev/null").await;
    let _ = run_sh(&app, "sudo flatpak uninstall -y --unused 2>/dev/null").await;
    emit_prog(&app, "cache", "Cleared unused Flatpak runtimes...", 0.7);

    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let _ = run_sh(&app, &format!("rm -f {home}/.hackeros/launchers/*/installer.exe 2>/dev/null")).await;
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
            get_app_info,
            clear_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
