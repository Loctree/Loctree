use std::sync::OnceLock;

use regex::Regex;

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid regex literal")
}

pub(crate) fn regex_import() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*import\s+([^;]+?)\s+from\s+["']([^"']+)["']"#))
}

pub(crate) fn regex_side_effect_import() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*import\s+["']([^"']+)["']"#))
}

pub(crate) fn regex_reexport_star() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*export\s+\*\s+from\s+["']([^"']+)["']"#))
}

pub(crate) fn regex_reexport_named() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*export\s+\{([^}]+)\}\s+from\s+["']([^"']+)["']"#))
}

pub(crate) fn regex_dynamic_import() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"import\s*\(\s*["']([^"']+)["']\s*\)"#))
}

/// Captures lazy import patterns like:
/// - `import('./Foo').then((m) => ({ default: m.Bar }))`
/// - `import('./Foo').then(m => ({ default: m.Bar }))`
/// - `import('./Foo').then(m => { return { default: m.Bar } })`
///
/// Captures: source (module path), export (accessed symbol name)
///
/// Note: The pattern intentionally doesn't require closing braces/parens
/// to handle various callback body styles (concise vs block return).
pub(crate) fn regex_lazy_import_then() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Note: Rust regex doesn't support backreferences, so we match any word for callback param
    RE.get_or_init(|| {
        regex(r#"import\s*\(\s*["'](?P<source>[^"']+)["']\s*\)\s*\.\s*then\s*\(\s*\(?\s*\w+\s*\)?\s*=>\s*[({]\s*(?:return\s+)?\{?\s*default\s*:\s*\w+\.(?P<export>\w+)"#)
    })
}

pub(crate) fn regex_export_named_decl() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex(
            r#"(?m)^\s*export\s+(?:async\s+)?(?:function|const|let|var|class|interface|type|enum)\s+([A-Za-z0-9_.$]+)"#,
        )
    })
}

pub(crate) fn regex_export_default() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*export\s+default(?:\s+(?:async\s+)?(?:function|class)\s+([A-Za-z0-9_.$]+))?"#))
}

pub(crate) fn regex_export_brace() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*export\s+\{([^}]+)\}\s*;?"#))
}

pub(crate) fn regex_safe_invoke() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Recognize common Tauri wrappers used in FE: safeInvoke, invokeWithSession, withSessionPayload, invokeWithSessionPayload
        // Also handles this.invokeWithSession(...) pattern used in class methods
        // Allows whitespace including newlines between function name, generics, and opening paren
        regex(r#"(?:this\.)?(?:safeInvoke|invokeWithSession(?:Payload)?|withSessionPayload)\s*(?:<(?P<generic>[^>]+)>)?\s*\(\s*["'](?P<cmd>[^"']+)["']"#)
    })
}

pub(crate) fn regex_invoke_snake() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex(r#"invokeSnake\s*(?:<(?P<generic>[^>]+)>+)?\(\s*["'](?P<cmd>[^"']+)["']\s*(?:,\s*(?P<payload>[^)\n]+))?"#)
    })
}

pub(crate) fn regex_invoke_audio() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // capture invokeAudio(...) and invokeAudioCamel(...) helpers used by FE audio API
    RE.get_or_init(|| {
        regex(r#"invokeAudio(?:Camel)?\s*(?:<(?P<generic>[^>]+)>+)?\(\s*["'](?P<cmd>[^"']+)["']\s*(?:,\s*(?P<payload>[^)\n]+))?"#)
    })
}

/// Matches wrapper functions containing "invoke" or "Command" in their name
/// where the first argument is a string literal that looks like a Tauri command.
/// Examples:
/// - `invokePinCommand('get_pin_status', ...)`
/// - `invokeHelper<T>('some_command', ...)`
/// - `myInvokeWrapper('cmd_name', ...)`
pub(crate) fn regex_invoke_wrapper() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Match function names containing "invoke" or "Command" (case insensitive via pattern)
        // followed by optional generics and a string literal first argument
        // The command name must be a valid identifier (alphanumeric + underscores, no spaces)
        regex(r#"(?i)[a-z_][a-z0-9_]*(?:invoke|command)[a-z0-9_]*\s*(?:<(?P<generic>[^>]+)>)?\s*\(\s*["'](?P<cmd>[a-z][a-z0-9_]*)["']\s*(?:,\s*(?P<payload>[^)\n]+))?"#)
    })
}

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

pub(crate) fn regex_tauri_invoke() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Matches top-level invoke("cmd") calls (avoids foo.invoke())
        regex(r#"(?m)(?:^|[^A-Za-z0-9_\.])invoke\s*(?:<[^>]*>+)?\(\s*[\\"']([^\\"']+)[\\"']\s*(?:,\s*(?P<payload>[^)\n]+))?"#)
    })
}

pub(crate) fn regex_event_emit_js() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // @tauri-apps/api/event emit/emitTo/emitAll("event") or emit(EVENT_CONST) + custom wrappers (emitTauriEvent)
        regex(r#"(?m)(?:emit(?:All|To)?|app\.emit|window\.emit|emitTauriEvent)\s*\(\s*(?P<target>["'][^"']+["']|[A-Za-z_][A-Za-z0-9_]*)\s*(?:,\s*(?P<payload>[^)\n]+))?"#)
    })
}

pub(crate) fn regex_event_listen_js() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // listen/once("event") or listen(EVENT_CONST) plus custom wrappers (listenToTauriEvent)
        regex(r#"(?m)(?:listen|once|listenToTauriEvent)\s*\(\s*(?P<target>["'][^"']+["']|[A-Za-z_][A-Za-z0-9_]*)"#)
    })
}

pub(crate) fn regex_event_const_js() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex(r#"(?m)^\s*(?:export\s+)?const\s+([A-Za-z0-9_]+)\s*=\s*["']([^"']+)["']"#)
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

pub(crate) fn regex_css_import() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // @import "x.css";  @import url("x.css"); @import url(x.css);
        regex(r#"(?m)@import\s+(?:url\()?['"]?([^"'()\s]+)['"]?\)?"#)
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
    // Matches visibility modifiers like pub(crate) and optional async for fn
    let pattern = format!(
        r#"(?m)^\s*pub\s*(?:\([^)]*\)\s*)?(?:async\s+)?{}\s+([A-Za-z0-9_]+)"#,
        kind
    );
    regex(&pattern)
}

pub(crate) fn regex_rust_pub_const_like(kind: &str) -> Regex {
    let pattern = format!(
        r#"(?m)^\s*pub\s*(?:\([^)]*\)\s*)?{}\s+([A-Za-z0-9_]+)"#,
        kind
    );
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
            regex_rust_pub_item("mod"),
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

pub(crate) fn regex_py_def() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)"#))
}

pub(crate) fn regex_py_class() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"(?m)^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)"#))
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
