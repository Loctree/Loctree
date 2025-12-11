use std::sync::OnceLock;

use regex::Regex;

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid regex literal")
}

// --- Rust Regexes ---

pub(crate) fn regex_tauri_command_fn() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Matches #[tauri::command] followed by optional additional attributes like #[allow(...)]
        // before the function definition
        regex(r#"(?m)#\s*\[\s*tauri::command([^\]]*)\](?:\s*#\s*\[[^\]]*\])*\s*(?:pub\s*(?:\([^)]*\)\s*)?)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)\s*\((?P<params>[^)]*)\)"#)
    })
}

/// Build a regex to match custom attribute macros that generate Tauri commands.
///
/// For example, if `macro_names` contains `["api_cmd_tauri", "custom_command"]`,
/// this will match `#[api_cmd_tauri(...)]` or `#[custom_command(...)]` on functions.
///
/// Returns `None` if `macro_names` is empty.
pub fn regex_custom_command_fn(macro_names: &[String]) -> Option<Regex> {
    if macro_names.is_empty() {
        return None;
    }

    // Escape any special regex characters in macro names and join with |
    let escaped: Vec<String> = macro_names.iter().map(|name| regex::escape(name)).collect();
    let pattern = escaped.join("|");

    // Build regex similar to regex_tauri_command_fn but with dynamic macro names
    // Matches: #[macro_name(...)] fn name(...)
    // Supports optional crate:: prefix and additional attributes
    let full_pattern = format!(
        r#"(?m)#\s*\[\s*(?:crate::)?(?:{})([^\]]*)\](?:\s*#\s*\[[^\]]*\])*\s*(?:pub\s*(?:\([^)]*\)\s*)?)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)\s*\((?P<params>[^)]*)\)"#,
        pattern
    );

    Regex::new(&full_pattern).ok()
}

/// Matches Tauri registrations like `tauri::generate_handler![foo, bar]` or `generate_handler![foo, bar]`.
/// Captures the comma-separated list of function identifiers inside the brackets.
pub(crate) fn regex_tauri_generate_handler() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Supports optional `tauri::` prefix and arbitrary whitespace/newlines around the list.
        regex(r#"(?m)(?:tauri::)?generate_handler!\s*\[([^\]]+)\]"#)
    })
}

pub(crate) fn regex_event_emit_rust() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // app.emit_all("evt", ..) or window.emit("evt", ..) etc., supports const identifiers
        regex(r#"(?m)\.\s*emit[_a-z]*\(\s*(?P<target>["'][^"']+["']|[A-Za-z_][A-Za-z0-9_]*)\s*(?:,\s*(?P<payload>[^)]*))?"#)
    })
}

pub(crate) fn regex_event_listen_rust() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // app.listen_global("evt", ..) or window.listen("evt", ..) supports const identifiers
        regex(r#"(?m)\.\s*listen[_a-z]*\(\s*(?P<target>["'][^"']+["']|[A-Za-z_][A-Za-z0-9_]*)"#)
    })
}

pub(crate) fn regex_event_const_rust() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // const EVENT: &str = "name";
        regex(r#"(?m)^\s*(?:pub\s+)?(?:const|static)\s+([A-Za-z0-9_]+)\s*:\s*&str\s*=\s*["']([^"']+)["']"#)
    })
}

pub(crate) fn regex_rust_use() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*(?:pub\s*(?:\([^)]*\))?\s+)?use\s+([^;]+);"#))
}

pub(crate) fn regex_rust_pub_use() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*pub\s*(?:\([^)]*\))?\s+use\s+([^;]+);"#))
}

pub(crate) fn regex_rust_pub_item(kind: &str) -> Regex {
    // Matches visibility modifiers like pub(crate), optional async/unsafe modifiers
    // For 'fn', also matches 'const fn' to capture const functions in impl blocks
    // Also matches associated functions inside impl blocks (not just items at line start)
    let modifiers = if kind == "fn" {
        r#"(?:(?:async|const|unsafe)\s+)*"#
    } else {
        r#"(?:(?:async|unsafe)\s+)*"#
    };
    let pattern = format!(
        r#"pub\s*(?:\([^)]*\)\s*)?{}{}\s+([A-Za-z0-9_]+)"#,
        modifiers, kind
    );
    regex(&pattern)
}

pub(crate) fn regex_rust_pub_const_like(kind: &str) -> Regex {
    // Matches pub const/static declarations anywhere (including in impl blocks)
    // Removed (?m)^\s* anchor to allow matching inside impl blocks
    // For 'const', we need to ensure it's followed by an identifier (not fn/unsafe/async)
    // This avoids matching "const fn" which should only be captured by the fn regex
    let suffix = if kind == "const" {
        // After "const ", expect an uppercase identifier (const names follow SCREAMING_SNAKE_CASE)
        // This naturally excludes "const fn/unsafe/async" which have lowercase keywords
        r#"([A-Z][A-Za-z0-9_]*)"#
    } else {
        // For static, we need to:
        // 1. Skip optional 'mut' keyword (for static mut)
        // 2. Skip 'ref' keyword (used in lazy_static! macro: pub static ref FOO)
        // 3. Then capture the actual identifier name (uppercase for constants)
        // The negative lookahead (?!ref\b|mut\b) ensures we don't capture these keywords
        r#"(?:mut\s+)?(?:ref\s+)?([A-Z][A-Za-z0-9_]*)"#
    };
    let pattern = format!(r#"pub\s*(?:\([^)]*\)\s*)?{}\s+{}"#, kind, suffix);
    regex(&pattern)
}

pub(crate) fn rust_pub_decl_regexes() -> &'static [Regex] {
    static RE: OnceLock<Vec<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        vec![
            regex_rust_pub_item("fn"),
            regex_rust_pub_item("struct"),
            regex_rust_pub_item("enum"),
            regex_rust_pub_item("trait"),
            regex_rust_pub_item("type"),
            regex_rust_pub_item("union"),
            // Note: pub mod is NOT included - modules are not exports that need to be imported
            // They are path prefixes for accessing items within the module
        ]
    })
    .as_slice()
}

pub(crate) fn rust_pub_const_regexes() -> &'static [Regex] {
    static RE: OnceLock<Vec<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        vec![
            regex_rust_pub_const_like("const"),
            regex_rust_pub_const_like("static"),
        ]
    })
    .as_slice()
}

// Rust entry point detection regexes
pub(crate) fn regex_rust_fn_main() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Match fn main() at start of line (with optional pub and async)
    RE.get_or_init(|| regex(r#"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+main\s*\("#))
}

pub(crate) fn regex_rust_async_main_attr() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Match #[tokio::main] or #[async_std::main] attributes
    RE.get_or_init(|| regex(r#"(?m)^#\[(tokio|async_std)::main\]"#))
}

// --- CSS Regexes ---

pub(crate) fn regex_css_import() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // @import "x.css";  @import url("x.css"); @import url(x.css);
        regex(r#"(?m)@import\s+(?:url\()?['"]?([^"'()\s]+)['"]?\)?"#)
    })
}

// --- Python Regexes ---

pub(crate) fn regex_py_dynamic_importlib() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"importlib\.import_module\(\s*([^)]+?)\s*(?:,|\))"#))
}

pub(crate) fn regex_py_dynamic_dunder() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"__import__\(\s*([^)]+?)\s*(?:,|\))"#))
}

pub(crate) fn regex_py_all() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?s)__all__\s*=\s*\[([^\]]*)\]"#))
}

#[allow(dead_code)]
pub(crate) fn regex_py_def() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)"#))
}

#[allow(dead_code)]
pub(crate) fn regex_py_class() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)"#))
}
