export type Category = "game_launchers" | "pentest_tools" | "drivers" | "hackeros_ecosystem" | "dev_tools" | "update" | "discover" | "settings" | "history" | "nix";
export interface Package {
  name: string; desc: string; category: Category; icon: string; tags?: string[];
  /** HackerOS Ecosystem only: false means `hacker` offers no way to remove
   * this tool once unpacked (currently just Hydra) — the row shows a
   * warning instead of an uninstall button. Defaults to true (removable)
   * for every other category/entry. */
  uninstallable?: boolean;
}

export const GAME_LAUNCHERS: Package[] = [
  { name:"Steam",            desc:"The largest PC gaming platform. Flatpak.",      category:"game_launchers", icon:"Gamepad2", tags:["flatpak","gaming"] },
  { name:"Lutris",           desc:"All-in-one gaming platform for Linux.",          category:"game_launchers", icon:"Gamepad2", tags:["flatpak","gaming"] },
  { name:"Heroic",           desc:"Open-source Epic Games & GOG launcher.",         category:"game_launchers", icon:"Rocket", tags:["flatpak","epic","gog"] },
  { name:"Epic Games Store", desc:"Epic Games via Heroic Launcher.",                category:"game_launchers", icon:"Gamepad2", tags:["flatpak","epic"] },
  { name:"Bottles",          desc:"Run Windows apps & games via Wine prefixes.",    category:"game_launchers", icon:"Wine", tags:["flatpak","wine"] },
  { name:"GOG",              desc:"GOG Galaxy client via Wine (DRM-free games).",   category:"game_launchers", icon:"Disc3", tags:["wine","gog"] },
  { name:"Battle.net",       desc:"Blizzard launcher via Wine (WoW, OW2, Diablo).",category:"game_launchers", icon:"Swords", tags:["wine","blizzard"] },
  { name:"EA App",           desc:"EA launcher via Wine (FIFA, Battlefield…).",    category:"game_launchers", icon:"Crosshair", tags:["wine","ea"] },
];

