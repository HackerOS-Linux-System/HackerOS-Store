using Gtk;
using GLib;
using Gdk;
using Posix;

errordomain HackerError {
    FAILED,
    NOT_SUPPORTED
}

private class ProgressDialog : Gtk.Dialog {
    public Gtk.ProgressBar progress_bar;
    private Cancellable cancellable = new Cancellable();
    public ProgressDialog (Gtk.Window? parent, string title) {
        set_transient_for (parent);
        set_modal (true);
        set_title (title);
        progress_bar = new Gtk.ProgressBar ();
        progress_bar.set_show_text (true);
        get_content_area ().append (progress_bar);
        add_button ("Cancel", ResponseType.CANCEL);
        response.connect ((id) => {
            if (id == ResponseType.CANCEL) {
                cancellable.cancel ();
            }
        });
    }
    public void set_progress (double fraction) {
        progress_bar.set_fraction (fraction);
    }
    public Cancellable get_cancellable () {
        return cancellable;
    }
}

public class HackerOSStore : Gtk.Application {
    private Gtk.Window window;
    private Gtk.Stack stack;
    private Gtk.ListBox category_list;
    private Gtk.Box main_box;
    // Categories
    private const string[] CATEGORIES = {
        "Game Launchers",
        "Pentest Tools",
        "Applications",
        "Drivers/Hardware"
    };
    // Items per category (as maps for simplicity: name -> description)
    private HashTable<string, string> game_launchers;
    private HashTable<string, string> pentest_tools;
    private HashTable<string, string> applications;
    private HashTable<string, string> drivers;
    public HackerOSStore() {
        Object(application_id: "com.hackeros.store", flags: ApplicationFlags.FLAGS_NONE);
        // Initialize item lists
        game_launchers = new HashTable<string, string>(str_hash, str_equal);
        game_launchers.insert("Steam", "Install Steam via Flatpak.");
        game_launchers.insert("GOG", "Install GOG with Proton isolation.");
        game_launchers.insert("Battle.net", "Install Battle.net with Proton isolation.");
        game_launchers.insert("Epic Games Store", "Install Epic Games Store with Proton isolation.");
        game_launchers.insert("EA App", "Install EA App with Proton isolation.");
        pentest_tools = new HashTable<string, string>(str_hash, str_equal);
        pentest_tools.insert("nmap", "Network scanner.");
        pentest_tools.insert("metasploit", "Exploitation framework.");
        pentest_tools.insert("wireshark", "Packet analyzer.");
        pentest_tools.insert("john", "Password cracker.");
        pentest_tools.insert("hydra", "Brute-force tool.");
        pentest_tools.insert("burpsuite", "Web vulnerability scanner.");
        pentest_tools.insert("sqlmap", "SQL injection tool.");
        pentest_tools.insert("nikto", "Web server scanner.");
        pentest_tools.insert("aircrack-ng", "WiFi security auditing suite.");
        pentest_tools.insert("hashcat", "Advanced password recovery tool.");
        pentest_tools.insert("bettercap", "MITM framework for network attacks.");
        pentest_tools.insert("theharvester", "OSINT tool for gathering emails and subdomains.");
        pentest_tools.insert("maltego", "Intelligence and forensics tool.");
        pentest_tools.insert("zaproxy", "Web app security scanner.");
        pentest_tools.insert("dirbuster", "Directory and file brute-forcer.");
        pentest_tools.insert("enum4linux", "SMB enumeration tool.");
        pentest_tools.insert("gobuster", "Directory/file, DNS and VHost busting tool.");
        pentest_tools.insert("responder", "LLMNR/NBT-NS/mDNS poisoner.");
        pentest_tools.insert("impacket", "Collection of Python classes for working with network protocols.");
        pentest_tools.insert("crackmapexec", "Swiss army knife for pentesting networks.");
        pentest_tools.insert("recon-ng", "Web reconnaissance framework.");
        pentest_tools.insert("set", "Social-Engineer Toolkit.");
        pentest_tools.insert("beef-xss", "Browser Exploitation Framework.");
        pentest_tools.insert("volatility", "Memory forensics framework.");
        pentest_tools.insert("autopsy", "Digital forensics platform.");
        pentest_tools.insert("fierce", "DNS reconnaissance tool.");
        pentest_tools.insert("dnsrecon", "DNS enumeration script.");
        pentest_tools.insert("lbd", "Load Balancing Detector.");
        pentest_tools.insert("knock", "Subdomain scan.");
        pentest_tools.insert("arp-scan", "ARP scanning and fingerprinting tool.");
        pentest_tools.insert("masscan", "Mass IP port scanner.");
        pentest_tools.insert("sslscan", "SSL/TLS scanner.");
        pentest_tools.insert("sslyze", "SSL configuration scanner.");
        pentest_tools.insert("arachni", "Web application security scanner.");
        pentest_tools.insert("wpscan", "WordPress vulnerability scanner.");
        pentest_tools.insert("joomscan", "Joomla vulnerability scanner.");
        pentest_tools.insert("cmsmap", "CMS scanner.");
        pentest_tools.insert("droopescan", "Drupal scanner.");
        pentest_tools.insert("openvas", "Open Vulnerability Assessment System.");
        pentest_tools.insert("empire", "Post-exploitation framework.");
        pentest_tools.insert("covenant", ".NET command and control framework.");
        pentest_tools.insert("bloodhound", "Active Directory attack graph tool.");
        pentest_tools.insert("evil-winrm", "WinRM shell for hacking/pentesting.");
        pentest_tools.insert("chisel", "Fast TCP/UDP tunnel over HTTP.");
        pentest_tools.insert("socat", "Multipurpose relay.");
        pentest_tools.insert("netcat", "Network utility for reading/writing across networks.");
        pentest_tools.insert("tcpdump", "Packet analyzer.");
        pentest_tools.insert("hping3", "Network tool for sending custom packets.");
        pentest_tools.insert("scapy", "Packet manipulation library.");
        applications = new HashTable<string, string>(str_hash, str_equal);
        applications.insert("Firefox", "Web browser.");
        applications.insert("VSCode", "Code editor.");
        applications.insert("LibreOffice", "Office suite.");
        applications.insert("Thunderbird", "Email client.");
        applications.insert("GIMP", "Image editor.");
        applications.insert("Inkscape", "Vector graphics editor.");
        applications.insert("Blender", "3D creation suite.");
        applications.insert("VLC", "Media player.");
        applications.insert("Audacity", "Audio editor.");
        applications.insert("FileZilla", "FTP client.");
        applications.insert("VirtualBox", "Virtualization software.");
        applications.insert("Tor Browser", "Privacy-focused browser.");
        applications.insert("KeePassXC", "Password manager.");
        applications.insert("Wireshark", "Network protocol analyzer (standalone).");
        drivers = new HashTable<string, string>(str_hash, str_equal);
        drivers.insert("NVIDIA Driver", "Install NVIDIA graphics driver.");
        drivers.insert("AMD Driver", "Install AMD graphics driver.");
        drivers.insert("WiFi Drivers", "Install common WiFi drivers.");
    }
    protected override void activate() {
        // Prefer dark theme
        var settings = Gtk.Settings.get_default();
        if (settings != null) {
            settings.gtk_application_prefer_dark_theme = true;
        }
        // CSS for custom styling
        var display = Gdk.Display.get_default();
        var css_provider = new Gtk.CssProvider();
        css_provider.load_from_data("""
        window {
        background-color: #1e1e1e;
        color: #ffffff;
    }
    .navigation-sidebar {
    background-color: #2d2d2d;
    border-right: 1px solid #3d3d3d;
    }
    .content {
    background-color: #1e1e1e;
    }
    .item-name {
    font-weight: bold;
    font-size: 1.2em;
    }
    .item-desc {
    color: #aaaaaa;
    }
    button.install {
    background-color: #0d8;
    color: #000000;
    border-radius: 5px;
    padding: 6px 12px;
    }
    button.install:hover {
    background-color: #0ea;
    }
    """.data);
        if (display != null) {
            Gtk.StyleContext.add_provider_for_display(display, css_provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
        window = new Gtk.Window() {
            title = "HackerOS Store",
            default_width = 800,
            default_height = 600
        };
        window.set_application(this);
        // Use HeaderBar for modern look
        var header_bar = new Gtk.HeaderBar();
        header_bar.set_show_title_buttons(true);
        header_bar.set_title_widget(new Gtk.Label("HackerOS Store"));
        window.set_titlebar(header_bar);
        main_box = new Gtk.Box(Orientation.HORIZONTAL, 0);
        window.set_child(main_box);
        // Sidebar for categories
        var sidebar = new Gtk.Box(Orientation.VERTICAL, 10) {
            margin_top = 10,
            margin_bottom = 10,
            margin_start = 10,
            margin_end = 10
        };
        var category_label = new Gtk.Label("Categories") {
            halign = Align.START,
            css_classes = {"title-3"}
        };
        sidebar.append(category_label);
        category_list = new Gtk.ListBox() {
            selection_mode = SelectionMode.SINGLE,
            css_classes = {"navigation-sidebar"}
        };
        category_list.row_selected.connect(on_category_selected);
        sidebar.append(category_list);
        foreach (string category in CATEGORIES) {
            var row = new Gtk.ListBoxRow();
            var hbox = new Gtk.Box(Orientation.HORIZONTAL, 5);
            var icon = new Gtk.Image.from_icon_name(get_category_icon(category));
            icon.set_pixel_size(24);
            hbox.append(icon);
            var label = new Gtk.Label(category) { margin_start = 5, margin_end = 10 };
            hbox.append(label);
            row.set_child(hbox);
            category_list.append(row);
        }
        main_box.append(sidebar);
        // Stack for content
        stack = new Gtk.Stack() {
            vexpand = true,
            hexpand = true,
            margin_top = 10,
            margin_bottom = 10,
            margin_start = 10,
            margin_end = 10
        };
        main_box.append(stack);
        // Create pages for each category
        foreach (string category in CATEGORIES) {
            var scrolled = new Gtk.ScrolledWindow();
            var listbox = new Gtk.ListBox() {
                selection_mode = SelectionMode.NONE,
                css_classes = {"content"}
            };
            scrolled.set_child(listbox);
            stack.add_named(scrolled, category);
            HashTable<string, string> items = get_items_for_category(category);
            if (items != null) {
                items.foreach((name, desc) => {
                    var row = create_item_row(name, desc, category);
                    listbox.append(row);
                });
            }
        }
        window.present();
    }
    private string get_category_icon(string category) {
        switch (category) {
            case "Game Launchers": return "input-gaming-symbolic";
            case "Pentest Tools": return "security-high-symbolic";
            case "Applications": return "applications-system-symbolic";
            case "Drivers/Hardware": return "drive-harddisk-symbolic";
            default: return "folder-symbolic";
        }
    }
    private HashTable<string, string>? get_items_for_category(string category) {
        switch (category) {
            case "Game Launchers": return game_launchers;
            case "Pentest Tools": return pentest_tools;
            case "Applications": return applications;
            case "Drivers/Hardware": return drivers;
            default: return null;
        }
    }
    private Gtk.ListBoxRow create_item_row(string name, string desc, string category) {
        var row = new Gtk.ListBoxRow();
        var main_box = new Gtk.Box(Orientation.HORIZONTAL, 10) {
            margin_top = 10,
            margin_bottom = 10,
            margin_start = 10,
            margin_end = 10
        };
        // Optional icon for item
        var icon = new Gtk.Image.from_icon_name(get_item_icon(name, category));
        icon.set_pixel_size(48);
        main_box.append(icon);
        var text_box = new Gtk.Box(Orientation.VERTICAL, 5) {
            hexpand = true
        };
        var name_label = new Gtk.Label(name) {
            halign = Align.START,
            css_classes = {"item-name"}
        };
        var desc_label = new Gtk.Label(desc) {
            halign = Align.START,
            wrap = true,
            css_classes = {"item-desc"}
        };
        text_box.append(name_label);
        text_box.append(desc_label);
        main_box.append(text_box);
        var install_button = new Gtk.Button.with_label("Install") {
            valign = Align.CENTER,
            css_classes = {"install"}
        };
        install_button.clicked.connect(() => on_install_clicked(name, category));
        main_box.append(install_button);
        row.set_child(main_box);
        return row;
    }
    private string get_item_icon(string name, string category) {
        // Simple mapping, can be expanded
        if (category == "Game Launchers") {
            return "input-gaming-symbolic";
        } else if (category == "Pentest Tools") {
            return "security-medium-symbolic";
        } else if (category == "Applications") {
            return "application-x-executable-symbolic";
        } else if (category == "Drivers/Hardware") {
            return "drive-harddisk-symbolic";
        }
        return "application-x-addon-symbolic";
    }
    private void on_category_selected(Gtk.ListBoxRow? row) {
        if (row != null) {
            var hbox = row.get_child() as Gtk.Box;
            if (hbox != null) {
                var label = hbox.get_last_child() as Gtk.Label;
                if (label != null) {
                    stack.set_visible_child_name(label.get_text());
                }
            }
        }
    }
    private void on_install_clicked(string name, string category) {
        install_async.begin (name, category, (obj, res) => {
            string message;
            try {
                install_async.end (res);
                message = name + " installed successfully.";
            } catch (HackerError e) {
                message = "Error installing " + name + ": " + e.message;
            } catch (Error e) {
                message = "Error: " + e.message;
            }
            show_message_dialog(message);
        });
    }
    private async void install_async (string name, string category) throws Error {
        switch (category) {
            case "Game Launchers":
                yield install_game_launcher_async (name);
                break;
            case "Pentest Tools":
                install_pentest_tool (name);
                break;
            case "Applications":
                install_application (name);
                break;
            case "Drivers/Hardware":
                install_driver (name);
                break;
            default:
                throw new HackerError.NOT_SUPPORTED ("Unknown category.");
        }
    }
    private void show_message_dialog(string message) {
        // Using Gtk.AlertDialog instead of deprecated MessageDialog
        var dialog = new Gtk.AlertDialog (message);
        dialog.set_buttons({"OK"});
        dialog.choose.begin(window, null, (obj, res) => {
            try {
                dialog.choose.end(res);
            } catch (Error e) {
                // Ignore
            }
        });
    }
    // Installation methods
    private async void install_game_launcher_async(string name) throws HackerError {
        switch (name) {
            case "Steam":
                install_steam ();
                break;
            case "GOG":
                yield install_with_proton_isolation_async("GOG", "gog");
                break;
            case "Battle.net":
                yield install_with_proton_isolation_async("Battle.net", "battlenet");
                break;
            case "Epic Games Store":
                yield install_with_proton_isolation_async("Epic Games Store", "epic");
                break;
            case "EA App":
                yield install_with_proton_isolation_async("EA App", "ea");
                break;
            default:
                throw new HackerError.NOT_SUPPORTED("Unsupported launcher");
        }
    }
    private void install_steam() throws HackerError {
        // Check if Flatpak is configured, add Flathub if not, install Steam
        string out_str = "";
        string err_str = "";
        int status = 0;
        bool success = false;
        try {
            success = Process.spawn_command_line_sync("flatpak remote-list", out out_str, out err_str, out status);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to check flatpak remotes: " + e.message);
        }
        if (success && !out_str.contains("flathub")) {
            try {
                success = Process.spawn_command_line_sync("flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo", null, null, out status);
            } catch (SpawnError e) {
                throw new HackerError.FAILED("Failed to add Flathub: " + e.message);
            }
            if (!success || status != 0) throw new HackerError.FAILED("Failed to add Flathub");
        }
        try {
            success = Process.spawn_command_line_sync("flatpak install -y flathub com.valvesoftware.Steam", null, null, out status);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to install Steam: " + e.message);
        }
        if (!success || status != 0) throw new HackerError.FAILED("Failed to install Steam");
    }
    private async void install_with_proton_isolation_async(string name, string id) throws HackerError {
        // Install Proton-GE if not present
        yield install_proton_ge_async ();
        var home = Environment.get_home_dir();
        var proton_dir = Path.build_filename(home, ".hackeros", "proton");
        var proton_path = Path.build_filename(proton_dir, "proton");
        var launchers_dir = Path.build_filename(home, ".hackeros", "launchers", id);
        var prefix = Path.build_filename(launchers_dir, "prefix");
        DirUtils.create_with_parents(prefix, 0755);
        // Download installer
        string url = get_launcher_url(id);
        string ext = (id == "epic") ? ".msi" : ".exe";
        var installer_file = Path.build_filename(launchers_dir, "installer" + ext);
        var progress = new ProgressDialog (window, "Downloading " + name + " installer");
        progress.show ();
        try {
            yield download_file_async (url, installer_file, progress);
        } catch (Error e) {
            progress.destroy ();
            throw new HackerError.FAILED("Failed to download installer for " + name + ": " + e.message);
        }
        progress.destroy ();
        // Run installer with Proton
        string run_cmd;
        if (id == "epic") {
            run_cmd = proton_path + " run msiexec /i " + installer_file;
        } else {
            run_cmd = proton_path + " run " + installer_file;
        }
        Environment.set_variable("STEAM_COMPAT_DATA_PATH", prefix, true);
        Environment.set_variable("STEAM_COMPAT_CLIENT_INSTALL_PATH", home + "/.steam/steam", true); // Assume Steam is installed, or dummy
        int status = 0;
        bool success = false;
        try {
            success = Process.spawn_command_line_sync(run_cmd, null, null, out status);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to run installer for " + name + ": " + e.message);
        }
        if (!success || status != 0) throw new HackerError.FAILED("Failed to run installer for " + name + ". Please check if the installer ran correctly.");
        // Get installed exe path
        string exe_path = get_installed_exe_path(id, prefix);
        // Create .desktop file
        var desktop_dir = Path.build_filename(Environment.get_user_data_dir(), "applications");
        DirUtils.create_with_parents(desktop_dir, 0755);
        var desktop_file = Path.build_filename(desktop_dir, id + ".desktop");
        var content = @"[Desktop Entry]\nName=$name\nExec=env STEAM_COMPAT_DATA_PATH=$prefix STEAM_COMPAT_CLIENT_INSTALL_PATH=$(home)/.steam/steam $proton_path run $exe_path\nType=Application\n";
        try {
            File.new_for_path(desktop_file).replace_contents(content.data, null, false, FileCreateFlags.NONE, null);
        } catch (Error e) {
            throw new HackerError.FAILED("Failed to create desktop file: " + e.message);
        }
    }
    private async void install_proton_ge_async() throws HackerError {
        var home = Environment.get_home_dir();
        var proton_dir = Path.build_filename(home, ".hackeros", "proton");
        var proton_path = Path.build_filename(proton_dir, "proton");
        if (File.new_for_path(proton_path).query_exists()) {
            return; // Already installed
        }
        // Get latest Proton-GE URL
        string url_out = "";
        bool success = false;
        try {
            success = Process.spawn_command_line_sync(@"curl -s -H \"Accept: application/vnd.github.v3+json\" -H \"User-Agent: Mozilla/5.0\" https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases/latest | grep \"browser_download_url.*\\.tar\\.gz\" | cut -d : -f 2,3 | tr -d \" | head -n 1", out url_out, null);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to get Proton-GE download URL: " + e.message);
        }
        if (!success) throw new HackerError.FAILED("Failed to get Proton-GE download URL");
        string url = url_out.strip();
        if (url == "") throw new HackerError.FAILED("No Proton-GE URL found");
        var tar_file = "/tmp/proton-ge.tar.gz";
        var progress = new ProgressDialog (window, "Downloading Proton-GE");
        progress.show ();
        try {
            yield download_file_async (url, tar_file, progress);
        } catch (Error e) {
            progress.destroy ();
            throw new HackerError.FAILED("Failed to download Proton-GE: " + e.message);
        }
        progress.destroy ();
        DirUtils.create_with_parents(proton_dir, 0755);
        try {
            success = Process.spawn_command_line_sync(@"tar -xzf $tar_file -C $proton_dir --strip-components=1", null, null);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to extract Proton-GE: " + e.message);
        }
        if (!success) throw new HackerError.FAILED("Failed to extract Proton-GE");
        try {
            FileUtils.remove(tar_file); // Cleanup
        } catch (FileError e) {
            // Ignore cleanup error
        }
    }
    private async void download_file_async (string url, string file, ProgressDialog progress) throws Error {
        Cancellable cancellable = progress.get_cancellable ();
        string[] args = { "wget", "--progress=bar:force:noscroll", "-O", file, url };
        Pid pid;
        int in_fd;
        int out_fd;
        int err_fd;
        try {
            Process.spawn_async_with_pipes (null, args, null, SpawnFlags.SEARCH_PATH, null, out pid, out in_fd, out out_fd, out err_fd);
        } catch (Error e) {
            throw e;
        }
        Posix.close (in_fd);
        Posix.close (out_fd);
        var channel = new IOChannel.unix_new (err_fd);
        GLib.Regex? regex = null;
        try {
            regex = new GLib.Regex ("""(\d+)%""");
        } catch (Error e) {
            Posix.kill (pid, SIGTERM);
            throw e;
        }
        channel.add_watch (IOCondition.IN | IOCondition.HUP, (ch, condition) => {
            if ((condition & IOCondition.HUP) == IOCondition.HUP) {
                return Source.REMOVE;
            }
            try {
                string? line = null;
                ch.read_line (out line, null, null);
                if (line != null) {
                    MatchInfo info;
                    if (regex.match (line, 0, out info)) {
                        string perc_str = info.fetch (1);
                        double fraction = int.parse (perc_str) / 100.0;
                        progress.set_progress (fraction);
                    }
                }
            } catch (Error e) {
            }
            return Source.CONTINUE;
        });
        cancellable.cancelled.connect (() => {
            Posix.kill (pid, SIGTERM);
        });
        int status = -1;
        ChildWatch.add (pid, (p, s) => {
            status = s;
            Process.close_pid (p);
            download_file_async.callback ();
        });
        yield;
        if (cancellable.is_cancelled ()) {
            throw new IOError.CANCELLED ("Download cancelled");
        }
        if (Process.exit_status (status) != 0) {
            throw new HackerError.FAILED ("Download failed");
        }
    }
    private string get_launcher_url(string id) {
        switch (id) {
            case "gog": return "https://webinstallers.gog.com/galaxy_installer_en.exe";
            case "battlenet": return "https://www.battle.net/download/getInstaller?os=win&installer=Battle.net-Setup.exe";
            case "epic": return "https://launcher-public-service-prod06.ol.epicgames.com/launcher/api/installer/download/EpicGamesLauncherInstaller.msi";
            case "ea": return "https://origin-a.akamaihd.net/EA-Desktop-Client-Download/installer-releases/EAappInstaller.exe";
            default: return "";
        }
    }
    private string get_installed_exe_path(string id, string prefix) {
        string drive_c = Path.build_filename(prefix, "drive_c");
        switch (id) {
            case "gog": return Path.build_filename(drive_c, "Program Files (x86)", "GOG Galaxy", "GalaxyClient.exe");
            case "battlenet": return Path.build_filename(drive_c, "Program Files (x86)", "Battle.net", "Battle.net.exe");
            case "epic": return Path.build_filename(drive_c, "Program Files (x86)", "Epic Games", "Launcher", "Portal", "Binaries", "Win32", "EpicGamesLauncher.exe");
            case "ea": return Path.build_filename(drive_c, "Program Files", "Electronic Arts", "EA Desktop", "EADesktop.exe");
            default: return "";
        }
    }
    private void install_pentest_tool(string name) throws HackerError {
        // Install Distrobox if not present
        int status = 0;
        bool success = false;
        try {
            success = Process.spawn_command_line_sync("distrobox --version", null, null, out status);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to check Distrobox: " + e.message);
        }
        if (!success || status != 0) {
            // Install Distrobox
            try {
                success = Process.spawn_command_line_sync("curl -s https://raw.githubusercontent.com/89luca89/distrobox/main/install | sudo sh", null, null, out status);
            } catch (SpawnError e) {
                throw new HackerError.FAILED("Failed to install Distrobox: " + e.message);
            }
            if (!success || status != 0) throw new HackerError.FAILED("Failed to install Distrobox");
        }
        // Create BlackArch container if not exists
        try {
            success = Process.spawn_command_line_sync("distrobox create --image blackarchlinux/blackarch --name blackarch-pentest", null, null, out status);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to create BlackArch container: " + e.message);
        }
        if (!success || (status != 0 && status != 1)) { // 1 might mean already exists
            throw new HackerError.FAILED("Failed to create BlackArch container");
        }
        // Install tool inside container
        try {
            success = Process.spawn_command_line_sync(@"distrobox enter blackarch-pentest -- sudo pacman -Syu --noconfirm $name", null, null, out status);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to install " + name + ": " + e.message);
        }
        if (!success || status != 0) throw new HackerError.FAILED("Failed to install " + name);
        // Create wrapper: a script that runs distrobox enter blackarch-pentest -- name
        var wrapper_path = Path.build_filename(Environment.get_home_dir(), ".local", "bin", name + "-pentest");
        DirUtils.create_with_parents(Path.get_dirname(wrapper_path), 0755);
        var wrapper_content = "#!/bin/sh\ndistrobox enter blackarch-pentest -- " + name + " \"$@\"\n";
        try {
            File.new_for_path(wrapper_path).replace_contents(wrapper_content.data, null, false, FileCreateFlags.NONE, null);
        } catch (Error e) {
            throw new HackerError.FAILED("Failed to create wrapper: " + e.message);
        }
        try {
            success = Process.spawn_command_line_sync(@"chmod 0755 $wrapper_path", null, null, out status);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to make wrapper executable: " + e.message);
        }
        if (!success || status != 0) throw new HackerError.FAILED("Failed to make wrapper executable");
        // Create .desktop if needed
        var desktop_file = Path.build_filename(Environment.get_user_data_dir(), "applications", name + "-pentest.desktop");
        var desktop_content = "[Desktop Entry]\nName=" + name + " (Pentest)\nExec=" + wrapper_path + "\nType=Application\n";
        try {
            File.new_for_path(desktop_file).replace_contents(desktop_content.data, null, false, FileCreateFlags.NONE, null);
        } catch (Error e) {
            throw new HackerError.FAILED("Failed to create desktop file: " + e.message);
        }
    }
    private void install_application(string name) throws HackerError {
        // Placeholder: install via flatpak or apt, assuming flatpak for cross-distro
        string flatpak_id;
        switch (name) {
            case "Firefox": flatpak_id = "org.mozilla.firefox"; break;
            case "VSCode": flatpak_id = "com.visualstudio.code"; break;
            case "LibreOffice": flatpak_id = "org.libreoffice.LibreOffice"; break;
            case "Thunderbird": flatpak_id = "org.mozilla.Thunderbird"; break;
            case "GIMP": flatpak_id = "org.gimp.GIMP"; break;
            case "Inkscape": flatpak_id = "org.inkscape.Inkscape"; break;
            case "Blender": flatpak_id = "org.blender.Blender"; break;
            case "VLC": flatpak_id = "org.videolan.VLC"; break;
            case "Audacity": flatpak_id = "org.audacityteam.Audacity"; break;
            case "FileZilla": flatpak_id = "org.filezillaproject.Filezilla"; break;
            case "VirtualBox": flatpak_id = "org.virtualbox.VirtualBox"; break;
            case "Tor Browser": flatpak_id = "com.github.micahflee.torbrowser-launcher"; break;
            case "KeePassXC": flatpak_id = "org.keepassxc.KeePassXC"; break;
            case "Wireshark": flatpak_id = "org.wireshark.Wireshark"; break;
            default: throw new HackerError.NOT_SUPPORTED("Unsupported application");
        }
        int status = 0;
        bool success = false;
        try {
            success = Process.spawn_command_line_sync(@"flatpak install -y flathub $flatpak_id", null, null, out status);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to install " + name + ": " + e.message);
        }
        if (!success || status != 0) throw new HackerError.FAILED("Failed to install " + name);
    }
    private void install_driver(string name) throws HackerError {
        // Placeholder: system-specific driver installation, e.g., via apt or dnf
        // Assuming Ubuntu-like for example
        string pkg;
        switch (name) {
            case "NVIDIA Driver": pkg = "nvidia-driver nvidia-kernel-dkms nvidia-smi libnvidia-ml1 nvidia-settings nvidia-cuda-mps"; break;
            case "AMD Driver": pkg = "amdgpu"; break;
            case "WiFi Drivers": pkg = "broadcom-wl"; break; // Example
            default: throw new HackerError.NOT_SUPPORTED("Unsupported driver");
        }
        int status = 0;
        bool success = false;
        try {
            success = Process.spawn_command_line_sync(@"sudo apt install -y $pkg", null, null, out status);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to install " + name + ": " + e.message);
        }
        if (!success || status != 0) throw new HackerError.FAILED("Failed to install " + name);
    }
    public static int main(string[] args) {
        return new HackerOSStore().run(args);
    }
}
