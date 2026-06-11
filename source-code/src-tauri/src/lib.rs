#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscoverResult {
    pub name: String,
    pub version: String,
    pub desc: String,
    pub source: String,
    pub package_id: String,
    pub size: Option<String>,
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
    let mut cmd = Command::new(argv[0]);
    for a in &argv[1..] { cmd.arg(a); }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);

    let mut child = cmd.spawn()
        .map_err(|e| format!("spawn '{}': {}", argv[0], e))?;

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

    let status = child.wait().await.map_err(|e| e.to_string())?;
    let _ = tokio::join!(t1, t2);

    if status.success() { Ok(()) }
    else { Err(format!("'{}' exited {}", argv[0], status.code().unwrap_or(-1))) }
}

async fn run_sh(app: &tauri::AppHandle, cmd: &str) -> Result<(), String> {
    run_streaming(app, &["sh", "-c", cmd]).await
}

async fn priv_run(app: &tauri::AppHandle, args: &[&str]) -> Result<(), String> {
    let has_pkexec = Command::new("which").arg("pkexec").output().await
        .map(|o| o.status.success()).unwrap_or(false);
    if has_pkexec {
        let mut full = vec!["pkexec"];
        full.extend_from_slice(args);
        if run_streaming(app, &full).await.is_ok() { return Ok(()); }
    }
    let mut full = vec!["sudo"];
    full.extend_from_slice(args);
    run_streaming(app, &full).await
}

async fn apt_install(app: &tauri::AppHandle, pkgs: &[&str]) -> Result<(), String> {
    let mut args = vec!["apt-get", "install", "-y", "--no-install-recommends"];
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
    let _ = run_sh(app, "flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo 2>/dev/null").await;
    let _ = run_sh(app, "sudo flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo 2>/dev/null").await;
    Ok(())
}

