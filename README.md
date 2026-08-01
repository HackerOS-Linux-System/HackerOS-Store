# HackerOS Store

![HackerOS Store](https://github.com/HackerOS-Linux-System/HackerOS-Updates/blob/main/HackerOS/ICONS/Hacker-Unpack.png)

A desktop app store for **HackerOS** (a Debian-based Linux distribution) that lets you search,
install, and manage software from **apt, Flatpak, Snap, Homebrew/Linuxbrew, the HackerOS
Community Repository (`hpm`), Nix/nixpkgs (`hnm`), and AppImage** all in one place, plus curated
sections for game launchers, penetration-testing tools, and proprietary drivers.

Built with [Tauri 2](https://tauri.app) — a [SolidJS](https://www.solidjs.com) frontend and a
Rust backend.

---

## Features

- **Discover** — live search and category browsing across apt, Flatpak, Snap, Homebrew, the
  HackerOS Community Repository (`hpm`), Nix/nixpkgs (`hnm`), and AppImage, with app details,
  screenshots, and community ratings. Every source can be toggled on/off in Settings; `hpm`,
  `hnm`, and AppImage each show a "not detected" hint when the underlying tool/feed isn't
  available instead of silently returning zero results.
  - **Nix/nixpkgs support** is provided entirely through `hnm` (HackerOS Nix Manager,
    `/usr/bin/hnm`) — this app never touches Nix itself or hnm's own package index directly, the
    same way it shells out to `hpm` rather than reading its SQLite store. It's opt-in by default
    (see `src/types.ts`'s `OPT_IN_SOURCES`) since it needs `hnm update` run once first to build
    the local nixpkgs index, and a first install can trigger a from-scratch Nix bootstrap.
  - Results are cached **per source** for a short TTL (120s), not just as one all-or-nothing
    blob per query — so nix (and every other source) benefits from "flip back to a
    recently-viewed search/category" the same way apt/Flatpak always have, even right after
    toggling a source on/off. Running "Build Nix index" or "Refresh AppImage catalog"
    immediately invalidates that source's cached results, so the next search reflects the
    rebuild right away instead of waiting out the TTL.
- **Game Launchers** — one-click install for Steam, Lutris, Heroic/Epic Games Store, Bottles,
  GOG Galaxy, Battle.net, and the EA App (the last three run through Wine, downloaded and
  configured automatically).
- **Pentest Tools** — a curated catalog of security tools. Debian-packaged tools install
  straight from apt; everything else runs inside a dedicated Kali Linux [distrobox](https://distrobox.it)
  container (created once, on first use).
- **Drivers** — proprietary/non-free driver installs (e.g. NVIDIA), including adding the
  non-free apt repositories when needed.
- **System updates** — a one-click `apt`/`nala`-based system update with a live terminal log.
- **Nix panel** — everything `hnm` (HackerOS Nix Manager) offers beyond Discover's plain
  search/install/remove: browsing and rolling back Nix profile generations, pinning/unpinning
  package versions, garbage collection, cache cleaning, `hnm doctor`/`hnm check` diagnostics,
  shell integration (`hnm env activate`/`deactivate`), and a one-click "build the local nixpkgs
  index" action (also available as a quick action in Settings).
- **Install History** — every install, removal, and system update is recorded locally, with
  best-effort rollback for apt packages (single version or per-package for multi-package
  actions like driver installs), Flatpak (pinned to the ostree commit active right after the
  action), the HackerOS Community Repository (`hpm`, which tracks its own restore snapshot),
  AppImage, and Nix (`hnm`) — though Nix rollback reverts the *whole* profile generation, not
  just one package, since that's the only granularity `hnm rollback` itself offers; the panel
  says so plainly rather than implying a narrower undo.
- **Ratings & reviews** — star ratings for every source (not just Flatpak), plus your own
  reviews, stored on-device.
- **Multi-language UI** — currently English and Polish; see [Adding a new language](#adding-a-new-language)
  below for how easy it is to add another.

## Screenshots

I can't generate real screenshots of a running GUI from here, but the folder and markdown
below are ready to go — just drop PNGs in `docs/screenshots/` with these exact names and
uncomment the `<img>` lines (or replace this whole section) to have them show up on GitHub:

| Suggested shot | File | What to show |
|---|---|---|
| Discover | `docs/screenshots/discover.png` | Search results mixing a few different sources' badges (apt/Flatpak/hpm/nix) |
| App detail | `docs/screenshots/app-detail.png` | An app with screenshots/description/size populated |
| Nix panel | `docs/screenshots/nix-panel.png` | The generations list plus the installed-packages table |
| Pentest Tools | `docs/screenshots/pentest-tools.png` | The curated tool grid with a couple already installed |
| Settings | `docs/screenshots/settings.png` | The sources grid, including an opt-in source's hint text |

```md
<!-- once the PNGs above exist, replace this comment with: -->
<p align="center">
  <img src="docs/screenshots/discover.png" width="49%" />
  <img src="docs/screenshots/app-detail.png" width="49%" />
</p>
<p align="center">
  <img src="docs/screenshots/nix-panel.png" width="49%" />
  <img src="docs/screenshots/pentest-tools.png" width="49%" />
</p>
```

*(Until then: `npm run dev` — see below — and this section stays a checklist rather than a
broken image link.)*

---

## Installing / building from source

Requirements: `npm`, `rustc`/`cargo`, and (on Debian/HackerOS) a handful of WebKitGTK dev
packages:

```sh
sudo apt install libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libdbus-1-dev \
                  libwebkit2gtk-4.1-dev libgtk-3-dev
```

Then:

```sh
cd source-code
npm install
npm run tauri build
```

The built package (`.deb`, AppImage, etc., depending on your platform) will be under
`source-code/src-tauri/target/release/bundle/`.

For local development (hot reload):

```sh
cd source-code
npm install
npm run tauri dev
```

---

## Architecture

```
source-code/
├─ src/                    SolidJS frontend
│  ├─ App.tsx              Thin orchestrator: wires hooks to views, owns top-level state
│  ├─ main.tsx             Entry point (I18nProvider + AppErrorBoundary + render)
│  ├─ types.ts             Shared TypeScript types
│  ├─ iconMap.tsx           Icon lookups (lucide-solid)
│  ├─ data/packages.ts     Curated package catalogs (launchers, pentest tools, drivers)
│  ├─ i18n/                Translations — one flat file per language (see below)
│  ├─ hooks/               useI18n, useSettings, useInstalledState, useOperationRunner,
│  │                       useQueue, useToasts, useOnlineStatus
│  └─ components/          Sidebar, DiscoverView, AppDetailModal, SettingsView,
│                          HistoryView, NixView, PentestView, TerminalLog, etc.
└─ src-tauri/              Rust backend
   └─ src/
      ├─ lib.rs            Tauri commands: install/uninstall/update, Discover search
      │                    (apt/flatpak/snap/brew/hpm/nix/appimage), settings, job/progress events
      ├─ hpm.rs            HackerOS Community Repository support, via the `hpm` CLI
      ├─ hnm.rs            Nix/nixpkgs support, via the `hnm` (HackerOS Nix Manager) CLI
      ├─ appimage.rs       AppImageHub-backed AppImage search/install/desktop integration
      ├─ security.rs       Shell-injection defenses (see below)
      ├─ ratings.rs        Local star ratings & reviews (all sources)
      └─ history.rs        Install history log + best-effort apt rollback
```

The frontend used to be a single ~52 KB `App.tsx` file (plain React). It's now split into
focused components and hooks — each view (Discover, Settings, History, Pentest Tools, …) is its
own file, and the repeated "run a long operation with a terminal log and a progress bar"
boilerplate lives in one hook (`useOperationRunner`) instead of being copy-pasted five times.

### Why SolidJS

The UI was ported from React to SolidJS. The visual design and behavior are unchanged; the
practical difference for anyone working on this codebase is that Solid components run their
setup code once (there's no re-render step to think about), so most of the
`useCallback`/`useMemo`/dependency-array bookkeeping React needed simply isn't there. State is
plain signals (`createSignal`), and lists/conditionals use Solid's `<For>`/`<Show>` control-flow
components instead of `.map()`/ternaries baked into JSX.

---

## Adding a new language

Translations live under `src/i18n/`, one flat file per language, instead of the old approach of
one big shared object every language had to touch. To add, say, German:

1. Copy `src/i18n/en.ts` to `src/i18n/de.ts` and translate every value. TypeScript enforces that
   you don't miss (or misspell) a key: every locale file is typed as
   `Record<TranslationKey, string>`, where `TranslationKey` comes from `en.ts`.
2. In `src/i18n/index.ts`, add `"de"` to the `Lang` union and to the `LANGUAGES` array.
3. Add `de` to the `LOCALES` map in the same file.

That's it — no other file needs to change, and the language picker in Settings picks it up
automatically.

---

## Ratings, reviews, and Install History — what's real vs. what's local-only

- **Community (ODRS) ratings** are fetched live from `odrs.gnome.org` and only exist for
  **Flatpak** apps, since that's the only source with a public ratings API this app integrates
  with. This can be turned off in Settings if you'd rather the app not make that outbound
  request.
- **Your own ratings/reviews** work for **every source** (apt, Flatpak, Snap, Homebrew). These
  are stored locally on your machine (`~/.hackeros/store/ratings.json`) — HackerOS Store has no
  server of its own to sync reviews across users, and the UI is upfront about that rather than
  implying otherwise.
- **Install History** (`~/.hackeros/store/history.json`) logs every install/removal/update. Fields
  like the resolved apt version are recorded on a best-effort basis.
- **Rollback** currently works for **apt-backed installs with a recorded version**: pentest
  tools installed via apt (single package), and drivers (which install several apt packages at
  once — e.g. "NVIDIA Driver" pulls in `nvidia-driver` and `firmware-misc-nonfree` — every one of
  them is tracked and rolled back together, or the rollback stops and reports exactly which
  package it failed on). This only works if the exact recorded version is still present in your
  local apt cache (apt doesn't keep old `.deb`s around by default). Flatpak rollback would need
  the specific Flatpak commit hash, which isn't recorded yet — asking to roll back a Flatpak
  install returns a clear "not supported" message rather than silently doing nothing. Snap,
  Homebrew, and game launchers are in the same "not supported yet" bucket — game launchers in
  particular are installed via Flatpak or a downloaded Wine installer, neither of which has a
  meaningful package version to pin here.

## Install queue

Requesting several installs/removals in a row (e.g. tapping Install on three Discover results)
queues them instead of firing them all at once. They're shown in a small queue panel in the
sidebar and run one at a time, in order. This is deliberate, not a limitation to be "fixed" with
true concurrency later: `apt`/`dpkg` hold an exclusive lock, so two `apt-get install` processes
running at once just fail against each other. Flatpak/Snap/Homebrew could technically run in
parallel, but a single consistent queue is easier to reason about (and to show progress for)
than a mix of concurrent and sequential operations.

- **Reordering**: pending (not-yet-started) items can be moved up/down in the queue with the
  arrows next to them. The item currently running can't be reordered or "un-started" — cancel it
  from the terminal-log modal instead, which the queue treats as a normal failure and moves on
  from.
- **Persistence**: the queue is mirrored to `~/.hackeros/store/queue.json` as it changes. If the
  app is closed (or crashes) with items still queued, they're reloaded and resumed the next time
  it starts — including a job that was mid-flight when the app closed, which simply restarts from
  the top, since there's no way to know how far an interrupted install actually got.

## Homebrew (Linuxbrew) support

Homebrew on Linux is supported as a fourth package source, but it's the least mature of the four:

- Metadata (descriptions, icons, screenshots, sizes) is generally sparser than apt/Flatpak/Snap.
- It's not installed by default on HackerOS or most Debian systems; Settings now shows whether
  `brew` was actually detected on your machine, instead of the source silently failing every
  search/install if you don't have it.
- If you don't use Homebrew, you can disable it as a source in Settings — Discover will simply
  not query it, which also speeds up searches slightly.

---

## Security

Every package name/id that Discover's search results (or a person's own input) hand to the
backend passes through an allowlist validator (`src-tauri/src/security.rs`) before it's used to
build any command — rejecting anything outside `[A-Za-z0-9 . - _ + : @ /]`, a leading `-`, or a
`..` path-traversal attempt. Most operations (Flatpak/Homebrew install/uninstall, Wine
downloads, the Kali distrobox tool installs, cache cleanup) were also converted from
`sh -c "…"` string interpolation to argv-based commands, so there's no shell parsing untrusted
input to begin with; the one remaining shell helper (`run_sh`) is unused by default and
documented as requiring validation + quoting if a future feature needs it.

If you find a security issue, please open an issue (or, for anything sensitive, contact the
maintainers directly) rather than filing a public exploit write-up.

---

## Accessibility & offline behavior

- Modals (app details, terminal log) are marked `role="dialog"` / `aria-modal`, take focus on
  open, trap Tab/Shift+Tab so focus can't escape onto the page behind them, restore focus to
  whatever triggered them (the Discover card, the "view log" button, …) on close, and close on
  `Escape`.
- Icon-only buttons have `aria-label`s; the active nav item is marked `aria-current`; toasts are
  in an `aria-live` region.
- All interactive elements have a visible focus ring (`:focus-visible`) for keyboard navigation.
- An offline banner appears app-wide when the browser reports no network connection. Discover
  additionally distinguishes "no results" from "all sources failed to respond" (a timeout/network
  error), instead of showing the same empty state for both.

---

## Known limitations / ideas for future work

- The Tauri bundle config currently targets `"all"` platforms, even though the app's
  functionality (apt, `pkexec`, distrobox, `.desktop` files) is Linux-only; scoping this down to
  `deb`/Linux targets is a reasonable follow-up.
- No app icon is currently checked into `src-tauri/icons/`, which `tauri.conf.json` references —
  add one before cutting a bundled release.
- No automated tests or CI workflow yet.
- True concurrent installs across independent sources (e.g. an apt install and a Flatpak install
  running at the same time) are intentionally not supported yet — see [Install queue](#install-queue).

---

## License

BSD 3-Clause License — see [`LICENSE`](./LICENSE).
