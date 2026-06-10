using Gtk;
using GLib;
using Gdk;
using Posix;
using Soup;

errordomain HackerError {
    FAILED,
    NOT_SUPPORTED,
    CANCELLED
}

// ── Progress window ───────────────────────────────────────────────────────────

private class ProgressDialog : Gtk.Window {
    public Gtk.ProgressBar progress_bar;
    private Cancellable _cancel = new Cancellable ();
    private Gtk.Label   _status;

    public ProgressDialog (Gtk.Window? parent, string title) {
        set_transient_for (parent);
        set_modal (true);
        set_title (title);
        set_default_size (420, -1);
        set_resizable (false);

        var box = new Gtk.Box (Orientation.VERTICAL, 10) {
            margin_top = 22, margin_bottom = 22,
            margin_start = 22, margin_end = 22
        };
        box.append (new Gtk.Label (title) {
            css_classes = { "pd-title" }, halign = Align.START
        });
        _status = new Gtk.Label ("Preparing…") {
            halign = Align.START, css_classes = { "pd-status" }
        };
        box.append (_status);
        progress_bar = new Gtk.ProgressBar () {
            show_text = false, css_classes = { "pd-bar" }
        };
        box.append (progress_bar);
        var btn = new Gtk.Button.with_label ("Cancel") {
            halign = Align.END, margin_top = 6, css_classes = { "btn-flat" }
        };
        btn.clicked.connect (() => _cancel.cancel ());
        box.append (btn);
        set_child (box);
    }

    public void set_progress (double f) {
        progress_bar.set_fraction (f);
        _status.set_text ("Downloading… %.0f%%".printf (f * 100));
    }
    public void set_status (string t) { _status.set_text (t); }
    public Cancellable get_cancellable () { return _cancel; }
}

// ── Application ───────────────────────────────────────────────────────────────

public class HackerOSStore : Gtk.Application {
    private Gtk.Window   window;
    private Gtk.Stack    stack;
    private Gtk.ListBox  category_list;

    private const string[] CATEGORIES = {
        "Game Launchers", "Pentest Tools", "Applications", "Drivers/Hardware"
    };

    private HashTable<string, string> game_launchers;
    private HashTable<string, string> pentest_tools;
    private HashTable<string, string> applications;
    private HashTable<string, string> drivers;
    private HashTable<string, bool>   installed_state;

