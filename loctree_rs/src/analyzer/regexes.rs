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
    RE.get_or_init(|| regex(r#"safeInvoke\s*(?:<[^>]*>+)?\(\s*["']([^"']+)["']"#))
}

pub(crate) fn regex_invoke_snake() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"invokeSnake\s*(?:<[^>]*>+)?\(\s*["']([^"']+)["']"#))
}

pub(crate) fn regex_invoke_audio() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // capture invokeAudio(...) and invokeAudioCamel(...) helpers used by FE audio API
    RE.get_or_init(|| {
        regex(r#"invokeAudio(?:Camel)?\s*(?:<[^>]*>+)?\(\s*["']([^"']+)["']"#)
    })
}

pub(crate) fn regex_tauri_command_fn() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex(r#"(?m)#\s*\[\s*tauri::command([^\]]*)\]\s*(?:pub\s*(?:\([^)]*\)\s*)?)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)"#)
    })
}

pub(crate) fn regex_tauri_invoke() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Matches top-level invoke("cmd") calls (avoids foo.invoke())
        regex(r#"(?m)(?:^|[^A-Za-z0-9_\.])invoke\s*(?:<[^>]*>+)?\(\s*[\\"']([^\\"']+)[\\"']"#)
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
    RE.get_or_init(|| regex(r#"importlib\.import_module\(\s*["']([^"']+)["']"#))
}

pub(crate) fn regex_py_dynamic_dunder() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| regex(r#"__import__\(\s*["']([^"']+)\)"#))
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