// Mirrors PENTEST_CATALOG in src-tauri/src/lib.rs exactly — same tool
// names in the same order, so the frontend catalog and the Rust
// install-strategy/installed-state catalog can never drift apart again.
// If you add a tool here, add the matching (name, in_debian) row there too.
export const PENTEST_TOOLS: Package[] = [
  // ── Network / recon ──
  { name:"nmap",          desc:"Network scanner and host discovery tool.",              category:"pentest_tools", icon:"Radar", tags:["network","recon"] },
  { name:"masscan",       desc:"Mass IP port scanner — fastest on earth.",             category:"pentest_tools", icon:"Radar", tags:["network","recon"] },
  { name:"arp-scan",      desc:"ARP scanning and local network fingerprinting.",       category:"pentest_tools", icon:"Radar", tags:["network","recon"] },
  { name:"netdiscover",   desc:"Active/passive ARP network scanner.",                  category:"pentest_tools", icon:"Radar", tags:["network","recon"] },
  { name:"hping3",        desc:"TCP/IP packet assembler and analyser.",                category:"pentest_tools", icon:"Waves", tags:["network"] },
  { name:"netcat",        desc:"Swiss army knife for networking (nc).",                category:"pentest_tools", icon:"Plug", tags:["network","utility"] },
  { name:"ncat",          desc:"Improved netcat from the Nmap project.",               category:"pentest_tools", icon:"Plug", tags:["network","utility"] },
  { name:"socat",         desc:"Multipurpose relay tool for sockets.",                 category:"pentest_tools", icon:"Plug", tags:["network","utility"] },
  { name:"rustscan",      desc:"Extremely fast modern port scanner.",                  category:"pentest_tools", icon:"Radar", tags:["network","recon"] },
  { name:"naabu",         desc:"Fast port scanner written in Go.",                     category:"pentest_tools", icon:"Radar", tags:["network","recon"] },
  { name:"wireshark",     desc:"Graphical packet analyser and protocol inspector.",    category:"pentest_tools", icon:"Fish", tags:["packet","network"] },
  { name:"tcpdump",       desc:"Command-line packet capture and analysis.",            category:"pentest_tools", icon:"Fish", tags:["packet"] },
  { name:"tshark",        desc:"Terminal version of Wireshark.",                       category:"pentest_tools", icon:"Fish", tags:["packet"] },
  { name:"tcpflow",       desc:"TCP flow recorder for protocol analysis.",             category:"pentest_tools", icon:"Fish", tags:["packet"] },
  { name:"scapy",         desc:"Interactive packet manipulation and crafting.",        category:"pentest_tools", icon:"Fish", tags:["packet","python"] },
  { name:"fping",         desc:"Send ICMP echo probes to multiple hosts in parallel.", category:"pentest_tools", icon:"Radar", tags:["network","recon"] },
  { name:"zmap",          desc:"Fast single-packet network scanner for large scans.",  category:"pentest_tools", icon:"Radar", tags:["network","recon"] },
  { name:"unicornscan",   desc:"Asynchronous stateless TCP/UDP port scanner.",         category:"pentest_tools", icon:"Radar", tags:["network","recon"] },
  { name:"dnsenum",       desc:"DNS enumeration and zone transfer testing tool.",       category:"pentest_tools", icon:"Globe", tags:["network","dns"] },
  { name:"fierce",        desc:"Domain DNS reconnaissance and zone-walker.",           category:"pentest_tools", icon:"Globe", tags:["network","dns"] },
  { name:"p0f",           desc:"Passive OS fingerprinting from network traffic.",       category:"pentest_tools", icon:"Fish", tags:["network","recon"] },
  { name:"dmitry",        desc:"Deepmagic information gathering tool.",                category:"pentest_tools", icon:"Telescope", tags:["network","recon"] },
  { name:"nbtscan",       desc:"NetBIOS name scanner for local networks.",             category:"pentest_tools", icon:"Radar", tags:["network","recon"] },
  // ── Web application testing ──
  { name:"burpsuite",     desc:"Web vulnerability scanner and intercepting proxy.",    category:"pentest_tools", icon:"Bug", tags:["web","proxy"] },
  { name:"zaproxy",       desc:"OWASP ZAP web application security scanner.",         category:"pentest_tools", icon:"Bug", tags:["web"] },
  { name:"sqlmap",        desc:"Automatic SQL injection and database takeover.",       category:"pentest_tools", icon:"Database", tags:["web","sql"] },
  { name:"nikto",         desc:"Web server vulnerability and configuration scanner.", category:"pentest_tools", icon:"Bug", tags:["web"] },
  { name:"gobuster",      desc:"Directory, DNS and VHost brute-force tool.",           category:"pentest_tools", icon:"Search", tags:["web","bruteforce"] },
  { name:"wpscan",        desc:"WordPress vulnerability scanner.",                     category:"pentest_tools", icon:"Search", tags:["web","cms"] },
  { name:"beef-xss",      desc:"Browser Exploitation Framework (BeEF).",              category:"pentest_tools", icon:"AppWindow", tags:["web","xss"] },
  { name:"feroxbuster",   desc:"Fast, recursive content discovery tool.",              category:"pentest_tools", icon:"Search", tags:["web","bruteforce"] },
  { name:"ffuf",          desc:"Fast web fuzzer written in Go.",                       category:"pentest_tools", icon:"Search", tags:["web","fuzzing"] },
  { name:"nuclei",        desc:"Fast, template-based vulnerability scanner.",         category:"pentest_tools", icon:"Search", tags:["web","vuln"] },
  { name:"httpx",         desc:"Fast, multi-purpose HTTP probing toolkit.",           category:"pentest_tools", icon:"Globe", tags:["web","recon"] },
  { name:"katana",        desc:"Next-gen crawling and spidering framework.",           category:"pentest_tools", icon:"Globe", tags:["web","recon"] },
  { name:"dirb",          desc:"Web content scanner using wordlists.",                category:"pentest_tools", icon:"Search", tags:["web","bruteforce"] },
  { name:"dirsearch",     desc:"Advanced web path brute-forcer.",                     category:"pentest_tools", icon:"Search", tags:["web","bruteforce"] },
  { name:"whatweb",       desc:"Web technology and CMS fingerprinting tool.",         category:"pentest_tools", icon:"Eye", tags:["web","recon"] },
  { name:"wafw00f",       desc:"Web Application Firewall detection tool.",            category:"pentest_tools", icon:"Shield", tags:["web","recon"] },
  { name:"commix",        desc:"Automated command injection exploitation tool.",       category:"pentest_tools", icon:"Bug", tags:["web","exploit"] },
  { name:"xsser",         desc:"Cross-site scripting (XSS) detection framework.",     category:"pentest_tools", icon:"Bug", tags:["web","xss"] },
  { name:"joomscan",      desc:"Joomla CMS vulnerability scanner.",                   category:"pentest_tools", icon:"Search", tags:["web","cms"] },
  { name:"droopescan",    desc:"Drupal/Silverstripe CMS security scanner.",           category:"pentest_tools", icon:"Search", tags:["web","cms"] },
  { name:"sslyze",        desc:"Fast and comprehensive TLS/SSL configuration scanner.",category:"pentest_tools", icon:"Lock", tags:["ssl","web"] },
  { name:"testssl.sh",    desc:"Command-line tool to test TLS/SSL of any server.",    category:"pentest_tools", icon:"Lock", tags:["ssl","web"] },
  { name:"wfuzz",         desc:"Web application fuzzer for parameters and content.",  category:"pentest_tools", icon:"Search", tags:["web","fuzzing"] },
  { name:"wapiti",        desc:"Black-box web vulnerability scanner.",                category:"pentest_tools", icon:"Bug", tags:["web","vuln"] },
  { name:"skipfish",      desc:"Active web application security reconnaissance tool.", category:"pentest_tools", icon:"Search", tags:["web","recon"] },
  { name:"xsstrike",      desc:"Advanced XSS detection and exploitation suite.",       category:"pentest_tools", icon:"Bug", tags:["web","xss"] },
  { name:"dalfox",        desc:"Fast parameter-based XSS scanner.",                    category:"pentest_tools", icon:"Bug", tags:["web","xss"] },
  // ── Password / credential attacks ──
  { name:"john",          desc:"John the Ripper — classic password cracker.",         category:"pentest_tools", icon:"KeyRound", tags:["password","crack"] },
  { name:"hydra",         desc:"Fast network login brute-force tool.",                category:"pentest_tools", icon:"KeyRound", tags:["password","bruteforce"] },
  { name:"hashcat",       desc:"GPU-accelerated password recovery tool.",              category:"pentest_tools", icon:"KeyRound", tags:["password","crack"] },
  { name:"medusa",        desc:"Parallel network login auditor.",                      category:"pentest_tools", icon:"KeyRound", tags:["password","bruteforce"] },
  { name:"crunch",        desc:"Wordlist generator based on criteria.",                category:"pentest_tools", icon:"FileText", tags:["password","wordlist"] },
  { name:"cewl",          desc:"Custom wordlist generator from website content.",     category:"pentest_tools", icon:"FileText", tags:["password","wordlist"] },
  { name:"patator",       desc:"Multi-purpose, modular brute-forcer.",                category:"pentest_tools", icon:"KeyRound", tags:["password","bruteforce"] },
  { name:"ncrack",        desc:"High-speed network authentication cracker.",           category:"pentest_tools", icon:"KeyRound", tags:["password","bruteforce"] },
  { name:"hashid",        desc:"Identifies hash types for cracking.",                  category:"pentest_tools", icon:"KeyRound", tags:["password"] },
  { name:"ophcrack",      desc:"Windows password cracker using rainbow tables.",       category:"pentest_tools", icon:"KeyRound", tags:["password","crack"] },
  { name:"fcrackzip",     desc:"Brute-force cracker for password-protected ZIP files.", category:"pentest_tools", icon:"KeyRound", tags:["password","crack"] },
  { name:"pdfcrack",      desc:"Password recovery tool for encrypted PDF files.",      category:"pentest_tools", icon:"KeyRound", tags:["password","crack"] },
  { name:"rarcrack",      desc:"Brute-force cracker for RAR/ZIP/7z archives.",         category:"pentest_tools", icon:"KeyRound", tags:["password","crack"] },
  { name:"bruteforce-luks", desc:"Brute-force password recovery for LUKS volumes.",    category:"pentest_tools", icon:"KeyRound", tags:["password","crack"] },
  // ── Wireless ──
  { name:"aircrack-ng",   desc:"802.11 WEP/WPA/WPA2 security auditing suite.",       category:"pentest_tools", icon:"Wifi", tags:["wifi","wireless"] },
  { name:"kismet",        desc:"Wireless network detector, sniffer, and IDS.",        category:"pentest_tools", icon:"Wifi", tags:["wifi","wireless"] },
  { name:"reaver",        desc:"WPS brute-force attack tool.",                         category:"pentest_tools", icon:"Wifi", tags:["wifi","bruteforce"] },
  { name:"wifite",        desc:"Automated wireless attack tool.",                      category:"pentest_tools", icon:"Wifi", tags:["wifi","wireless"] },
  { name:"cowpatty",      desc:"WPA-PSK dictionary/rainbow-table cracker.",            category:"pentest_tools", icon:"Wifi", tags:["wifi","crack"] },
  { name:"pixiewps",      desc:"WPS pixie-dust offline attack tool.",                 category:"pentest_tools", icon:"Wifi", tags:["wifi","crack"] },
  { name:"hcxdumptool",   desc:"WiFi handshake/PMKID capture tool.",                  category:"pentest_tools", icon:"Wifi", tags:["wifi","capture"] },
  { name:"hcxtools",      desc:"Convert captured WiFi handshakes for cracking.",       category:"pentest_tools", icon:"Wifi", tags:["wifi","crack"] },
  { name:"bully",         desc:"WPS brute-force attack tool (reaver alternative).",    category:"pentest_tools", icon:"Wifi", tags:["wifi","bruteforce"] },
  { name:"mdk4",          desc:"WiFi stress-testing and deauth attack tool.",          category:"pentest_tools", icon:"Wifi", tags:["wifi","dos"] },
  { name:"fern-wifi-cracker", desc:"Graphical WiFi security auditing tool.",           category:"pentest_tools", icon:"Wifi", tags:["wifi","wireless"] },
  // ── MITM / network attacks ──
  { name:"bettercap",     desc:"Swiss army knife for MITM network attacks.",          category:"pentest_tools", icon:"Eye", tags:["mitm","network"] },
  { name:"responder",     desc:"LLMNR/NBT-NS/mDNS poisoner for credential capture.",  category:"pentest_tools", icon:"Eye", tags:["mitm","windows"] },
  { name:"ettercap",      desc:"Comprehensive MITM attack suite.",                     category:"pentest_tools", icon:"Eye", tags:["mitm"] },
  { name:"sslstrip",      desc:"HTTPS downgrade and stripping attack tool.",           category:"pentest_tools", icon:"Unlock", tags:["mitm","ssl"] },
  { name:"mitmproxy",     desc:"Interactive TLS-capable intercepting proxy.",         category:"pentest_tools", icon:"Eye", tags:["mitm","proxy"] },
  { name:"dsniff",        desc:"Classic suite of network auditing/sniffing tools.",   category:"pentest_tools", icon:"Eye", tags:["mitm","sniffing"] },
  { name:"dnschef",       desc:"Highly configurable DNS proxy for pentesters.",       category:"pentest_tools", icon:"Globe", tags:["mitm","dns"] },
  { name:"yersinia",      desc:"Layer 2 network protocol attack framework.",           category:"pentest_tools", icon:"Zap", tags:["mitm","network"] },
  { name:"macchanger",    desc:"View/manipulate MAC addresses of network interfaces.",category:"pentest_tools", icon:"Fingerprint", tags:["network","spoofing"] },
  { name:"tcpreplay",     desc:"Replay captured network traffic for testing.",         category:"pentest_tools", icon:"Fish", tags:["mitm","network"] },
  { name:"netsniff-ng",   desc:"High-performance packet sniffer and traffic analyser.", category:"pentest_tools", icon:"Fish", tags:["mitm","network"] },
  // ── Exploitation / Windows / AD ──
  { name:"metasploit",    desc:"World's most used penetration testing framework.",     category:"pentest_tools", icon:"Bomb", tags:["exploit","framework"] },
  { name:"impacket",      desc:"Python classes for network protocol interaction.",     category:"pentest_tools", icon:"Binary", tags:["exploit","windows"] },
  { name:"crackmapexec",  desc:"Swiss army knife for Windows/AD pentesting.",          category:"pentest_tools", icon:"Map", tags:["exploit","windows","ad"] },
  { name:"evil-winrm",    desc:"WinRM shell for Windows pentesting.",                 category:"pentest_tools", icon:"Skull", tags:["exploit","windows"] },
  { name:"bloodhound",    desc:"Active Directory attack path visualisation tool.",     category:"pentest_tools", icon:"Share2", tags:["ad","windows","recon"] },
  { name:"enum4linux",    desc:"Linux SMB/Samba enumeration tool.",                   category:"pentest_tools", icon:"Server", tags:["ad","smb"] },
  { name:"smbclient",     desc:"FTP-like SMB/CIFS client for share enumeration.",     category:"pentest_tools", icon:"Server", tags:["ad","smb"] },
  { name:"ldap-utils",    desc:"Command-line tools for querying LDAP directories.",   category:"pentest_tools", icon:"Server", tags:["ad","ldap"] },
  { name:"smbmap",        desc:"SMB share enumeration and permission auditing tool.", category:"pentest_tools", icon:"Server", tags:["ad","smb"] },
  // ── OSINT ──
  { name:"theharvester",  desc:"OSINT: emails, subdomains, hosts, employee names.",   category:"pentest_tools", icon:"Telescope", tags:["osint","recon"] },
  { name:"maltego",       desc:"Interactive data mining and link analysis tool.",      category:"pentest_tools", icon:"Telescope", tags:["osint"] },
  { name:"recon-ng",      desc:"Web-based open source intelligence framework.",        category:"pentest_tools", icon:"Telescope", tags:["osint","recon"] },
  { name:"dnsrecon",      desc:"DNS enumeration and reconnaissance script.",           category:"pentest_tools", icon:"Globe", tags:["osint","dns"] },
  { name:"subfinder",     desc:"Subdomain discovery using passive sources.",           category:"pentest_tools", icon:"Search", tags:["osint","recon"] },
  { name:"amass",         desc:"In-depth DNS enumeration and network mapping.",        category:"pentest_tools", icon:"Globe", tags:["osint","dns"] },
  { name:"sherlock",      desc:"Hunt usernames across social networks.",              category:"pentest_tools", icon:"Telescope", tags:["osint"] },
  { name:"spiderfoot",    desc:"Automated OSINT reconnaissance framework.",           category:"pentest_tools", icon:"Telescope", tags:["osint","recon"] },
  { name:"exiftool",      desc:"Read/write/edit metadata in files and images.",       category:"pentest_tools", icon:"Image", tags:["osint","forensics"] },
  { name:"whois",         desc:"Query domain/IP registration ownership records.",     category:"pentest_tools", icon:"Telescope", tags:["osint","dns"] },
  { name:"gitleaks",      desc:"Scan git repos for hardcoded secrets and keys.",      category:"pentest_tools", icon:"KeyRound", tags:["osint","secrets"] },
  { name:"h8mail",        desc:"Email OSINT and breach-data hunting tool.",           category:"pentest_tools", icon:"Mail", tags:["osint"] },
  // ── Tunneling / proxy ──
  { name:"proxychains",   desc:"Force any TCP connection through proxies.",            category:"pentest_tools", icon:"Link2", tags:["proxy","utility"] },
  { name:"tor",           desc:"The Onion Router — anonymous network tool.",           category:"pentest_tools", icon:"CircleDot", tags:["anonymity","proxy"] },
  { name:"chisel",        desc:"Fast TCP/UDP tunnel over HTTP.",                       category:"pentest_tools", icon:"Link2", tags:["proxy","tunnel"] },
  { name:"stunnel",       desc:"Universal TLS tunneling wrapper.",                     category:"pentest_tools", icon:"Lock", tags:["proxy","tunnel"] },
  { name:"sshuttle",      desc:"Transparent VPN-like proxy over an SSH connection.",   category:"pentest_tools", icon:"Link2", tags:["proxy","tunnel"] },
  { name:"iodine",        desc:"Tunnel IPv4 traffic through a DNS server.",           category:"pentest_tools", icon:"Link2", tags:["proxy","tunnel","dns"] },
  // ── Vulnerability scanning ──
  { name:"sslscan",       desc:"SSL/TLS configuration scanner and cipher checker.",   category:"pentest_tools", icon:"Lock", tags:["ssl","web"] },
  { name:"openvas",       desc:"Open Vulnerability Assessment System (full VA).",     category:"pentest_tools", icon:"Microscope", tags:["scanner","va"] },
  { name:"trivy",         desc:"Vulnerability scanner for containers and filesystems.", category:"pentest_tools", icon:"Microscope", tags:["scanner","container"] },
  // ── Forensics / reverse engineering / malware ──
  { name:"volatility",    desc:"Advanced memory forensics framework.",                 category:"pentest_tools", icon:"Brain", tags:["forensics","memory"] },
  { name:"autopsy",       desc:"Graphical digital forensics platform.",               category:"pentest_tools", icon:"Microscope", tags:["forensics"] },
  { name:"binwalk",       desc:"Firmware analysis and embedded file extraction.",      category:"pentest_tools", icon:"Binary", tags:["forensics","reverse"] },
  { name:"foremost",      desc:"File recovery based on headers and data structures.", category:"pentest_tools", icon:"FolderOpen", tags:["forensics"] },
  { name:"steghide",      desc:"Steganography program to hide data in images.",       category:"pentest_tools", icon:"Image", tags:["forensics","stego"] },
  { name:"radare2",       desc:"Reverse engineering framework and binary analysis.",   category:"pentest_tools", icon:"Binary", tags:["reverse","binary"] },
  { name:"ghidra",        desc:"NSA reverse engineering suite (SRE framework).",       category:"pentest_tools", icon:"Binary", tags:["reverse","binary"] },
  { name:"gdb",           desc:"GNU Debugger for binary analysis and exploitation.",   category:"pentest_tools", icon:"Bug", tags:["reverse","debug"] },
  { name:"yara",          desc:"Pattern-matching engine for malware research.",       category:"pentest_tools", icon:"Binary", tags:["forensics","malware"] },
  { name:"clamav",        desc:"Open-source antivirus engine.",                       category:"pentest_tools", icon:"Shield", tags:["forensics","malware"] },
  { name:"mat2",          desc:"Metadata anonymisation toolkit.",                     category:"pentest_tools", icon:"Image", tags:["forensics","privacy"] },
  { name:"testdisk",      desc:"Partition recovery and disk repair tool.",            category:"pentest_tools", icon:"FolderOpen", tags:["forensics","recovery"] },
  { name:"photorec",      desc:"File carving and data recovery tool.",                category:"pentest_tools", icon:"FolderOpen", tags:["forensics","recovery"] },
  { name:"sleuthkit",     desc:"Library and tools for digital forensics analysis.",   category:"pentest_tools", icon:"Microscope", tags:["forensics"] },
  { name:"bulk-extractor", desc:"Extracts emails, URLs and other artifacts from disk images.", category:"pentest_tools", icon:"FolderOpen", tags:["forensics"] },
  { name:"hexedit",       desc:"Simple terminal-based hex editor.",                   category:"pentest_tools", icon:"Binary", tags:["forensics","reverse"] },
  { name:"upx",           desc:"Executable packer/unpacker for binary analysis.",     category:"pentest_tools", icon:"Binary", tags:["reverse","malware"] },
  { name:"apktool",       desc:"Reverse-engineer and rebuild Android APK files.",     category:"pentest_tools", icon:"Code", tags:["reverse","mobile"] },
  { name:"jadx",          desc:"Dex-to-Java decompiler for Android apps.",            category:"pentest_tools", icon:"Code", tags:["reverse","mobile"] },
  // ── System hardening / auditing ──
  { name:"lynis",         desc:"Security auditing tool for Unix/Linux systems.",      category:"pentest_tools", icon:"Microscope", tags:["audit","hardening"] },
  { name:"rkhunter",      desc:"Rootkit, backdoor and exploit scanner.",              category:"pentest_tools", icon:"Bug", tags:["audit","malware"] },
  { name:"chkrootkit",    desc:"Locally checks for signs of a rootkit.",              category:"pentest_tools", icon:"Bug", tags:["audit","malware"] },
];