async fn flatpak_install(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    ensure_flatpak(app).await?;
    emit_log(app, "info", &format!("Installing {} from Flathub...", id));
    emit_prog(app, "install", &format!("Installing {}...", id), 0.3);
    if run_sh(app, &format!("flatpak install -y --user flathub '{id}'")).await.is_err() {
        run_sh(app, &format!("sudo flatpak install -y flathub '{id}'")).await?;
    }
    emit_prog(app, "done", "Done!", 1.0);
    emit_log(app, "success", "Installation complete.");
    Ok(())
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
    // Detect wine32
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
        .unwrap_or_else(|| "bookworm".into());
    let line = format!("deb http://deb.debian.org/debian {cn} main contrib non-free non-free-firmware");
    emit_log(app, "info", "Adding non-free repositories...");
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

async fn ensure_kali(app: &tauri::AppHandle) -> Result<(), String> {
    let list = Command::new("distrobox").arg("list").output().await
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    if list.contains("kali-pentest") { return Ok(()); }
    emit_log(app, "info", "Creating Kali Linux container (first run ~5 min)...");
    emit_prog(app, "install", "Creating Kali container...", 0.15);
    run_sh(app, "distrobox create --image kalilinux/kali-rolling --name kali-pentest --yes").await?;
    let _ = run_sh(app, "distrobox enter kali-pentest -- sudo apt-get update -qq").await;
    Ok(())
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
        ("store::Firefox",      "org.mozilla.firefox"),
        ("store::VSCode",       "com.visualstudio.code"),
        ("store::LibreOffice",  "org.libreoffice.LibreOffice"),
        ("store::Thunderbird",  "org.mozilla.Thunderbird"),
        ("store::GIMP",         "org.gimp.GIMP"),
        ("store::Inkscape",     "org.inkscape.Inkscape"),
        ("store::Blender",      "org.blender.Blender"),
        ("store::VLC",          "org.videolan.VLC"),
        ("store::Audacity",     "org.audacityteam.Audacity"),
        ("store::FileZilla",    "org.filezillaproject.Filezilla"),
        ("store::VirtualBox",   "org.virtualbox.VirtualBox"),
        ("store::Tor Browser",  "com.github.micahflee.torbrowser-launcher"),
        ("store::KeePassXC",    "org.keepassxc.KeePassXC"),
        ("store::Wireshark",    "org.wireshark.Wireshark"),
        ("store::Obsidian",     "md.obsidian.Obsidian"),
        ("store::Discord",      "com.discordapp.Discord"),
        ("store::Telegram",     "org.telegram.desktop"),
        ("store::Signal",       "org.signal.Signal"),
        ("store::Spotify",      "com.spotify.Client"),
        ("store::OBS Studio",   "com.obsproject.Studio"),
        ("store::Kdenlive",     "org.kde.kdenlive"),
        ("store::Handbrake",    "fr.handbrake.ghb"),
        ("store::Krita",        "org.kde.krita"),
        ("store::Darktable",    "org.darktable.Darktable"),
        ("store::Shotcut",      "org.shotcut.Shotcut"),
        ("store::Nextcloud",    "com.nextcloud.desktopclient"),
        ("store::Bitwarden",    "com.bitwarden.desktop"),
        ("store::Flatseal",     "com.github.tchx84.Flatseal"),
        ("store::LocalSend",    "org.localsend.localsend_app"),
        ("store::Vesktop",      "dev.vencord.Vesktop"),
        ("store::Zed",          "dev.zed.Zed"),
        ("store::Neovide",      "com.neovide.neovide"),
        ("store::Postman",      "com.getpostman.Postman"),
        ("store::DBeaver",      "io.dbeaver.DBeaverCommunity"),
        ("store::Insomnia",     "rest.insomnia.Insomnia"),
        ("store::Meld",         "org.gnome.meld"),
        ("store::FreeCAD",      "org.freecadweb.FreeCAD"),
        ("store::Prusa Slicer", "com.prusa3d.PrusaSlicer"),
        ("store::Calibre",      "com.calibre_ebook.calibre"),
        ("store::Remmina",      "org.remmina.Remmina"),
        ("store::Pika Backup",  "org.gnome.World.PikaBackup"),
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

    let apt_items: &[(&str, &str)] = &[
        ("pentest_tools::nmap",       "nmap"),
        ("pentest_tools::wireshark",  "wireshark"),
        ("pentest_tools::tcpdump",    "tcpdump"),
        ("pentest_tools::netcat",     "netcat"),
        ("pentest_tools::aircrack-ng","aircrack-ng"),
        ("pentest_tools::masscan",    "masscan"),
        ("pentest_tools::sqlmap",     "sqlmap"),
        ("pentest_tools::john",       "john"),
        ("pentest_tools::hydra",      "hydra"),
        ("pentest_tools::hashcat",    "hashcat"),
        ("pentest_tools::gobuster",   "gobuster"),
        ("pentest_tools::nikto",      "nikto"),
        ("pentest_tools::enum4linux", "enum4linux"),
        ("pentest_tools::volatility", "volatility"),
        ("pentest_tools::autopsy",    "autopsy"),
        ("pentest_tools::tcpflow",    "tcpflow"),
        ("pentest_tools::tshark",     "tshark"),
        ("pentest_tools::socat",      "socat"),
        ("pentest_tools::medusa",     "medusa"),
        ("pentest_tools::gdb",        "gdb"),
        ("pentest_tools::binwalk",    "binwalk"),
        ("pentest_tools::foremost",   "foremost"),
        ("pentest_tools::steghide",   "steghide"),
        ("pentest_tools::tor",        "tor"),
        ("pentest_tools::proxychains","proxychains"),
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
    emit_log(&app, "info", &format!("Starting installation of {}...", name));
    match category.as_str() {
        "game_launchers" => install_launcher(&app, &name).await?,
        "pentest_tools"  => install_pentest(&app, &name).await?,
        "store"          => install_store(&app, &name).await?,
        "drivers"        => install_driver(&app, &name).await?,
        _ => return Err(format!("Unknown category: {category}")),
    }
    emit_log(&app, "success", &format!("{} installed successfully.", name));
    Ok(format!("{name} installed successfully."))
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

async fn install_wine_launcher(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    ensure_wine(app).await?;
    let (id, url, exe) = match name {
        "GOG"        => ("gog",       "https://webinstallers.gog.com/galaxy_installer_en.exe",
                         "GOG Galaxy/GalaxyClient.exe"),
        "Battle.net" => ("battlenet", "https://www.battle.net/download/getInstaller?os=win&installer=Battle.net-Setup.exe",
                         "Battle.net/Battle.net.exe"),
        _            => ("ea",        "https://origin-a.akamaihd.net/EA-Desktop-Client-Download/installer-releases/EAappInstaller.exe",
                         "Electronic Arts/EA Desktop/EADesktop.exe"),
    };
    let home   = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let dir    = format!("{home}/.hackeros/launchers/{id}");
    let prefix = format!("{dir}/prefix");
    std::fs::create_dir_all(&prefix).ok();
    let installer = format!("{dir}/installer.exe");

    emit_log(app, "info", &format!("Downloading {} installer...", name));
    emit_prog(app, "download", &format!("Downloading {}...", name), 0.1);
    run_sh(app, &format!("wget -q --show-progress -O '{installer}' '{url}' 2>&1")).await?;

    emit_log(app, "info", "Initialising Wine prefix (win32)...");
    emit_prog(app, "wine", "Initialising Wine prefix...", 0.40);
    run_sh(app, &format!(
        "WINEPREFIX='{prefix}' WINEARCH=win32 WINEDEBUG=-all wineboot --init 2>&1"
    )).await?;

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

fn in_debian(name: &str) -> bool {
    matches!(name,
        "nmap"|"wireshark"|"tcpdump"|"netcat"|"aircrack-ng"|"masscan"|"arp-scan"|
        "sqlmap"|"nikto"|"john"|"hydra"|"hashcat"|"sslscan"|"dnsrecon"|"enum4linux"|
        "gobuster"|"recon-ng"|"volatility"|"autopsy"|"tcpflow"|"tshark"|"ncat"|
        "socat"|"netdiscover"|"hping3"|"medusa"|"binwalk"|"foremost"|"steghide"|
        "gdb"|"tor"|"proxychains"|"p0f"|"crunch"
    )
}

async fn install_pentest(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    if in_debian(name) {
        emit_log(app, "info", &format!("Installing {} from Debian repos...", name));
        emit_prog(app, "install", &format!("Installing {}...", name), 0.2);
        apt_install(app, &[name]).await?;
    } else {
        ensure_distrobox(app).await?;
        ensure_kali(app).await?;
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

fn store_id(name: &str) -> &'static str {
    match name {
        "Firefox"=>"org.mozilla.firefox","VSCode"=>"com.visualstudio.code",
        "LibreOffice"=>"org.libreoffice.LibreOffice","Thunderbird"=>"org.mozilla.Thunderbird",
        "GIMP"=>"org.gimp.GIMP","Inkscape"=>"org.inkscape.Inkscape","Blender"=>"org.blender.Blender",
        "VLC"=>"org.videolan.VLC","Audacity"=>"org.audacityteam.Audacity",
        "FileZilla"=>"org.filezillaproject.Filezilla","VirtualBox"=>"org.virtualbox.VirtualBox",
        "Tor Browser"=>"com.github.micahflee.torbrowser-launcher","KeePassXC"=>"org.keepassxc.KeePassXC",
        "Wireshark"=>"org.wireshark.Wireshark","Obsidian"=>"md.obsidian.Obsidian",
        "Discord"=>"com.discordapp.Discord","Telegram"=>"org.telegram.desktop",
        "Signal"=>"org.signal.Signal","Spotify"=>"com.spotify.Client",
        "OBS Studio"=>"com.obsproject.Studio","Kdenlive"=>"org.kde.kdenlive",
        "Handbrake"=>"fr.handbrake.ghb","Krita"=>"org.kde.krita",
        "Darktable"=>"org.darktable.Darktable","Shotcut"=>"org.shotcut.Shotcut",
        "Nextcloud"=>"com.nextcloud.desktopclient","Bitwarden"=>"com.bitwarden.desktop",
        "Flatseal"=>"com.github.tchx84.Flatseal","LocalSend"=>"org.localsend.localsend_app",
        "Bottles"=>"com.usebottles.bottles","Vesktop"=>"dev.vencord.Vesktop",
        "Zed"=>"dev.zed.Zed","Neovide"=>"com.neovide.neovide",
        "Postman"=>"com.getpostman.Postman","DBeaver"=>"io.dbeaver.DBeaverCommunity",
        "Insomnia"=>"rest.insomnia.Insomnia","Meld"=>"org.gnome.meld",
        "FreeCAD"=>"org.freecadweb.FreeCAD","Prusa Slicer"=>"com.prusa3d.PrusaSlicer",
        "Calibre"=>"com.calibre_ebook.calibre","Remmina"=>"org.remmina.Remmina",
        "Pika Backup"=>"org.gnome.World.PikaBackup",
        _=>"",
    }
}

async fn install_store(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    let id = store_id(name);
    if id.is_empty() { return Err(format!("No Flatpak ID for: {name}")); }
    flatpak_install(app, id).await
}

async fn install_driver(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    ensure_nonfree(app).await?;
    let pkgs: &[&str] = match name {
        "NVIDIA Driver"       => &["nvidia-driver", "firmware-misc-nonfree"],
        "AMD Driver"          => &["firmware-amd-graphics","libgl1-mesa-dri","xserver-xorg-video-amdgpu"],
        "Intel Driver"        => &["firmware-misc-nonfree","intel-media-va-driver","i965-va-driver","libva-drm2"],
        "WiFi — Broadcom"     => &["broadcom-sta-dkms","dkms","linux-headers-amd64"],
        "WiFi — Realtek"      => &["rtl8812au-dkms","dkms","linux-headers-amd64"],
        "Firmware (non-free)" => &["firmware-linux-nonfree","firmware-misc-nonfree","firmware-realtek","firmware-iwlwifi","firmware-atheros"],
        _ => return Err(format!("Unknown driver: {name}")),
    };
    emit_log(app, "info", &format!("Installing {}...", name));
    emit_prog(app, "install", &format!("Installing {}...", name), 0.3);
    apt_install(app, pkgs).await?;
    emit_prog(app, "done", "Done!", 1.0);
    Ok(())
}

// ─── update_system ────────────────────────────────────────────────────────────

#[tauri::command]
async fn update_system(app: tauri::AppHandle) -> Result<String, String> {
    let home   = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let script = format!("{home}/.hackeros/hacker/update-system");
    emit_log(&app, "info", "Running system update...");
    emit_prog(&app, "update", "Running system update...", 0.1);
    run_streaming(&app, &[&script]).await?;
    emit_prog(&app, "done", "System updated!", 1.0);
    emit_log(&app, "success", "System updated successfully.");
    Ok("System updated successfully.".into())
}

// ─── discover_search (parallel tokio::join!) ──────────────────────────────────

async fn search_apt(query: String) -> Vec<DiscoverResult> {
    let Ok(out) = Command::new("apt-cache")
        .args(["search", "--names-only", &query]).output().await else { return vec![]; };
    let items: Vec<(String,String)> = String::from_utf8_lossy(&out.stdout)
        .lines().take(14).filter_map(|l| {
            let mut p = l.splitn(2, " - ");
            let n = p.next()?.trim().to_string();
            let d = p.next().unwrap_or("").trim().to_string();
            if n.is_empty() { return None; }
            Some((n, d))
        }).collect();
    if items.is_empty() { return vec![]; }
    // batch dpkg-query for versions
    let names: Vec<&str> = items.iter().map(|(n,_)| n.as_str()).collect();
    let dpkg = Command::new("dpkg-query").arg("-W").arg("-f=${Package} ${Version}\n")
        .args(&names).output().await
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
        DiscoverResult { name:name.clone(), version:ver, desc, source:"apt".into(), package_id:name, size:None }
    }).collect()
}

async fn search_flatpak(query: String) -> Vec<DiscoverResult> {
    let Ok(out) = Command::new("flatpak")
        .args(["search","--columns=name,description,application,version",&query])
        .output().await else { return vec![]; };
    String::from_utf8_lossy(&out.stdout).lines()
        .filter(|l| !l.starts_with("Name")).take(10).filter_map(|line| {
            let c: Vec<&str> = line.split('\t').collect();
            if c.len() < 3 { return None; }
            let name=c[0].trim().to_string(); if name.is_empty() { return None; }
            let desc=c[1].trim().to_string();
            let id  =c[2].trim().to_string();
            let ver =c.get(3).map(|s|s.trim().to_string()).unwrap_or_default();
            Some(DiscoverResult { name, version:ver, desc, source:"flatpak".into(), package_id:id, size:None })
        }).collect()
}

async fn search_snap(query: String) -> Vec<DiscoverResult> {
    let Ok(out) = Command::new("snap").args(["find",&query]).output().await else { return vec![]; };
    String::from_utf8_lossy(&out.stdout).lines().skip(1).take(8).filter_map(|line| {
        let c: Vec<&str> = line.split_whitespace().collect();
        if c.is_empty() { return None; }
        let name=c[0].to_string();
        let ver =c.get(1).map(|s|s.to_string()).unwrap_or_default();
        let desc=c.get(3..).map(|s|s.join(" ")).unwrap_or_default();
        Some(DiscoverResult { name:name.clone(), version:ver, desc, source:"snap".into(), package_id:name, size:None })
    }).collect()
}

async fn search_brew(query: String) -> Vec<DiscoverResult> {
    let Ok(out) = Command::new("brew").args(["search",&query]).output().await else { return vec![]; };
    let mut res = vec![];
    let mut in_sec = false;
    for line in String::from_utf8_lossy(&out.stdout).lines().take(40) {
        if line.starts_with('=') { in_sec=true; continue; }
        if !in_sec { continue; }
        let name=line.trim().to_string(); if name.is_empty() { continue; }
        res.push(DiscoverResult { name:name.clone(), version:String::new(), desc:String::new(),
            source:"brew".into(), package_id:name, size:None });
        if res.len()>=8 { break; }
    }
    res
}

#[tauri::command]
async fn discover_search(query: String) -> Vec<DiscoverResult> {
    let (apt, fp, snap, brew) = tokio::join!(
        search_apt(query.clone()),
        search_flatpak(query.clone()),
        search_snap(query.clone()),
        search_brew(query.clone()),
    );
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out: Vec<DiscoverResult> = Vec::new();
    for r in apt.into_iter().chain(fp).chain(snap).chain(brew) {
        if seen.insert(r.name.to_lowercase()) { out.push(r); }
    }
    out
}

#[tauri::command]
async fn discover_install(app: tauri::AppHandle, package_id: String, source: String) -> Result<String, String> {
    emit_log(&app, "info", &format!("Installing {} via {}...", package_id, source));
    emit_prog(&app, "install", &format!("Installing {}...", package_id), 0.2);
    match source.as_str() {
        "apt"     => apt_install(&app, &[package_id.as_str()]).await?,
        "flatpak" => {
            ensure_flatpak(&app).await?;
            if run_sh(&app, &format!("flatpak install -y --user flathub '{package_id}'")).await.is_err() {
                run_sh(&app, &format!("sudo flatpak install -y flathub '{package_id}'")).await?;
            }
        },
        "snap" => priv_run(&app, &["snap","install",&package_id]).await?,
        "brew" => run_sh(&app, &format!("brew install '{package_id}'")).await?,
        _ => return Err(format!("Unknown source: {source}")),
    }
    emit_prog(&app, "done", "Done!", 1.0);
    emit_log(&app, "success", &format!("{package_id} installed."));
    Ok(format!("{package_id} installed."))
}

// ─── get_package_info ─────────────────────────────────────────────────────────

#[tauri::command]
async fn get_package_info(name: String, category: String) -> serde_json::Value {
    let mut info = serde_json::json!({"size":null,"version":null});
    match category.as_str() {
        "store"|"game_launchers" => {
            let id = store_id(&name);
            if id.is_empty() { return info; }
            if let Ok(out) = Command::new("flatpak")
                .args(["remote-info","--user","flathub",id]).output().await {
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
        },
        "pentest_tools" => {
            if let Ok(out) = Command::new("apt-cache")
                .args(["show","--no-all-versions",&name]).output().await {
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
        },
        _ => {}
    }
    info
}

// ─── run ──────────────────────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            install_package,
            update_system,
            check_all_installed,
            discover_search,
            discover_install,
            get_package_info,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
