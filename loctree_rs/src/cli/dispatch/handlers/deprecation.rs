//! Deprecation warnings for commands migrating to 0.7.0 syntax
//!
//! All deprecated commands still work but print a warning to stderr
//! directing users to the new 0.7.0 syntax.

/// Print a deprecation warning to stderr (does not break piped output)
pub fn warn_deprecated(old_cmd: &str, new_cmd: &str) {
    eprintln!(
        "[DEPRECATED] 'loct {}' will be removed in 0.8. Use: {}",
        old_cmd, new_cmd
    );
}