    public HackerOSStore () {
        Object (application_id: "com.hackeros.store", flags: ApplicationFlags.FLAGS_NONE);
        installed_state = new HashTable<string, bool> (str_hash, str_equal);

        // ── Game launchers ───────────────────────────────────────────────────
        game_launchers = new HashTable<string, string> (str_hash, str_equal);
        game_launchers.insert ("Steam",            "Install Steam via Flatpak.");
        game_launchers.insert ("GOG",              "Install GOG Galaxy via Wine.");
        game_launchers.insert ("Battle.net",       "Install Battle.net via Wine.");
        game_launchers.insert ("Epic Games Store", "Install Epic Games Store via Heroic Launcher (Flatpak).");
        game_launchers.insert ("EA App",           "Install EA App via Wine.");
        game_launchers.insert ("Lutris",           "Install Lutris gaming platform (Flatpak).");
        game_launchers.insert ("Heroic",           "Install Heroic Games Launcher — Epic/GOG (Flatpak).");

        // ── Pentest tools ────────────────────────────────────────────────────
        pentest_tools = new HashTable<string, string> (str_hash, str_equal);
        pentest_tools.insert ("nmap",         "Network scanner and host discovery.");
        pentest_tools.insert ("metasploit",   "Exploitation and post-exploitation framework.");
        pentest_tools.insert ("wireshark",    "Packet analyzer and protocol inspector.");
        pentest_tools.insert ("john",         "Password cracker (John the Ripper).");
        pentest_tools.insert ("hydra",        "Fast network login brute-force tool.");
        pentest_tools.insert ("burpsuite",    "Web vulnerability scanner and proxy.");
        pentest_tools.insert ("sqlmap",       "Automatic SQL injection tool.");
        pentest_tools.insert ("nikto",        "Web server vulnerability scanner.");
        pentest_tools.insert ("aircrack-ng",  "WiFi security auditing suite.");
        pentest_tools.insert ("hashcat",      "Advanced GPU-accelerated password recovery.");
        pentest_tools.insert ("bettercap",    "MITM framework for network attacks.");
        pentest_tools.insert ("theharvester", "OSINT tool for emails and subdomains.");
        pentest_tools.insert ("maltego",      "Intelligence and forensics visualization.");
        pentest_tools.insert ("zaproxy",      "OWASP web application security scanner.");
        pentest_tools.insert ("gobuster",     "Directory, DNS and VHost busting tool.");
        pentest_tools.insert ("enum4linux",   "SMB/Samba enumeration tool.");
        pentest_tools.insert ("responder",    "LLMNR/NBT-NS/mDNS poisoner.");
        pentest_tools.insert ("impacket",     "Python classes for network protocol work.");
        pentest_tools.insert ("crackmapexec", "Swiss army knife for network pentesting.");
        pentest_tools.insert ("recon-ng",     "Web reconnaissance framework.");
        pentest_tools.insert ("beef-xss",     "Browser Exploitation Framework.");
        pentest_tools.insert ("volatility",   "Memory forensics framework.");
        pentest_tools.insert ("autopsy",      "Digital forensics platform.");
        pentest_tools.insert ("dnsrecon",     "DNS enumeration script.");
        pentest_tools.insert ("arp-scan",     "ARP scanning and fingerprinting tool.");
        pentest_tools.insert ("masscan",      "Mass IP port scanner.");
        pentest_tools.insert ("sslscan",      "SSL/TLS configuration scanner.");
        pentest_tools.insert ("wpscan",       "WordPress vulnerability scanner.");
        pentest_tools.insert ("openvas",      "Open Vulnerability Assessment System.");
        pentest_tools.insert ("bloodhound",   "Active Directory attack graph tool.");
        pentest_tools.insert ("evil-winrm",   "WinRM shell for pentesting Windows.");
        pentest_tools.insert ("netcat",       "Network utility for reading/writing.");
        pentest_tools.insert ("tcpdump",      "Command-line packet analyzer.");
        pentest_tools.insert ("scapy",        "Packet manipulation and crafting library.");

        // ── Applications ─────────────────────────────────────────────────────
        applications = new HashTable<string, string> (str_hash, str_equal);
        applications.insert ("Firefox",     "Fast and private web browser.");
        applications.insert ("VSCode",      "Powerful code editor by Microsoft.");
        applications.insert ("LibreOffice", "Free and open-source office suite.");
        applications.insert ("Thunderbird", "Email and calendar client.");
        applications.insert ("GIMP",        "GNU Image Manipulation Program.");
        applications.insert ("Inkscape",    "Professional vector graphics editor.");
        applications.insert ("Blender",     "3D creation suite for modeling and animation.");
        applications.insert ("VLC",         "Versatile media player.");
        applications.insert ("Audacity",    "Multi-track audio editor and recorder.");
        applications.insert ("FileZilla",   "Fast and reliable FTP/SFTP client.");
        applications.insert ("VirtualBox",  "x86 and AMD64 virtualization software.");
        applications.insert ("Tor Browser", "Privacy-focused browser via Tor network.");
        applications.insert ("KeePassXC",   "Secure cross-platform password manager.");
        applications.insert ("Wireshark",   "Network protocol analyzer (standalone UI).");

        // ── Drivers ──────────────────────────────────────────────────────────
        drivers = new HashTable<string, string> (str_hash, str_equal);
        drivers.insert ("NVIDIA Driver",       "Install NVIDIA proprietary driver (non-free).");
        drivers.insert ("AMD Driver",          "Install AMD firmware and mesa drivers.");
        drivers.insert ("Intel Driver",        "Install Intel graphics firmware and va-drivers.");
        drivers.insert ("WiFi — Broadcom",     "Install Broadcom STA driver (broadcom-sta-dkms).");
        drivers.insert ("WiFi — Realtek",      "Install Realtek rtl8812au / rtl88xxau driver.");
        drivers.insert ("Firmware (non-free)", "Install firmware-linux-nonfree and firmware-misc-nonfree.");
    }

    // ── activate ──────────────────────────────────────────────────────────────

    protected override void activate () {
        var gtk_settings = Gtk.Settings.get_default ();
        if (gtk_settings != null) gtk_settings.gtk_application_prefer_dark_theme = true;
        load_css ();

        window = new Gtk.Window () {
            title = "HackerOS Store",
            default_width = 900, default_height = 640,
            css_classes = { "app-window" }
        };
        window.set_application (this);

        var header = new Gtk.HeaderBar () { css_classes = { "app-header" } };
        header.set_show_title_buttons (true);
        header.set_title_widget (new Gtk.Label ("HackerOS Store") { css_classes = { "header-title" } });
        window.set_titlebar (header);

        var paned = new Gtk.Paned (Orientation.HORIZONTAL);
        paned.set_position (204);

        // Sidebar
        var sidebar = new Gtk.Box (Orientation.VERTICAL, 0) { css_classes = { "sidebar" } };
        sidebar.append (new Gtk.Label ("Categories") {
            halign = Align.START, css_classes = { "sidebar-section" },
            margin_top = 18, margin_bottom = 8, margin_start = 14, margin_end = 14
        });
        category_list = new Gtk.ListBox () {
            selection_mode = SelectionMode.SINGLE,
            css_classes = { "cat-list" }, hexpand = true
        };
        category_list.row_selected.connect (on_category_selected);
        sidebar.append (category_list);
        foreach (string cat in CATEGORIES)
            category_list.append (build_category_row (cat));
        sidebar.append (new Gtk.Label ("v0.6 · Debian") {
            css_classes = { "sidebar-ver" }, halign = Align.CENTER,
            vexpand = true, valign = Align.END, margin_bottom = 12
        });
        paned.set_start_child (sidebar);
        paned.set_shrink_start_child (false);
        paned.set_resize_start_child (false);

        // Content stack
        stack = new Gtk.Stack () {
            transition_type = StackTransitionType.CROSSFADE,
            transition_duration = 150,
            hexpand = true, vexpand = true
        };
        foreach (string cat in CATEGORIES) {
            var sw = new Gtk.ScrolledWindow () { hscrollbar_policy = PolicyType.NEVER };
            var lb = new Gtk.ListBox () {
                selection_mode = SelectionMode.NONE, css_classes = { "item-list" }
            };
            sw.set_child (lb);
            stack.add_named (sw, cat);
            var items = get_items_for_category (cat);
            if (items != null)
                items.foreach ((n, d) => lb.append (create_item_row (n, d, cat)));
        }
        paned.set_end_child (stack);
        paned.set_shrink_end_child (false);
        window.set_child (paned);

        var first = category_list.get_row_at_index (0);
        if (first != null) category_list.select_row (first);
        window.present ();
    }

