const MAX_TOKEN_LEN: usize = 200;

/// Validates a package name / package id / other single shell "word"
/// against a strict allowlist of characters that cover every real package
/// naming scheme we deal with:
///   - APT/Debian package names: lowercase letters, digits, `+ - . :`
///   - Flatpak/AppStream ids: reverse-DNS, e.g. `org.mozilla.firefox`
///   - Snap names: lowercase letters, digits, `-`
///   - Homebrew formulae/casks: letters, digits, `@ + - . /`
///
/// Returns the trimmed token on success, or a human-readable error
/// (safe to show to the user / put in a toast) on rejection. Never panics.
pub fn validate_pkg_token(raw: &str) -> Result<String, String> {
    let s = raw.trim();

    if s.is_empty() {
        return Err("Empty package identifier.".to_string());
    }
    if s.len() > MAX_TOKEN_LEN {
        return Err("Package identifier is unreasonably long.".to_string());
    }
    if s.starts_with('-') {
        // Prevents the token from being interpreted as a CLI flag by
        // apt/flatpak/snap/brew when placed as a bare argv element.
        return Err("Package identifier must not start with '-'.".to_string());
    }

    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+' | ':' | '@' | '/');
    if !s.chars().all(allowed) {
        return Err(format!(
            "Rejected package identifier containing disallowed characters: {s:?}. \
             Only letters, digits, and '. - _ + : @ /' are permitted."
        ));
    }
    // Defense against path traversal via ids used to build filesystem paths
    // elsewhere (e.g. desktop-entry / wrapper-script filenames).
    if s.contains("..") {
        return Err("Rejected package identifier containing '..'.".to_string());
    }

    Ok(s.to_string())
}

/// Validates a whole batch, short-circuiting on the first failure. Handy
/// for the curated-section installers that operate on a small fixed list
/// of package names at once.
pub fn validate_all<'a>(tokens: &[&'a str]) -> Result<Vec<String>, String> {
    tokens.iter().map(|t| validate_pkg_token(t)).collect()
}

/// POSIX single-quote escaping for the rare case where a value must be
/// interpolated into a `sh -c` string rather than passed as a separate
/// argv element. Closes the current quote, emits an escaped literal quote,
/// then reopens quoting: `it's` -> `'it'\''s'`.
///
/// Prefer restructuring the call to avoid the shell entirely (pass the
/// value as its own `Command::arg(...)`) over relying on this — quoting is
/// the fallback, not the first line of defense.
pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_ids() {
        assert!(validate_pkg_token("firefox").is_ok());
        assert!(validate_pkg_token("org.mozilla.firefox").is_ok());
        assert!(validate_pkg_token("nmap").is_ok());
        assert!(validate_pkg_token("g++").is_ok());
        assert!(validate_pkg_token("some-package_1.2+deb12").is_ok());
    }

    #[test]
    fn rejects_shell_metacharacters() {
        assert!(validate_pkg_token("foo; rm -rf ~").is_err());
        assert!(validate_pkg_token("foo`whoami`").is_err());
        assert!(validate_pkg_token("$(whoami)").is_err());
        assert!(validate_pkg_token("foo'bar").is_err());
        assert!(validate_pkg_token("foo|bar").is_err());
        assert!(validate_pkg_token("foo && bar").is_err());
        assert!(validate_pkg_token("").is_err());
        assert!(validate_pkg_token("-rf").is_err());
        assert!(validate_pkg_token("../../etc/passwd").is_err());
    }

    #[test]
    fn quoting_is_safe() {
        assert_eq!(sh_quote("firefox"), "'firefox'");
        assert_eq!(sh_quote("it's"), r"'it'\''s'");
    }
}
