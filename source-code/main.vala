// HackerOS Store - A simple store application for installing games launchers, pentest tools, applications, and drivers/hardware.
// Inspired by RagataOS, but tailored for HackerOS.
// Written in Vala with GTK4.
// This application provides a GUI to install various items, with pentest tools installed in a Distrobox container (BlackArch) and wrappers created.

using Gtk;
using GLib;

errordomain HackerError {
    FAILED,
    NOT_SUPPORTED
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
        pentest_tools.insert("Responder", "LLMNR/NBT-NS/mDNS poisoner.");
        pentest_tools.insert("impacket", "Collection of Python classes for working with network protocols.");
        pentest_tools.insert("crackmapexec", "Swiss army knife for pentesting networks.");

        applications = new HashTable<string, string>(str_hash, str_equal);
        applications.insert("Firefox", "Web browser.");
        applications.insert("VSCode", "Code editor.");
        applications.insert("LibreOffice", "Office suite.");

        drivers = new HashTable<string, string>(str_hash, str_equal);
        drivers.insert("NVIDIA Driver", "Install NVIDIA graphics driver.");
        drivers.insert("AMD Driver", "Install AMD graphics driver.");
        drivers.insert("WiFi Drivers", "Install common WiFi drivers.");
    }

    protected override void activate() {
        window = new Gtk.Window() {
            title = "HackerOS Store",
            default_width = 800,
            default_height = 600
        };
        window.set_application(this);

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
            var label = new Gtk.Label(category) { margin_start = 10, margin_end = 10 };
            row.set_child(label);
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
                selection_mode = SelectionMode.NONE
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
        var box = new Gtk.Box(Orientation.HORIZONTAL, 10) {
            margin_top = 5,
            margin_bottom = 5,
            margin_start = 10,
            margin_end = 10
        };
        var label = new Gtk.Label(name + ": " + desc) {
            hexpand = true,
            halign = Align.START
        };
        var install_button = new Gtk.Button.with_label("Install");
        install_button.clicked.connect(() => on_install_clicked(name, category));
        box.append(label);
        box.append(install_button);
        row.set_child(box);
        return row;
    }

    private void on_category_selected(Gtk.ListBoxRow? row) {
        if (row != null) {
            var label = row.get_child() as Gtk.Label;
            if (label != null) {
                stack.set_visible_child_name(label.get_text());
            }
        }
    }

    private void on_install_clicked(string name, string category) {
        string message;
        try {
            switch (category) {
                case "Game Launchers":
                    install_game_launcher(name);
                    message = name + " installed successfully.";
                    break;
                case "Pentest Tools":
                    install_pentest_tool(name);
                    message = name + " installed in Distrobox (BlackArch) with wrapper.";
                    break;
                case "Applications":
                    install_application(name);
                    message = name + " installed successfully.";
                    break;
                case "Drivers/Hardware":
                    install_driver(name);
                    message = name + " installed successfully.";
                    break;
                default:
                    message = "Unknown category.";
                    break;
            }
        } catch (Error e) {
            message = "Error installing " + name + ": " + e.message;
        }

        show_message_dialog(message);
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

    private void install_game_launcher(string name) throws HackerError {
        switch (name) {
            case "Steam":
                install_steam();
                break;
            case "GOG":
                install_with_proton_isolation("GOG", "gog");
                break;
            case "Battle.net":
                install_with_proton_isolation("Battle.net", "battlenet");
                break;
            case "Epic Games Store":
                install_with_proton_isolation("Epic Games Store", "epic");
                break;
            case "EA App":
                install_with_proton_isolation("EA App", "ea");
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

    private void install_with_proton_isolation(string name, string id) throws HackerError {
        // Install Proton-GE if not present
        install_proton_ge();

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
        bool success = false;
        try {
            success = Process.spawn_command_line_sync(@"wget -O $installer_file $url", null, null);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to download installer for " + name + ": " + e.message);
        }
        if (!success) throw new HackerError.FAILED("Failed to download installer for " + name);

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

    private void install_proton_ge() throws HackerError {
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
            success = Process.spawn_command_line_sync("curl -s https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases/latest | grep \"browser_download_url.*\\.tar\\.gz\" | cut -d : -f 2,3 | tr -d \\\" ", out url_out, null);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to get Proton-GE download URL: " + e.message);
        }
        if (!success) throw new HackerError.FAILED("Failed to get Proton-GE download URL");
        string url = url_out.strip();

        if (url == "") throw new HackerError.FAILED("No Proton-GE URL found");

        var tar_file = "/tmp/proton-ge.tar.gz";
        try {
            success = Process.spawn_command_line_sync(@"wget -O $tar_file $url", null, null);
        } catch (SpawnError e) {
            throw new HackerError.FAILED("Failed to download Proton-GE: " + e.message);
        }
        if (!success) throw new HackerError.FAILED("Failed to download Proton-GE");

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
            case "NVIDIA Driver": pkg = "nvidia-driver"; break;
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