    // ── CSS ───────────────────────────────────────────────────────────────────

    private void load_css () {
        var display = Gdk.Display.get_default ();
        if (display == null) return;
        var p = new Gtk.CssProvider ();
        p.load_from_data ("""
window.app-window { background-color: #1c1c1e; color: #e5e5ea; }

headerbar.app-header {
    background-color: #1c1c1e;
    border-bottom: 1px solid #2c2c2e;
    min-height: 46px;
    box-shadow: none;
}
.header-title { font-size: 0.9em; font-weight: 600; color: #e5e5ea; }

.sidebar { background-color: #161618; border-right: 1px solid #2c2c2e; min-width: 196px; }
.sidebar-section { font-size: 0.7em; font-weight: 600; letter-spacing: 0.07em; color: #636366; }
.sidebar-ver { font-size: 0.68em; color: #3a3a3c; }

.cat-list { background: transparent; border: none; }
.cat-list row { background: transparent; border-radius: 7px; margin: 1px 8px; border: none; }
.cat-list row:hover    { background-color: rgba(255,255,255,0.06); }
.cat-list row:selected { background-color: rgba(10,132,255,0.18); }
.cat-list row:selected .cat-label { color: #0a84ff; }
.cat-list row:selected .cat-icon  { color: #0a84ff; }
.cat-row   { padding: 9px 10px; }
.cat-icon  { color: #636366; }
.cat-label { font-size: 0.88em; font-weight: 500; color: #8e8e93; }

.item-list { background-color: #1c1c1e; border: none; }
.item-list > row { background: transparent; border-bottom: 1px solid #2c2c2e; border-radius: 0; padding: 0; }
.item-list > row:hover { background-color: rgba(255,255,255,0.04); }

.item-icon { color: #48484a; min-width: 44px; }
.item-name { font-size: 0.92em; font-weight: 600; color: #e5e5ea; }
.item-desc { font-size: 0.8em; color: #636366; }

button.btn-install {
    background-color: #0a84ff; color: #fff;
    border: none; border-radius: 8px;
    padding: 7px 18px;
    font-size: 0.82em; font-weight: 600; min-width: 84px;
}
button.btn-install:hover   { background-color: #2196ff; }
button.btn-install:active  { background-color: #0060df; }
button.btn-install:disabled { background-color: #2c2c2e; color: #48484a; }

button.btn-installed {
    background-color: transparent; color: #30d158;
    border: 1px solid #1f3d28; border-radius: 8px;
    padding: 7px 18px; font-size: 0.82em; font-weight: 600; min-width: 84px;
}

button.btn-flat {
    background: none; border: none; color: #8e8e93;
    border-radius: 6px; padding: 5px;
}
button.btn-flat:hover { background-color: rgba(255,255,255,0.07); color: #e5e5ea; }

.pd-title  { font-size: 0.9em; font-weight: 600; color: #e5e5ea; }
.pd-status { font-size: 0.78em; color: #8e8e93; }
progressbar.pd-bar trough  { background-color: #2c2c2e; border-radius: 3px; min-height: 5px; }
progressbar.pd-bar progress { background-color: #0a84ff; border-radius: 3px; min-height: 5px; }

scrollbar { background: transparent; }
scrollbar slider { background-color: #3a3a3c; border-radius: 4px; min-width: 4px; min-height: 4px; }
scrollbar slider:hover { background-color: #48484a; }
paned > separator { background-color: #2c2c2e; min-width: 1px; }
""".data);
        Gtk.StyleContext.add_provider_for_display (display, p, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    // ── UI helpers ────────────────────────────────────────────────────────────

    private Gtk.ListBoxRow build_category_row (string cat) {
        var row  = new Gtk.ListBoxRow ();
        var hbox = new Gtk.Box (Orientation.HORIZONTAL, 9) { css_classes = { "cat-row" } };
        var icon = new Gtk.Image.from_icon_name (get_category_icon (cat)) { css_classes = { "cat-icon" } };
        icon.set_pixel_size (16);
        hbox.append (icon);
        hbox.append (new Gtk.Label (cat) {
            halign = Align.START, hexpand = true, css_classes = { "cat-label" }
        });
        row.set_child (hbox);
        return row;
    }

    private string get_category_icon (string cat) {
        switch (cat) {
            case "Game Launchers":   return "input-gaming-symbolic";
            case "Pentest Tools":    return "security-high-symbolic";
            case "Applications":     return "applications-system-symbolic";
            case "Drivers/Hardware": return "drive-harddisk-symbolic";
            default:                 return "folder-symbolic";
        }
    }

    private HashTable<string, string>? get_items_for_category (string cat) {
        switch (cat) {
            case "Game Launchers":   return game_launchers;
            case "Pentest Tools":    return pentest_tools;
            case "Applications":     return applications;
            case "Drivers/Hardware": return drivers;
            default:                 return null;
        }
    }

    private Gtk.ListBoxRow create_item_row (string name, string desc, string category) {
        var row  = new Gtk.ListBoxRow ();
        var hbox = new Gtk.Box (Orientation.HORIZONTAL, 12) {
            margin_top = 12, margin_bottom = 12,
            margin_start = 18, margin_end = 18
        };
        var icon = new Gtk.Image.from_icon_name (get_item_icon (name, category)) {
            css_classes = { "item-icon" }
        };
        icon.set_pixel_size (36);
        hbox.append (icon);

        var vbox = new Gtk.Box (Orientation.VERTICAL, 3) { hexpand = true, valign = Align.CENTER };
        vbox.append (new Gtk.Label (name) { halign = Align.START, css_classes = { "item-name" } });
        vbox.append (new Gtk.Label (desc) {
            halign = Align.START, wrap = true, max_width_chars = 58, css_classes = { "item-desc" }
        });
        hbox.append (vbox);

        string key = category + "::" + name;
        Gtk.Button btn;
        if (installed_state.contains (key)) {
            btn = new Gtk.Button.with_label ("✓ Installed") {
                valign = Align.CENTER, css_classes = { "btn-installed" }, sensitive = false
            };
        } else {
            btn = new Gtk.Button.with_label ("Install") {
                valign = Align.CENTER, css_classes = { "btn-install" }
            };
            btn.clicked.connect (() => {
                btn.set_sensitive (false);
                btn.set_label ("Installing…");
                do_install (name, category, btn);
            });
        }
        hbox.append (btn);
        row.set_child (hbox);
        return row;
    }

    private string get_item_icon (string name, string category) {
        switch (name) {
            case "Steam": case "Lutris": case "Heroic": return "input-gaming-symbolic";
            case "Firefox": case "Tor Browser": return "web-browser-symbolic";
            case "VSCode": return "text-editor-symbolic";
            case "GIMP": case "Inkscape": return "image-x-generic-symbolic";
            case "VLC": return "media-playback-start-symbolic";
            case "Blender": return "applications-graphics-symbolic";
            case "KeePassXC": return "dialog-password-symbolic";
            case "VirtualBox": return "computer-symbolic";
            case "Thunderbird": return "mail-unread-symbolic";
            case "Audacity": return "audio-x-generic-symbolic";
            case "nmap": case "masscan": case "arp-scan": return "network-wired-symbolic";
            case "wireshark": case "tcpdump": return "network-receive-symbolic";
            case "john": case "hydra": case "hashcat": return "dialog-password-symbolic";
            case "NVIDIA Driver": case "AMD Driver": case "Intel Driver": return "video-display-symbolic";
            default:
                if (category == "Pentest Tools")    return "security-medium-symbolic";
                if (category == "Game Launchers")   return "input-gaming-symbolic";
                if (category == "Drivers/Hardware") return "drive-harddisk-symbolic";
                return "application-x-executable-symbolic";
        }
    }

    private void on_category_selected (Gtk.ListBoxRow? row) {
        if (row == null) return;
        var hbox = row.get_child () as Gtk.Box;
        if (hbox == null) return;
        var lbl = hbox.get_last_child () as Gtk.Label;
        if (lbl != null) stack.set_visible_child_name (lbl.get_text ());
    }

    // ── Install dispatch ──────────────────────────────────────────────────────

    private void do_install (string name, string category, Gtk.Button btn) {
        install_async.begin (name, category, (obj, res) => {
            string msg;
            try {
                install_async.end (res);
                msg = "%s installed successfully.".printf (name);
                installed_state.insert (category + "::" + name, true);
                btn.set_label ("✓ Installed");
                btn.set_css_classes ({ "btn-installed" });
            } catch (IOError.CANCELLED e) {
                msg = "Installation cancelled.";
                btn.set_label ("Install");
                btn.set_css_classes ({ "btn-install" });
                btn.set_sensitive (true);
            } catch (Error e) {
                msg = "Error installing %s:\n%s".printf (name, e.message);
                btn.set_label ("Retry");
                btn.set_css_classes ({ "btn-install" });
                btn.set_sensitive (true);
            }
            show_alert (msg);
        });
    }

    private async void install_async (string name, string category) throws Error {
        switch (category) {
            case "Game Launchers":   yield install_game_launcher_async (name); break;
            case "Pentest Tools":    yield install_pentest_tool_async (name);  break;
            case "Applications":     yield install_application_async (name);   break;
            case "Drivers/Hardware": yield install_driver_async (name);        break;
            default: throw new HackerError.NOT_SUPPORTED ("Unknown category.");
        }
    }

    private void show_alert (string msg) {
        var d = new Gtk.AlertDialog (msg);
        d.set_buttons ({ "OK" });
        d.choose.begin (window, null, (obj, res) => {
            try { d.choose.end (res); } catch (Error e) { }
        });
    }

    // ── Game Launchers ────────────────────────────────────────────────────────
    //
    //  On Debian we use Wine (apt) — Proton-GE requires Steam Runtime container
    //  libraries and exits 255 on a bare Debian system.
    //
    //  Strategy:
    //    Steam        → Flatpak (simplest, always works)
    //    Lutris       → Flatpak
    //    Heroic       → Flatpak  (also covers Epic + GOG natively)
    //    GOG/Battle.net/EA App → Wine via apt, each in isolated WINEPREFIX

    private async void install_game_launcher_async (string name) throws Error {
        switch (name) {
            case "Steam":
                yield install_flatpak_app ("com.valvesoftware.Steam");
                break;
            case "Lutris":
                yield install_flatpak_app ("net.lutris.Lutris");
                break;
            case "Heroic":
            case "Epic Games Store":
                // Heroic covers both Epic and GOG without Wine complexity
                yield install_flatpak_app ("com.heroicgameslauncher.hgl");
                break;
            case "GOG":
                yield install_wine_launcher_async (
                    "GOG Galaxy",
                    "gog",
                    "https://webinstallers.gog.com/galaxy_installer_en.exe",
                    ".exe",
                    "GOG Galaxy/GalaxyClient.exe");
                break;
            case "Battle.net":
                yield install_wine_launcher_async (
                    "Battle.net",
                    "battlenet",
                    "https://www.battle.net/download/getInstaller?os=win&installer=Battle.net-Setup.exe",
                    ".exe",
                    "Battle.net/Battle.net.exe");
                break;
            case "EA App":
                yield install_wine_launcher_async (
                    "EA App",
                    "ea",
                    "https://origin-a.akamaihd.net/EA-Desktop-Client-Download/installer-releases/EAappInstaller.exe",
                    ".exe",
                    "Electronic Arts/EA Desktop/EADesktop.exe");
                break;
            default:
                throw new HackerError.NOT_SUPPORTED ("Unknown launcher: " + name);
        }
    }

    // ── Wine launcher installer ───────────────────────────────────────────────
    //
    //  1. Ensure wine wine64 winetricks are installed via apt.
    //  2. Download the Windows installer.
    //  3. Run it in an isolated WINEPREFIX with wine.
    //  4. Write a .desktop launcher.

    private async void install_wine_launcher_async (string display_name,
                                                     string id,
                                                     string url,
                                                     string ext,
                                                     string rel_exe) throws Error {
        // 1. Install Wine if needed
        yield ensure_wine_async ();

        var home         = Environment.get_home_dir ();
        var launcher_dir = Path.build_filename (home, ".hackeros", "launchers", id);
        var prefix       = Path.build_filename (launcher_dir, "prefix");
        DirUtils.create_with_parents (prefix, 0755);

        // 2. Download installer
        var installer = Path.build_filename (launcher_dir, "installer" + ext);
        var prog = new ProgressDialog (window, "Downloading %s".printf (display_name));
        prog.show ();
        try { yield download_file_async (url, installer, prog); }
        finally { prog.destroy (); }

        // 3. Initialise WINEPREFIX (creates registry etc.) — needed before running installer
        prog = new ProgressDialog (window, "Initialising Wine prefix…");
        prog.set_status ("This may take a minute…");
        prog.show ();
        try {
            yield run_wine_async (prefix, { "wineboot", "--init" });
        } finally {
            prog.destroy ();
        }

        // 4. Run installer silently
        prog = new ProgressDialog (window, "Installing %s via Wine…".printf (display_name));
        prog.set_status ("Running Windows installer…");
        prog.show ();
        try {
            yield run_wine_async (prefix, { "wine", installer, "/S" });
        } finally {
            prog.destroy ();
        }

        // 5. Write .desktop
        var exe_full = Path.build_filename (prefix, "drive_c", "Program Files (x86)", rel_exe);
        if (!File.new_for_path (exe_full).query_exists ()) {
            // Try Program Files (non-x86)
            exe_full = Path.build_filename (prefix, "drive_c", "Program Files", rel_exe);
        }
        write_wine_desktop (id, display_name, exe_full, prefix);
    }

    private async void ensure_wine_async () throws Error {
        // Check if wine is already installed
        int st = 0;
        try { Process.spawn_command_line_sync ("which wine", null, null, out st); }
        catch (SpawnError e) { st = 1; }
        if (st == 0) return; // already present

        // Enable i386 architecture (needed for 32-bit wine on Debian)
        yield apt_run ({ "dpkg", "--add-architecture", "i386" });
        yield apt_run ({ "apt-get", "update", "-qq" });
        yield apt_run ({
            "apt-get", "install", "-y", "--no-install-recommends",
            "wine", "wine64", "wine32", "winetricks",
            "libgl1", "libgl1:i386"
        });
    }

    private async void run_wine_async (string prefix, string[] cmd) throws Error {
        var env = build_wine_env (prefix);
        // Use pkexec-free approach: we can't run wine as root anyway
        yield run_cmd_async (cmd, env);
    }

    private string[] build_wine_env (string prefix) {
        var env = new GenericArray<string> ();
        foreach (string key in new string[] {
                "HOME", "USER", "PATH", "DISPLAY", "WAYLAND_DISPLAY",
                "XDG_RUNTIME_DIR", "XDG_DATA_DIRS", "DBUS_SESSION_BUS_ADDRESS",
                "PULSE_SERVER", "LD_LIBRARY_PATH" }) {
            string? val = Environment.get_variable (key);
            if (val != null) env.add (key + "=" + val);
        }
        env.add ("WINEPREFIX=" + prefix);
        env.add ("WINEDEBUG=-all");          // suppress Wine debug spam
        env.add ("WINEARCH=win64");
        return env.data;
    }

    private void write_wine_desktop (string id, string name,
                                      string exe, string prefix) throws HackerError {
        var dir  = Path.build_filename (Environment.get_user_data_dir (), "applications");
        DirUtils.create_with_parents (dir, 0755);
        var path = Path.build_filename (dir, id + ".desktop");
        var content =
            "[Desktop Entry]\n" +
            "Name=%s\n".printf (name) +
            "Exec=env WINEPREFIX=%s WINEDEBUG=-all wine \"%s\"\n".printf (prefix, exe) +
            "Type=Application\nCategories=Game;\n";
        try {
            File.new_for_path (path).replace_contents (
                content.data, null, false, FileCreateFlags.NONE, null);
        } catch (Error e) {
            throw new HackerError.FAILED ("Failed to write .desktop: " + e.message);
        }
    }

    // ── Pentest tools ──────────────────────────────────────────────────────────
    //
    //  Uses distrobox + Kali Linux container (Debian-based, best apt compatibility).
    //  Falls back to direct apt if the tool exists in Debian repos.

    private async void install_pentest_tool_async (string name) throws Error {
        // First try direct apt on Debian (many tools are in Debian repos)
        if (tool_in_debian_repo (name)) {
            yield apt_run ({ "apt-get", "install", "-y", name });
            return;
        }

        // Otherwise use distrobox + Kali
        yield ensure_distrobox_async ();
        yield ensure_kali_container_async ();
        yield run_cmd_async ({
            "distrobox", "enter", "kali-pentest", "--",
            "sudo", "apt-get", "install", "-y", name
        }, null);

        // Wrapper script
        var bin      = Path.build_filename (Environment.get_home_dir (), ".local", "bin");
        var wrapper  = Path.build_filename (bin, name);
        DirUtils.create_with_parents (bin, 0755);
        File.new_for_path (wrapper).replace_contents (
            ("#!/bin/sh\ndistrobox enter kali-pentest -- %s \"$@\"\n".printf (name)).data,
            null, false, FileCreateFlags.NONE, null);
        FileUtils.chmod (wrapper, 0755);

        // .desktop
        var desktop = Path.build_filename (
            Environment.get_user_data_dir (), "applications", name + ".desktop");
        File.new_for_path (desktop).replace_contents (
            ("[Desktop Entry]\nName=%s\nExec=%s\nType=Application\nCategories=Security;\n".printf (
                name, wrapper)).data,
            null, false, FileCreateFlags.NONE, null);
    }

    // Tools available directly in Debian repos (no container needed)
    private bool tool_in_debian_repo (string name) {
        switch (name) {
            case "nmap": case "wireshark": case "tcpdump": case "netcat":
            case "aircrack-ng": case "masscan": case "arp-scan":
            case "sqlmap": case "nikto": case "john": case "hydra":
            case "sslscan": case "dnsrecon": case "fierce":
            case "enum4linux": case "gobuster": case "recon-ng":
            case "volatility": case "autopsy":
                return true;
            default:
                return false;
        }
    }

    private async void ensure_distrobox_async () throws Error {
        int st = 0;
        try { Process.spawn_command_line_sync ("which distrobox", null, null, out st); }
        catch (SpawnError e) { st = 1; }
        if (st == 0) return;
        // Install distrobox from Debian repos (available in trixie/testing)
        yield apt_run ({ "apt-get", "install", "-y", "distrobox" });
    }

    private async void ensure_kali_container_async () throws Error {
        // Check if container already exists
        string out_str = "";
        int st = 0;
        try {
            Process.spawn_command_line_sync (
                "distrobox list", out out_str, null, out st);
        } catch (SpawnError e) { st = 1; }
        if (st == 0 && out_str.contains ("kali-pentest")) return;

        yield run_cmd_async ({
            "distrobox", "create",
            "--image", "kalilinux/kali-rolling",
            "--name",  "kali-pentest",
            "--yes"
        }, null);
        // Initial update inside container
        try {
            yield run_cmd_async ({
                "distrobox", "enter", "kali-pentest", "--",
                "sudo", "apt-get", "update", "-qq"
            }, null);
        } catch (Error e) { /* non-fatal */ }
    }

    // ── Applications ──────────────────────────────────────────────────────────

    private async void install_application_async (string name) throws Error {
        string fid;
        switch (name) {
            case "Firefox":     fid = "org.mozilla.firefox";                       break;
            case "VSCode":      fid = "com.visualstudio.code";                     break;
            case "LibreOffice": fid = "org.libreoffice.LibreOffice";               break;
            case "Thunderbird": fid = "org.mozilla.Thunderbird";                   break;
            case "GIMP":        fid = "org.gimp.GIMP";                             break;
            case "Inkscape":    fid = "org.inkscape.Inkscape";                     break;
            case "Blender":     fid = "org.blender.Blender";                       break;
            case "VLC":         fid = "org.videolan.VLC";                          break;
            case "Audacity":    fid = "org.audacityteam.Audacity";                 break;
            case "FileZilla":   fid = "org.filezillaproject.Filezilla";            break;
            case "VirtualBox":  fid = "org.virtualbox.VirtualBox";                 break;
            case "Tor Browser": fid = "com.github.micahflee.torbrowser-launcher"; break;
            case "KeePassXC":   fid = "org.keepassxc.KeePassXC";                  break;
            case "Wireshark":   fid = "org.wireshark.Wireshark";                   break;
            default: throw new HackerError.NOT_SUPPORTED ("No Flatpak ID for: " + name);
        }
        yield ensure_flatpak_async ();
        yield install_flatpak_app (fid);
    }

    // ── Drivers ───────────────────────────────────────────────────────────────
    //
    //  All driver installs on Debian require non-free repos.
    //  We add them if missing before installing.

    private async void install_driver_async (string name) throws Error {
        yield ensure_nonfree_repos_async ();
        switch (name) {
            case "NVIDIA Driver":
                yield apt_run ({ "apt-get", "install", "-y",
                    "nvidia-driver", "firmware-misc-nonfree" });
                break;
            case "AMD Driver":
                yield apt_run ({ "apt-get", "install", "-y",
                    "firmware-amd-graphics", "libgl1-mesa-dri",
                    "xserver-xorg-video-amdgpu" });
                break;
            case "Intel Driver":
                yield apt_run ({ "apt-get", "install", "-y",
                    "firmware-misc-nonfree", "intel-media-va-driver",
                    "i965-va-driver", "libva-drm2" });
                break;
            case "WiFi — Broadcom":
                yield apt_run ({ "apt-get", "install", "-y",
                    "broadcom-sta-dkms", "dkms",
                    "linux-headers-amd64" });
                break;
            case "WiFi — Realtek":
                // rtl8812au is in Debian testing as rtl8812au-dkms
                yield apt_run ({ "apt-get", "install", "-y",
                    "rtl8812au-dkms", "dkms",
                    "linux-headers-amd64" });
                break;
            case "Firmware (non-free)":
                yield apt_run ({ "apt-get", "install", "-y",
                    "firmware-linux-nonfree", "firmware-misc-nonfree",
                    "firmware-realtek", "firmware-iwlwifi",
                    "firmware-atheros" });
                break;
            default:
                throw new HackerError.NOT_SUPPORTED ("Unknown driver: " + name);
        }
    }

    private async void ensure_nonfree_repos_async () throws Error {
        // Check if non-free is already in sources
        string out_str = "";
        int st = 0;
        try {
            Process.spawn_command_line_sync (
                "grep -r non-free /etc/apt/sources.list /etc/apt/sources.list.d/",
                out out_str, null, out st);
        } catch (SpawnError e) { st = 1; }

        if (st != 0 || !out_str.contains ("non-free")) {
            // Detect codename
            string codename = "trixie";
            try {
                string cn = "";
                Process.spawn_command_line_sync ("lsb_release -sc", out cn, null, out st);
                codename = cn.strip ();
            } catch (SpawnError e) { }

            // Write non-free sources file
            var sources_line =
                "deb http://deb.debian.org/debian %s main contrib non-free non-free-firmware\n".printf (codename);
            var sources_path = "/etc/apt/sources.list.d/hackeros-nonfree.list";
            try {
                // Need pkexec or sudo to write to /etc/apt
                string tmp = "/tmp/hackeros-nonfree.list";
                File.new_for_path (tmp).replace_contents (
                    sources_line.data, null, false, FileCreateFlags.NONE, null);
                yield run_cmd_async ({
                    "pkexec", "cp", tmp, sources_path
                }, null);
            } catch (Error e) {
                // Fall back to sudo
                try {
                    yield run_cmd_async ({
                        "sudo", "sh", "-c",
                        "echo '%s' > %s".printf (sources_line.strip (), sources_path)
                    }, null);
                } catch (Error e2) {
                    throw new HackerError.FAILED (
                        "Cannot write apt sources. Run as sudo or grant polkit rights.\n" + e2.message);
                }
            }
            yield apt_run ({ "apt-get", "update", "-qq" });
        }
    }

    // ── Flatpak helpers ───────────────────────────────────────────────────────

    private async void ensure_flatpak_async () throws Error {
        int st = 0;
        try { Process.spawn_command_line_sync ("which flatpak", null, null, out st); }
        catch (SpawnError e) { st = 1; }
        if (st != 0)
            yield apt_run ({ "apt-get", "install", "-y", "flatpak" });

        // Add flathub if not present
        string out_str = "";
        try { Process.spawn_command_line_sync ("flatpak remote-list", out out_str, null, out st); }
        catch (SpawnError e) { out_str = ""; }
        if (!out_str.contains ("flathub"))
            yield run_cmd_async ({
                "flatpak", "remote-add", "--if-not-exists", "flathub",
                "https://dl.flathub.org/repo/flathub.flatpakrepo"
            }, null);
    }

    private async void install_flatpak_app (string app_id) throws Error {
        yield ensure_flatpak_async ();
        yield run_cmd_async ({ "flatpak", "install", "-y", "flathub", app_id }, null);
    }

    // ── apt helper — always uses pkexec/sudo ──────────────────────────────────

    private async void apt_run (string[] args) throws Error {
        // Prepend sudo (pkexec works too but sudo is simpler in terminal context)
        var cmd = new GenericArray<string> ();
        cmd.add ("sudo");
        foreach (string a in args) cmd.add (a);
        yield run_cmd_async (cmd.data, null);
    }

    // ── Download (libsoup, no wget) ───────────────────────────────────────────

    private async void download_file_async (string url, string dest,
                                             ProgressDialog prog) throws Error {
        var cancel  = prog.get_cancellable ();
        var session = new Soup.Session ();
        session.user_agent = "HackerOS-Store/0.6";
        string cur   = url;
        int    redir = 0;

        while (true) {
            var msg = new Soup.Message ("GET", cur);
            InputStream input;
            try { input = yield session.send_async (msg, Priority.DEFAULT, cancel); }
            catch (Error e) {
                if (cancel.is_cancelled ()) throw new IOError.CANCELLED ("Cancelled.");
                throw new HackerError.FAILED ("Connection: " + e.message);
            }
            uint st = msg.status_code;
            if (st == 301 || st == 302 || st == 307 || st == 308) {
                string? loc = msg.response_headers.get_one ("Location");
                if (loc == null || ++redir > 12) throw new HackerError.FAILED ("Too many redirects.");
                cur = loc; continue;
            }
            if (st != 200) throw new HackerError.FAILED ("HTTP %u".printf (st));
            int64 clen = msg.response_headers.get_content_length ();
            var fout = yield File.new_for_path (dest).replace_async (
                null, false, FileCreateFlags.REPLACE_DESTINATION, Priority.DEFAULT, cancel);
            uint8[] buf = new uint8[65536];
            int64 total = 0;
            while (true) {
                ssize_t n = yield input.read_async (buf, Priority.DEFAULT, cancel);
                if (n == 0) break;
                total += n;
                yield fout.write_async (buf[0:n], Priority.DEFAULT, cancel);
                if (clen > 0) prog.set_progress ((double) total / (double) clen);
                else prog.progress_bar.pulse ();
            }
            yield fout.close_async (Priority.DEFAULT, cancel);
            return;
        }
    }

    // ── Async process runner ──────────────────────────────────────────────────
    //
    //  Pass env=null to inherit the current process environment.
    //  Uses ChildWatch so the GTK main loop is never blocked.

    private async void run_cmd_async (string[] argv, string[]? env) throws Error {
        Pid pid;
        var flags = SpawnFlags.SEARCH_PATH |
                    SpawnFlags.STDOUT_TO_DEV_NULL |
                    SpawnFlags.STDERR_TO_DEV_NULL;
        try {
            Process.spawn_async (null, argv, env, flags, null, out pid);
        } catch (SpawnError e) {
            throw new HackerError.FAILED ("spawn '%s': %s".printf (argv[0], e.message));
        }
        int wait_status = -1;
        ChildWatch.add (pid, (p, s) => {
            wait_status = s;
            Process.close_pid (p);
            run_cmd_async.callback ();
        });
        yield;
        int code = Process.exit_status (wait_status);
        if (code != 0)
            throw new HackerError.FAILED (
                "'%s' exited with code %d.".printf (argv[0], code));
    }

    public static int main (string[] args) {
        return new HackerOSStore ().run (args);
    }
}
