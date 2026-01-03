// HackerOS Store - A simple store application for installing games launchers, pentest tools, applications, and drivers/hardware.
// Inspired by RagataOS, but tailored for HackerOS.
// Written in Vala with GTK4.
// This application provides a GUI to install various items, with pentest tools installed in a Distrobox container (BlackArch) and wrappers created.

using Gtk;
using GLib;

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

        var dialog = new Gtk.MessageDialog(window, DialogFlags.MODAL, MessageType.INFO, ButtonsType.OK, message);
        dialog.response.connect((response_id) => dialog.destroy());
        dialog.present();
    }

    // Installation methods

    private void install_game_launcher(string name) throws Error {
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
                throw new Error.literal(ErrorCode.NOT_SUPPORTED, "Unsupported launcher");
        }
    }

    private void install_steam() throws Error {
        // Check if Flatpak is configured, add Flathub if not, install Steam
        string[] check_flatpak = { "flatpak", "remote-list" };
        int status;
        string out_str, err_str;
        Process.spawn_sync(null, check_flatpak, null, SpawnFlags.SEARCH_PATH, null, out out_str, out err_str, out status);

        if (!out_str.contains("flathub")) {
            string[] add_flathub = { "flatpak", "remote-add", "--if-not-exists", "flathub", "https://dl.flathub.org/repo/flathub.flatpakrepo" };
            Process.spawn_sync(null, add_flathub, null, SpawnFlags.SEARCH_PATH, null, null, null, out status);
            if (status != 0) throw new Error.literal(ErrorCode.FAILED, "Failed to add Flathub");
        }

        string[] install_steam = { "flatpak", "install", "-y", "flathub", "com.valvesoftware.Steam" };
        Process.spawn_sync(null, install_steam, null, SpawnFlags.SEARCH_PATH, null, null, null, out status);
        if (status != 0) throw new Error.literal(ErrorCode.FAILED, "Failed to install Steam");
    }

    private void install_with_proton_isolation(string name, string id) throws Error {
        // Assuming Proton is installed or install it
        // Create isolated environment in ~/.hackeros/launchers/
        var home = Environment.get_home_dir();
        var launchers_dir = Path.build_filename(home, ".hackeros", "launchers", id);
        DirUtils.create_with_parents(launchers_dir, 0755);

        // Install Proton (example: use ge-proton or something; assuming a script or command)
        // For simplicity, assume downloading and setting up Proton
        // Then install the launcher (assuming it's a downloadable executable; in reality, you'd need specific commands)
        // This is placeholder; in real code, use actual installation commands, e.g., via wget or package manager

        // Create .desktop file
        var desktop_file = Path.build_filename(Environment.get_user_data_dir(), "applications", id + ".desktop");
        var content = "[Desktop Entry]\nName=" + name + "\nExec=proton run " + launchers_dir + "/launcher.exe\nType=Application\n";
        File.new_for_path(desktop_file).replace_contents(content.data, null, false, FileCreateFlags.NONE, null, null);
    }

    private void install_pentest_tool(string name) throws Error {
        // Install Distrobox if not present
        string[] check_distrobox = { "distrobox", "--version" };
        int status;
        Process.spawn_sync(null, check_distrobox, null, SpawnFlags.SEARCH_PATH | SpawnFlags.DO_NOT_REAP_CHILD, null, null, null, out status);
        if (status != 0) {
            // Install Distrobox (assuming via curl or something; placeholder)
            string[] install_distrobox = { "curl", "-s", "https://raw.githubusercontent.com/89luca89/distrobox/main/install", "|", "sudo", "sh" };
            Process.spawn_sync(null, install_distrobox, null, SpawnFlags.SEARCH_PATH, null, null, null, out status);
            if (status != 0) throw new Error.literal(ErrorCode.FAILED, "Failed to install Distrobox");
        }

        // Create BlackArch container if not exists
        string[] create_container = { "distrobox", "create", "--image", "blackarchlinux/blackarch", "--name", "blackarch-pentest" };
        Process.spawn_sync(null, create_container, null, SpawnFlags.SEARCH_PATH, null, null, null, out status);
        if (status != 0 && status != 1) { // 1 might mean already exists
            throw new Error.literal(ErrorCode.FAILED, "Failed to create BlackArch container");
        }

        // Install tool inside container
        string[] install_tool = { "distrobox", "enter", "blackarch-pentest", "--", "sudo", "pacman", "-Syu", "--noconfirm", name };
        Process.spawn_sync(null, install_tool, null, SpawnFlags.SEARCH_PATH, null, null, null, out status);
        if (status != 0) throw new Error.literal(ErrorCode.FAILED, "Failed to install " + name);

        // Create wrapper: a script that runs distrobox enter blackarch-pentest -- name
        var wrapper_path = Path.build_filename(Environment.get_home_dir(), ".local", "bin", name + "-pentest");
        var wrapper_content = "#!/bin/sh\ndistrobox enter blackarch-pentest -- " + name + " \"$@\"\n";
        File.new_for_path(wrapper_path).replace_contents(wrapper_content.data, null, false, FileCreateFlags.NONE, null, null);
        Posix.chmod(wrapper_path, 0755);

        // Create .desktop if needed
        var desktop_file = Path.build_filename(Environment.get_user_data_dir(), "applications", name + "-pentest.desktop");
        var desktop_content = "[Desktop Entry]\nName=" + name + " (Pentest)\nExec=" + wrapper_path + "\nType=Application\n";
        File.new_for_path(desktop_file).replace_contents(desktop_content.data, null, false, FileCreateFlags.NONE, null, null);
    }

    private void install_application(string name) throws Error {
        // Placeholder: install via flatpak or apt, assuming flatpak for cross-distro
        string flatpak_id;
        switch (name) {
            case "Firefox": flatpak_id = "org.mozilla.firefox"; break;
            case "VSCode": flatpak_id = "com.visualstudio.code"; break;
            case "LibreOffice": flatpak_id = "org.libreoffice.LibreOffice"; break;
            default: throw new Error.literal(ErrorCode.NOT_SUPPORTED, "Unsupported application");
        }
        string[] install_cmd = { "flatpak", "install", "-y", "flathub", flatpak_id };
        int status;
        Process.spawn_sync(null, install_cmd, null, SpawnFlags.SEARCH_PATH, null, null, null, out status);
        if (status != 0) throw new Error.literal(ErrorCode.FAILED, "Failed to install " + name);
    }

    private void install_driver(string name) throws Error {
        // Placeholder: system-specific driver installation, e.g., via apt or dnf
        // Assuming Ubuntu-like for example
        string pkg;
        switch (name) {
            case "NVIDIA Driver": pkg = "nvidia-driver"; break;
            case "AMD Driver": pkg = "amdgpu"; break;
            case "WiFi Drivers": pkg = "broadcom-wl"; break; // Example
            default: throw new Error.literal(ErrorCode.NOT_SUPPORTED, "Unsupported driver");
        }
        string[] install_cmd = { "sudo", "apt", "install", "-y", pkg };
        int status;
        Process.spawn_sync(null, install_cmd, null, SpawnFlags.SEARCH_PATH, null, null, null, out status);
        if (status != 0) throw new Error.literal(ErrorCode.FAILED, "Failed to install " + name);
    }

    public static int main(string[] args) {
        return new HackerOSStore().run(args);
    }
}
