export type Category = "game_launchers" | "pentest_tools" | "drivers" | "update" | "discover" | "settings";
export interface Package { name: string; desc: string; category: Category; icon: string; tags?: string[]; }

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
  // ── Wireless ──
  { name:"aircrack-ng",   desc:"802.11 WEP/WPA/WPA2 security auditing suite.",       category:"pentest_tools", icon:"Wifi", tags:["wifi","wireless"] },
  { name:"kismet",        desc:"Wireless network detector, sniffer, and IDS.",        category:"pentest_tools", icon:"Wifi", tags:["wifi","wireless"] },
  { name:"reaver",        desc:"WPS brute-force attack tool.",                         category:"pentest_tools", icon:"Wifi", tags:["wifi","bruteforce"] },
  { name:"wifite",        desc:"Automated wireless attack tool.",                      category:"pentest_tools", icon:"Wifi", tags:["wifi","wireless"] },
  { name:"cowpatty",      desc:"WPA-PSK dictionary/rainbow-table cracker.",            category:"pentest_tools", icon:"Wifi", tags:["wifi","crack"] },
  { name:"pixiewps",      desc:"WPS pixie-dust offline attack tool.",                 category:"pentest_tools", icon:"Wifi", tags:["wifi","crack"] },
  { name:"hcxdumptool",   desc:"WiFi handshake/PMKID capture tool.",                  category:"pentest_tools", icon:"Wifi", tags:["wifi","capture"] },
  { name:"hcxtools",      desc:"Convert captured WiFi handshakes for cracking.",       category:"pentest_tools", icon:"Wifi", tags:["wifi","crack"] },
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
  // ── Exploitation / Windows / AD ──
  { name:"metasploit",    desc:"World's most used penetration testing framework.",     category:"pentest_tools", icon:"Bomb", tags:["exploit","framework"] },
  { name:"impacket",      desc:"Python classes for network protocol interaction.",     category:"pentest_tools", icon:"Binary", tags:["exploit","windows"] },
  { name:"crackmapexec",  desc:"Swiss army knife for Windows/AD pentesting.",          category:"pentest_tools", icon:"Map", tags:["exploit","windows","ad"] },
  { name:"evil-winrm",    desc:"WinRM shell for Windows pentesting.",                 category:"pentest_tools", icon:"Skull", tags:["exploit","windows"] },
  { name:"bloodhound",    desc:"Active Directory attack path visualisation tool.",     category:"pentest_tools", icon:"Share2", tags:["ad","windows","recon"] },
  { name:"enum4linux",    desc:"Linux SMB/Samba enumeration tool.",                   category:"pentest_tools", icon:"Server", tags:["ad","smb"] },
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
  // ── Tunneling / proxy ──
  { name:"proxychains",   desc:"Force any TCP connection through proxies.",            category:"pentest_tools", icon:"Link2", tags:["proxy","utility"] },
  { name:"tor",           desc:"The Onion Router — anonymous network tool.",           category:"pentest_tools", icon:"CircleDot", tags:["anonymity","proxy"] },
  { name:"chisel",        desc:"Fast TCP/UDP tunnel over HTTP.",                       category:"pentest_tools", icon:"Link2", tags:["proxy","tunnel"] },
  { name:"stunnel",       desc:"Universal TLS tunneling wrapper.",                     category:"pentest_tools", icon:"Lock", tags:["proxy","tunnel"] },
  // ── Vulnerability scanning ──
  { name:"sslscan",       desc:"SSL/TLS configuration scanner and cipher checker.",   category:"pentest_tools", icon:"Lock", tags:["ssl","web"] },
  { name:"openvas",       desc:"Open Vulnerability Assessment System (full VA).",     category:"pentest_tools", icon:"Microscope", tags:["scanner","va"] },
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

// Only these curated sections (game launchers, pentest tools, drivers) are
// covered by the local text search box — Discover intentionally searches
// live against the package sources instead (see DiscoverView), not this
// array, so it deliberately does not include any Discover-style apps.
export const ALL_PACKAGES: Package[] = [
  ...GAME_LAUNCHERS,
  ...PENTEST_TOOLS,
  ...DRIVERS,
];
