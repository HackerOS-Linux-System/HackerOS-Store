export interface Progress { step: string; message: string; progress: number; }
export interface LogLine  { stream: "stdout" | "stderr" | "info" | "error" | "success"; line: string; }
export interface ToastItem { id: number; type: "success" | "error" | "info"; message: string; }
export interface InstalledState { key: string; installed: boolean; version?: string; }

export type Source = "apt" | "flatpak" | "snap" | "brew" | "hpm" | "nix" | "appimage";

export interface DiscoverResult {
  name: string; version: string; desc: string;
  source: Source; package_id: string;
  size?: string; icon?: string | null;
}
export interface DiscoverItem { package_id: string; source: string; name: string; }

/** One source's problem answering a Discover query — shown alongside
 * whatever results the other sources did return, instead of the person
 * just seeing fewer results with no explanation. */
export interface SourceIssue {
  source: string;
  kind: "timeout" | "unavailable" | "error";
  message: string;
}
export interface DiscoverResponse { results: DiscoverResult[]; issues: SourceIssue[]; }

export interface CategoryDef { id: string; label: string; icon: string; }
export interface RatingInfo { average: number; count: number; }

export interface LocalReview { stars: number; comment?: string | null; timestamp: string; }

export interface AppDetails {
  id: string; name: string; source: string; package_id: string;
  summary: string; description: string; icon?: string | null;
  screenshots: string[]; version?: string; license?: string; homepage?: string;
  categories: string[]; size?: string; rating?: RatingInfo | null;
  local_rating?: RatingInfo | null;
  /** Snap only: "strict" | "classic" | "devmode". */
  confinement?: string | null;
}

export interface InstalledSets {
  apt: string[]; flatpak: string[]; snap: string[]; brew: string[];
  hpm: string[];
  /** Packages installed via `hnm` (HackerOS Nix Manager), keyed by the
   * bare nixpkgs attribute name (matches `package_id`). */
  nix: string[];
  /** Installed AppImages, keyed by "owner/repo" (matches `package_id`). */
  appimage: string[];
}

export interface FlatpakRemote { name: string; url: string; }

export interface AppSettings {
  language: string;
  flatpak_remotes: FlatpakRemote[];
  flatpak_default_remote: string;
  flatpak_default_branch: string;
  apt_mirror: string;
  check_updates_on_startup: boolean;
  enabled_sources: string[];
  snap_default_channel: string;
  ratings_enabled: boolean;
  default_section: string;
}
export interface AppInfo { version: string; name: string; target_release: string; }

export interface PackageVersion { name: string; version: string; }

export interface HistoryEntry {
  id: string;
  timestamp: string;
  action: "install" | "uninstall" | "update" | "rollback";
  source: string;
  name: string;
  package_id: string;
  version?: string | null;
  packages?: PackageVersion[] | null;
  success: boolean;
  message?: string | null;
  /** Flatpak only: the ostree commit active right after this action —
   * present iff rollback is actually possible for this entry. */
  commit?: string | null;
  /** Nix (hnm) only: the profile generation active right after this
   * action — present iff rollback is actually possible for this entry.
   * Rolling it back reverts the *whole* nix profile, not just this
   * package (see history.rs's module doc comment). */
  nix_generation?: number | null;
}

export type InstalledMap    = Record<string, { installed: boolean; version?: string }>;
export type InstallingMap   = Record<string, boolean>;
export type UninstallingMap = Record<string, boolean>;

/** One row from `nix_list_generations` — mirrors backend `hnm::NixGeneration`. */
export interface NixGeneration {
  generation: number;
  date: string;
  current: boolean;
}

/** One row from `nix_list_installed` — mirrors backend `hnm::NixInstalledPkg`
 * (itself hnm's own `state::InstalledPkg`, passed through as-is). */
export interface NixInstalledPkg {
  name: string;
  version: string;
  attr_path: string;
  installed_at: string;
  pinned?: string | null;
  description?: string | null;
}

/** One entry in the client-managed install/uninstall queue (see
 * `hooks/useQueue.ts`). Only one entry is ever actually "running" at a
 * time — apt/dpkg only allow a single writer — the rest wait as
 * "pending" and start automatically as the queue drains. */
export interface QueueJob {
  id: string;
  kind: "curated-install" | "curated-uninstall" | "discover-install" | "discover-uninstall" | "update";
  label: string;
  status: "pending" | "running" | "done" | "error";
  error?: string;
  payload: unknown;
}

export const DEFAULT_SETTINGS: AppSettings = {
  language: "en",
  flatpak_remotes: [{ name: "flathub", url: "https://dl.flathub.org/repo/flathub.flatpakrepo" }],
  flatpak_default_remote: "flathub",
  flatpak_default_branch: "",
  apt_mirror: "",
  check_updates_on_startup: true,
  enabled_sources: ["apt", "flatpak", "snap", "brew", "hpm"],
  snap_default_channel: "stable",
  ratings_enabled: true,
  default_section: "discover",
};

export const SOURCES: { id: Source; label: string; color: string }[] = [
  { id: "apt",      label: "APT",      color: "#f97316" },
  { id: "flatpak",  label: "Flatpak",  color: "#3b82f6" },
  { id: "snap",     label: "Snap",     color: "#e11d48" },
  { id: "brew",     label: "Homebrew", color: "#84cc16" },
  { id: "hpm",      label: "HackerOS Repo", color: "#a855f7" },
  { id: "nix",      label: "Nix (hnm)", color: "#7ebae4" },
  { id: "appimage", label: "AppImage", color: "#facc15" },
];

/** Sources not enabled by default (see `DEFAULT_SETTINGS.enabled_sources`)
 * — the Settings sources grid shows a short "why" note for these instead
 * of just an unchecked box, since it's not obvious at a glance why one
 * source starts off instead of on the way the others do. */
export const OPT_IN_SOURCES: Record<string, string> = {
  appimage: "Depends on the community AppImageHub feed and installs desktop integration (wrapper + .desktop + icon) itself.",
  nix: "Needs `hnm update` run once first to build the local nixpkgs index, and can take a while to bootstrap Nix on first install.",
};

/** Common snap channels/tracks, offered as quick picks in the install
 * dialog — not exhaustive (a snap can publish arbitrary tracks), just the
 * ones that exist for virtually every snap. */
export const SNAP_CHANNELS = ["stable", "candidate", "beta", "edge"];

export function sourceColor(s: string): string {
  return SOURCES.find(x => x.id === s)?.color ?? "#8e8e93";
}

export const PENTEST_TAGS = [
  "all", "network", "web", "password", "wifi", "mitm", "exploit", "osint",
  "forensics", "reverse", "ad", "packet", "audit", "utility",
];
