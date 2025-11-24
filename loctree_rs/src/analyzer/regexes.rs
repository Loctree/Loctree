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
    RE.get_or_init(|| {
        regex(r#"safeInvoke\s*(?:<(?P<generic>[^>]+)>+)?\(\s*["'](?P<cmd>[^"']+)["']"#)
    })
}

pub(crate) fn regex_invoke_snake() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex(r#"invokeSnake\s*(?:<(?P<generic>[^>]+)>+)?\(\s*["'](?P<cmd>[^"']+)["']"#)
    })
}

pub(crate) fn regex_invoke_audio() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // capture invokeAudio(...) and invokeAudioCamel(...) helpers used by FE audio API
    RE.get_or_init(|| {
        regex(r#"invokeAudio(?:Camel)?\s*(?:<(?P<generic>[^>]+)>+)?\(\s*["'](?P<cmd>[^"']+)["']"#)
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

pub(crate) fn regex_event_emit_js() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // @tauri-apps/api/event emit/emitTo/emitAll("event") or emit(EVENT_CONST)
        regex(r#"(?m)(?:emit(?:All|To)?|app\.emit|window\.emit)\s*\(\s*(?P<target>["'][^"']+["']|[A-Za-z_][A-Za-z0-9_]*)\s*(?:,\s*(?P<payload>[^)\n]+))?"#)
    })
}

pub(crate) fn regex_event_listen_js() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // listen/once("event") or listen(EVENT_CONST)
        regex(r#"(?m)(?:listen|once)\s*\(\s*(?P<target>["'][^"']+["']|[A-Za-z_][A-Za-z0-9_]*)"#)
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