export const DRIVERS: Package[] = [
  { name:"NVIDIA Driver",       desc:"Proprietary NVIDIA GPU driver (non-free).",           category:"drivers", icon:"Cpu", tags:["gpu","nvidia"] },
  { name:"AMD Driver",          desc:"AMD firmware and Mesa open-source GPU drivers.",       category:"drivers", icon:"Cpu", tags:["gpu","amd"] },
  { name:"Intel Driver",        desc:"Intel graphics firmware and VA-API drivers.",          category:"drivers", icon:"Cpu", tags:["gpu","intel"] },
  { name:"WiFi — Broadcom",     desc:"Broadcom STA driver via broadcom-sta-dkms.",          category:"drivers", icon:"Wifi", tags:["wifi","broadcom"] },
  { name:"WiFi — Realtek",      desc:"Realtek rtl8812au / rtl88xxau kernel driver.",        category:"drivers", icon:"Wifi", tags:["wifi","realtek"] },
  { name:"Firmware (non-free)", desc:"Linux non-free firmware: Realtek, Intel WiFi…",      category:"drivers", icon:"Wrench", tags:["firmware"] },
];

// Mirrors HACKEROS_ECOSYSTEM_CATALOG in src-tauri/src/lib.rs exactly — same
// names in the same order, so the frontend catalog and the Rust
// install-strategy/installed-state catalog can never drift apart. If you
// add a tool here, add the matching (name, slug, uninstallable) row there
// too. Install/uninstall run `hacker unpack <slug>` / `hacker pack <slug>`
// on the backend; see lib.rs's HackerOS Ecosystem section for details.
//
// Descriptions are intentionally short for now — a bigger writeup for each
// tool is planned for a future update.
export const HACKEROS_ECOSYSTEM: Package[] = [
  { name:"HackerOS TV",         desc:"Curated streaming & IPTV app hub for HackerOS.",             category:"hackeros_ecosystem", icon:"Tv",            tags:["media","tv"] },
  { name:"Add-ons",             desc:"Optional extras and companion apps for HackerOS.",           category:"hackeros_ecosystem", icon:"Blocks",        tags:["addons"] },
  { name:"GS",                  desc:"Gaming & cybersecurity bundle — tools from both worlds.",    category:"hackeros_ecosystem", icon:"Gauge",         tags:["gaming","cybersecurity"] },
  { name:"Dev Tools",           desc:"Developer toolchain bundle for HackerOS.",                    category:"hackeros_ecosystem", icon:"Code",          tags:["dev"] },
  { name:"Emulators",           desc:"Console and retro system emulator bundle.",                   category:"hackeros_ecosystem", icon:"MonitorPlay",   tags:["gaming","emulation"] },
  { name:"Cybersecurity",       desc:"Core cybersecurity tool bundle for HackerOS.",                category:"hackeros_ecosystem", icon:"Shield",        tags:["cybersecurity"] },
  { name:"Gaming",              desc:"General gaming bundle for HackerOS.",                         category:"hackeros_ecosystem", icon:"Gamepad2",      tags:["gaming"] },
  { name:"Gaming — Roblox",     desc:"Roblox gaming support bundle.",                               category:"hackeros_ecosystem", icon:"Boxes",         tags:["gaming","roblox"] },
  { name:"Hacker Mode",         desc:"Switches the desktop into HackerOS's Hacker Mode.",           category:"hackeros_ecosystem", icon:"Terminal",      tags:["mode"] },
  { name:"Automatic Updates",   desc:"Enables automatic background updates for HackerOS.",          category:"hackeros_ecosystem", icon:"Repeat",        tags:["updates"] },
  { name:"Alacritty Config",    desc:"HackerOS's curated Alacritty terminal configuration.",        category:"hackeros_ecosystem", icon:"SquareTerminal",tags:["terminal","config"] },
  { name:"Winboat",             desc:"Run Windows apps aboard HackerOS via Winboat.",               category:"hackeros_ecosystem", icon:"Wine",          tags:["windows","compat"] },
  { name:"NVIDIA Drivers",      desc:"HackerOS's own NVIDIA driver install flow (via `hacker`).",   category:"hackeros_ecosystem", icon:"Cpu",           tags:["gpu","nvidia","drivers"] },
  { name:"HackerOS Containers", desc:"Container tooling and runtime for HackerOS.",                 category:"hackeros_ecosystem", icon:"Container",     tags:["containers"] },
  { name:"H#",                  desc:"The H# language toolchain.",                                  category:"hackeros_ecosystem", icon:"Binary",        tags:["dev","language"] },
  { name:"H# Utils",            desc:"Utility library and CLI tools for H#.",                       category:"hackeros_ecosystem", icon:"Wrench",        tags:["dev","language"] },
  { name:"HackerOS Builder",    desc:"Build and packaging tool for HackerOS images/apps.",          category:"hackeros_ecosystem", icon:"HardHat",       tags:["dev","build"] },
  { name:"Isolator",            desc:"Sandboxing/isolation tool for running apps safely.",          category:"hackeros_ecosystem", icon:"Lock",          tags:["security","sandbox"] },
  { name:"Hydra",               desc:"HackerOS's Hydra environment. One-way install — cannot be removed via `hacker pack` once unpacked.", category:"hackeros_ecosystem", icon:"Skull", tags:["environment"], uninstallable:false },
  { name:"Hammer",              desc:"HackerOS's `hammer` package-backend fallback tool.",          category:"hackeros_ecosystem", icon:"Hammer",        tags:["packages"] },
  { name:"HackerOS Games",      desc:"Additional curated games for HackerOS.",                      category:"hackeros_ecosystem", icon:"PackagePlus",   tags:["gaming"] },
  { name:"HexAi",               desc:"HackerOS's built-in AI assistant tool.",                      category:"hackeros_ecosystem", icon:"Brain",         tags:["ai"] },
  { name:"HackerDeck",          desc:"Steam Deck-style handheld/gaming UI mode.",                   category:"hackeros_ecosystem", icon:"PlayCircle",    tags:["gaming","handheld"] },
  { name:"Blue Environment",    desc:"The Blue desktop environment for HackerOS.",                  category:"hackeros_ecosystem", icon:"Cloud",         tags:["environment","desktop"] },
  { name:"HWDE",                desc:"HackerOS Windows-style Desktop Environment.",                 category:"hackeros_ecosystem", icon:"Monitor",       tags:["environment","desktop"] },
  { name:"Cybersecurity Mode",  desc:"Switches the desktop into a cybersecurity-focused mode.",     category:"hackeros_ecosystem", icon:"ShieldCheck",   tags:["cybersecurity","mode"] },
  { name:"SDE",                 desc:"HackerOS's Secure Desktop Environment.",                      category:"hackeros_ecosystem", icon:"Layers3",       tags:["environment","security"] },
];

