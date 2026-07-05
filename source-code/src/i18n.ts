import { useCallback, useState } from "react";

export type Lang = "en" | "pl";

export const LANGUAGES: { id: Lang; label: string }[] = [
  { id: "en", label: "English" },
  { id: "pl", label: "Polski" },
];

const STRINGS: Record<string, Record<Lang, string>> = {
  "nav.discover":        { en: "Discover",        pl: "Odkrywaj" },
  "nav.game_launchers":  { en: "Game Launchers",   pl: "Launchery gier" },
  "nav.pentest_tools":   { en: "Pentest Tools",    pl: "Narzędzia pentest" },
  "nav.drivers":         { en: "Drivers",          pl: "Sterowniki" },
  "nav.update":          { en: "Update System",    pl: "Aktualizuj system" },
  "nav.settings":        { en: "Settings",         pl: "Ustawienia" },

  "search.placeholder":  { en: "Search… (Ctrl+F)", pl: "Szukaj… (Ctrl+F)" },

  "btn.install":         { en: "Install",          pl: "Instaluj" },
  "btn.installing":      { en: "Installing…",       pl: "Instalowanie…" },
  "btn.installed":       { en: "Installed",         pl: "Zainstalowano" },
  "btn.uninstall":       { en: "Remove",            pl: "Usuń" },
  "btn.uninstalling":    { en: "Removing…",         pl: "Usuwanie…" },
  "btn.cancel":          { en: "Cancel",            pl: "Anuluj" },
  "btn.cancelling":      { en: "Cancelling…",       pl: "Anulowanie…" },
  "btn.updateNow":       { en: "Update Now",        pl: "Aktualizuj teraz" },
  "btn.updating":        { en: "Updating…",         pl: "Aktualizowanie…" },
  "btn.viewLog":         { en: "View terminal log", pl: "Pokaż log terminala" },
  "btn.save":            { en: "Save",              pl: "Zapisz" },
  "btn.clearCache":      { en: "Clear cache",       pl: "Wyczyść pamięć podręczną" },
  "btn.clearing":        { en: "Clearing…",         pl: "Czyszczenie…" },

  "toast.installOk":     { en: "installed successfully.",   pl: "zainstalowano pomyślnie." },
  "toast.installFail":   { en: "Failed:",                   pl: "Błąd:" },
  "toast.uninstallOk":   { en: "removed successfully.",     pl: "usunięto pomyślnie." },
  "toast.uninstallFail": { en: "Removal failed:",           pl: "Usuwanie nie powiodło się:" },
  "toast.updateOk":      { en: "System updated!",           pl: "System zaktualizowany!" },
  "toast.updateFail":    { en: "Update failed:",            pl: "Aktualizacja nie powiodła się:" },
  "toast.cancelled":     { en: "Cancelled.",                pl: "Anulowano." },
  "toast.settingsSaved": { en: "Settings saved.",           pl: "Ustawienia zapisane." },
  "toast.cacheCleared":  { en: "Cache cleared.",             pl: "Pamięć podręczna wyczyszczona." },

  "update.title":        { en: "Update System",     pl: "Aktualizacja systemu" },

  "settings.title":            { en: "Settings",                       pl: "Ustawienia" },
  "settings.language":         { en: "Language",                       pl: "Język" },
  "settings.mirrors":          { en: "Mirrors",                        pl: "Serwery lustrzane" },
  "settings.flatpakRemote":    { en: "Flatpak remote URL",             pl: "Adres URL zdalnego repozytorium Flatpak" },
  "settings.aptMirror":        { en: "APT mirror host (optional)",     pl: "Host mirrora APT (opcjonalnie)" },
  "settings.aptMirrorHint":    { en: "Used only when adding non-free repositories for drivers. Leave empty for the default (deb.debian.org).",
                                  pl: "Używane tylko przy dodawaniu repozytoriów non-free dla sterowników. Pozostaw puste, aby użyć domyślnego (deb.debian.org)." },
  "settings.maintenance":      { en: "Maintenance",                    pl: "Konserwacja" },
  "settings.clearCacheHint":   { en: "Removes apt's package cache, unused Flatpak runtimes, and downloaded Wine installers.",
                                  pl: "Usuwa pamięć podręczną pakietów apt, nieużywane środowiska Flatpak i pobrane instalatory Wine." },
  "settings.about":            { en: "About",                          pl: "O programie" },
  "settings.version":          { en: "Version",                        pl: "Wersja" },
  "settings.targetRelease":    { en: "Target release",                 pl: "Docelowe wydanie" },
  "settings.startupUpdates":   { en: "Check for system updates on startup", pl: "Sprawdzaj aktualizacje systemu przy starcie" },

  "pentest.sub": {
    en: "Debian-repo tools install via apt directly. Others use a Kali Linux distrobox container (created once, ~5 min first run).",
    pl: "Narzędzia z repozytoriów Debiana instalowane są bezpośrednio przez apt. Pozostałe używają kontenera Kali Linux (distrobox), tworzonego jednorazowo (~5 min przy pierwszym uruchomieniu).",
  },
  "store.hero.badge":   { en: "Discover",   pl: "Odkrywaj" },
  "store.hero.title":   { en: "Find great software", pl: "Znajdź świetne oprogramowanie" },
  "store.hero.sub":     { en: "Search apt, Flatpak, Snap and Homebrew in one place.",
                          pl: "Przeszukuj apt, Flatpak, Snap i Homebrew w jednym miejscu." },
  "store.searchPlaceholder": { en: "Search all package sources…", pl: "Szukaj we wszystkich źródłach pakietów…" },
  "store.allFeatured":  { en: "All Featured", pl: "Wszystkie polecane" },

  "info.noInfo":        { en: "No info available", pl: "Brak dostępnych informacji" },
  "info.latest":        { en: "Latest:",  pl: "Najnowsza:" },
  "info.size":          { en: "Size:",    pl: "Rozmiar:" },

  // ── Discover ──
  "discover.title":       { en: "Discover",                   pl: "Odkrywaj" },
  "discover.sub":         { en: "Browse and search apps live across apt, Flatpak, Snap and Homebrew.",
                             pl: "Przeglądaj i wyszukuj aplikacje na żywo w apt, Flatpak, Snap i Homebrew." },
  "discover.searchPlaceholder": { en: "Search apps across all sources…", pl: "Szukaj aplikacji we wszystkich źródłach…" },
  "discover.categories":  { en: "Categories",                  pl: "Kategorie" },
  "discover.back":        { en: "Back to categories",          pl: "Wróć do kategorii" },
  "discover.resultsFor":  { en: "results for",                 pl: "wyników dla" },
  "discover.noResults":   { en: "No apps found. Try a different search or enable more sources in Settings.",
                             pl: "Nie znaleziono aplikacji. Spróbuj innego wyszukiwania lub włącz więcej źródeł w Ustawieniach." },
  "discover.loading":     { en: "Searching apt, Flatpak, Snap and Homebrew…",
                             pl: "Przeszukiwanie apt, Flatpak, Snap i Homebrew…" },
  "discover.viewDetails": { en: "View details",                pl: "Zobacz szczegóły" },
  "discover.source.apt":     { en: "APT / Debian",  pl: "APT / Debian" },
  "discover.source.flatpak": { en: "Flatpak",        pl: "Flatpak" },
  "discover.source.snap":    { en: "Snap",           pl: "Snap" },
  "discover.source.brew":    { en: "Homebrew",       pl: "Homebrew" },

  "category.development": { en: "Development",             pl: "Programowanie" },
  "category.office":      { en: "Office & Productivity",   pl: "Biuro i produktywność" },
  "category.graphics":    { en: "Graphics & Photography",  pl: "Grafika i fotografia" },
  "category.media":       { en: "Audio & Video",           pl: "Audio i wideo" },
  "category.internet":    { en: "Internet & Communication",pl: "Internet i komunikacja" },
  "category.security":    { en: "Security & Privacy",      pl: "Bezpieczeństwo i prywatność" },
  "category.system":      { en: "System Tools",            pl: "Narzędzia systemowe" },
  "category.games":       { en: "Games",                   pl: "Gry" },
  "category.utilities":   { en: "Utilities",                pl: "Narzędzia" },

  // ── App detail modal ──
  "detail.description":   { en: "Description",   pl: "Opis" },
  "detail.screenshots":   { en: "Screenshots",    pl: "Zrzuty ekranu" },
  "detail.noScreenshots": { en: "No screenshots available for this app.", pl: "Brak dostępnych zrzutów ekranu dla tej aplikacji." },
  "detail.rating":        { en: "Community rating", pl: "Ocena społeczności" },
  "detail.noRating":      { en: "No ratings yet",   pl: "Brak jeszcze ocen" },
  "detail.ratingsOff":    { en: "Ratings are disabled in Settings.", pl: "Oceny są wyłączone w Ustawieniach." },
  "detail.version":       { en: "Version",   pl: "Wersja" },
  "detail.size":          { en: "Size",      pl: "Rozmiar" },
  "detail.license":       { en: "License",   pl: "Licencja" },
  "detail.homepage":      { en: "Homepage",  pl: "Strona domowa" },
  "detail.source":        { en: "Source",    pl: "Źródło" },
  "detail.loading":       { en: "Loading app details…", pl: "Wczytywanie szczegółów aplikacji…" },
  "detail.close":         { en: "Close",     pl: "Zamknij" },

  // ── Settings (extended) ──
  "settings.sources":          { en: "Package sources",  pl: "Źródła pakietów" },
  "settings.sourcesHint":      { en: "Choose which sources Discover searches and browses. Disabling a source you don't have installed avoids errors and speeds up search.",
                                  pl: "Wybierz, które źródła przeszukuje i przegląda Discover. Wyłączenie źródła, którego nie masz zainstalowanego, unika błędów i przyspiesza wyszukiwanie." },
  "settings.ratings":          { en: "Community ratings",             pl: "Oceny społeczności" },
  "settings.ratingsToggle":    { en: "Fetch star ratings from odrs.gnome.org for Flatpak apps",
                                  pl: "Pobieraj oceny gwiazdkowe z odrs.gnome.org dla aplikacji Flatpak" },
  "settings.ratingsHint":      { en: "This makes an outbound network request when you open an app's details. Turn off if you'd rather the app stayed fully offline.",
                                  pl: "Powoduje to zapytanie sieciowe po otwarciu szczegółów aplikacji. Wyłącz, jeśli wolisz, aby aplikacja pozostała całkowicie offline." },
  "settings.defaultSection":   { en: "Section to open on launch", pl: "Sekcja otwierana przy starcie" },
  "settings.reset":            { en: "Reset all settings to defaults", pl: "Przywróć wszystkie ustawienia domyślne" },
  "settings.resetConfirm":     { en: "Reset all settings to their defaults?", pl: "Przywrócić wszystkie ustawienia do wartości domyślnych?" },

  "update.badge":         { en: "updates",  pl: "aktualizacji" },
};

export function translate(lang: Lang, key: string): string {
  return STRINGS[key]?.[lang] ?? STRINGS[key]?.en ?? key;
}

export function useTranslation(initial: Lang = "en") {
  const [lang, setLang] = useState<Lang>(initial);
  const t = useCallback((key: string) => translate(lang, key), [lang]);
  return { lang, setLang, t };
}