// Mirrors DEV_TOOLS_CATALOG in src-tauri/src/lib.rs: same eight toolchains,
// each contributing exactly two rows here — "Local" (installed straight
// onto the host via apt) and "Container" (installed inside the shared
// `hackeros-devbox` Podman/Distrobox container, exposed on the host via a
// `~/.local/bin/<tool>` wrapper). If you add a toolchain here, add the
// matching (local name, container name, apt package, primary binary) row
// on the Rust side too — install/uninstall dispatch by matching these
// exact display-name strings, so the two catalogs can't drift apart.
//
// Both rows for a tool share `tags` (used by ECOSYSTEM_TAGS-style filter
// pills in DevToolsView) plus one extra tag — "local" or "container" — so
// the tag pills can also filter by install mode, not just by language.
export const DEV_TOOLS: Package[] = [
  { name:"Rust (cargo) — Local",              desc:"Rust toolchain (rustc + cargo) installed directly via apt.", category:"dev_tools", icon:"Cpu",     tags:["rust","local"] },
  { name:"Rust (cargo) — Container",          desc:"Rust toolchain (rustc + cargo), isolated in the HackerOS Dev Tools container.", category:"dev_tools", icon:"Container", tags:["rust","container"] },

  { name:"Node.js (npm) — Local",             desc:"Node.js runtime and npm installed directly via apt.",        category:"dev_tools", icon:"Terminal", tags:["node","javascript","local"] },
  { name:"Node.js (npm) — Container",         desc:"Node.js runtime and npm, isolated in the HackerOS Dev Tools container.", category:"dev_tools", icon:"Container", tags:["node","javascript","container"] },

  { name:"Python (pip) — Local",              desc:"Python 3 and pip installed directly via apt.",               category:"dev_tools", icon:"Code",    tags:["python","local"] },
  { name:"Python (pip) — Container",          desc:"Python 3 and pip, isolated in the HackerOS Dev Tools container.", category:"dev_tools", icon:"Container", tags:["python","container"] },

  { name:"Go — Local",                        desc:"The Go toolchain installed directly via apt.",                category:"dev_tools", icon:"Boxes",   tags:["go","local"] },
  { name:"Go — Container",                    desc:"The Go toolchain, isolated in the HackerOS Dev Tools container.", category:"dev_tools", icon:"Container", tags:["go","container"] },

  { name:"Java (JDK) — Local",                desc:"OpenJDK installed directly via apt.",                         category:"dev_tools", icon:"Coffee",  tags:["java","local"] },
  { name:"Java (JDK) — Container",            desc:"OpenJDK, isolated in the HackerOS Dev Tools container.",      category:"dev_tools", icon:"Container", tags:["java","container"] },

  { name:"Ruby (gem) — Local",                desc:"Ruby and RubyGems installed directly via apt.",               category:"dev_tools", icon:"Gem",     tags:["ruby","local"] },
  { name:"Ruby (gem) — Container",            desc:"Ruby and RubyGems, isolated in the HackerOS Dev Tools container.", category:"dev_tools", icon:"Container", tags:["ruby","container"] },

  { name:"PHP — Local",                       desc:"PHP CLI installed directly via apt.",                         category:"dev_tools", icon:"Code",    tags:["php","local"] },
  { name:"PHP — Container",                   desc:"PHP CLI, isolated in the HackerOS Dev Tools container.",      category:"dev_tools", icon:"Container", tags:["php","container"] },

  { name:"C/C++ (build-essential) — Local",     desc:"gcc, g++, make and friends installed directly via apt.",     category:"dev_tools", icon:"Wrench",  tags:["c","cpp","local"] },
  { name:"C/C++ (build-essential) — Container", desc:"gcc, g++, make and friends, isolated in the HackerOS Dev Tools container.", category:"dev_tools", icon:"Container", tags:["c","cpp","container"] },
];

/** One row per language/toolchain (8, not 16) — pairs up each tool's
 * Local and Container `Package` entries from `DEV_TOOLS` above so
 * `DevToolsView.tsx` can render a single grouped row with one primary
 * "Install" action (which then asks *how*, per
 * `AppSettings.dev_tools_default_mode`) instead of showing two
 * independent, easy-to-miss rows per tool. Relies on `DEV_TOOLS` being
 * written as consecutive (Local, Container) pairs in the same order —
 * if you add a toolchain there, keep that pairing and this derives the
 * grouping automatically with no further changes needed. */
export interface DevToolGroup { label: string; icon: string; tags: string[]; local: Package; container: Package; }
export const DEV_TOOL_GROUPS: DevToolGroup[] = (() => {
  const groups: DevToolGroup[] = [];
  for (let i = 0; i + 1 < DEV_TOOLS.length; i += 2) {
    const local = DEV_TOOLS[i];
    const container = DEV_TOOLS[i + 1];
    groups.push({
      label: local.name.replace(/ — Local$/, ""),
      icon: local.icon,
      tags: (local.tags ?? []).filter(tg => tg !== "local"),
      local, container,
    });
  }
  return groups;
})();

// Only these curated sections (game launchers, pentest tools, drivers,
// HackerOS Ecosystem, Dev Tools) are covered by the local text search
// box — Discover intentionally searches live against the package sources
// instead (see DiscoverView), not this array, so it deliberately does not
// include any Discover-style apps.
export const ALL_PACKAGES: Package[] = [
  ...GAME_LAUNCHERS,
  ...PENTEST_TOOLS,
  ...DRIVERS,
  ...HACKEROS_ECOSYSTEM,
  ...DEV_TOOLS,
];
