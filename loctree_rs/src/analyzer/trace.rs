//! Handler trace functionality - shows WHY a handler is unused/missing
//!
//! When loctree says "unused handler", AI agents want to know:
//! 1. Where is it defined in BE?
//! 2. Where is it mentioned in FE (even if not invoked)?
//! 3. What did loctree actually search for?
//! 4. Final verdict with explanation

use serde::Serialize;

use super::coverage::CommandUsage;
use crate::types::FileAnalysis;

/// A mention of a handler name in the frontend (not necessarily an invoke)
#[derive(Debug, Clone, Serialize)]
pub struct FrontendMention {
    pub file: String,
    pub line: usize,
    pub context: String, // "invoke", "allowlist", "string_literal", "comment"
    pub is_invoke: bool,
    pub snippet: Option<String>,
}

/// Backend definition of a handler
#[derive(Debug, Clone, Serialize)]
pub struct BackendDefinition {
    pub file: String,
    pub line: usize,
    pub function_name: String,
    pub exposed_name: Option<String>,
    pub is_registered: bool,
}

/// Complete trace result for a handler
#[derive(Debug, Clone, Serialize)]
pub struct TraceResult {
    pub handler_name: String,
    pub search_variations: Vec<String>,
    pub backend: Option<BackendDefinition>,
    pub frontend_invokes: Vec<FrontendMention>,
    pub frontend_mentions: Vec<FrontendMention>,
    pub files_searched: usize,
    pub verdict: String,
    pub suggestion: String,
}

/// Run a trace investigation for a handler
pub fn trace_handler(
    handler_name: &str,
    analyses: &[FileAnalysis],
    fe_commands: &CommandUsage,
    _be_commands: &CommandUsage, // Reserved for future use
    registered_handlers: &std::collections::HashSet<String>,
) -> TraceResult {
    let search_variations = generate_search_variations(handler_name);

    // Find backend definition
    let backend = find_backend_definition(handler_name, analyses, registered_handlers);

    // Find frontend invokes (actual invoke() calls)
    let frontend_invokes = find_frontend_invokes(handler_name, fe_commands, &search_variations);

    // Search for all mentions in frontend (not just invokes)
    let frontend_mentions = find_frontend_mentions(handler_name, analyses, &search_variations);

    let files_searched = analyses.len();

    // Generate verdict
    let (verdict, suggestion) = generate_verdict(
        &backend,
        &frontend_invokes,
        &frontend_mentions,
        handler_name,
    );

    TraceResult {
        handler_name: handler_name.to_string(),
        search_variations,
        backend,
        frontend_invokes,
        frontend_mentions,
        files_searched,
        verdict,
        suggestion,
    }
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn generate_search_variations(name: &str) -> Vec<String> {
    let mut variations = vec![name.to_string()];

    // Add snake_case variant
    let snake = to_snake_case(name);
    if snake != name {
        variations.push(snake);
    }

    // Add camelCase variant
    let camel = to_camel_case(name);
    if camel != name && !variations.contains(&camel) {
        variations.push(camel);
    }

    // Add with/without _command suffix
    if name.ends_with("_command") {
        variations.push(name.strip_suffix("_command").unwrap().to_string());
    } else {
        variations.push(format!("{}_command", name));
    }

    variations
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

fn find_backend_definition(
    handler_name: &str,
    analyses: &[FileAnalysis],
    registered_handlers: &std::collections::HashSet<String>,
) -> Option<BackendDefinition> {
    let normalized = normalize_name(handler_name);

    for analysis in analyses {
        if !analysis.path.ends_with(".rs") {
            continue;
        }

        for handler in &analysis.command_handlers {
            let fn_normalized = normalize_name(&handler.name);
            let exposed_normalized = handler
                .exposed_name
                .as_ref()
                .map(|e| normalize_name(e))
                .unwrap_or_default();

            if fn_normalized == normalized
                || exposed_normalized == normalized
                || normalize_name(handler_name) == fn_normalized
            {
                let is_registered = registered_handlers.contains(&handler.name);

                return Some(BackendDefinition {
                    file: analysis.path.clone(),
                    line: handler.line,
                    function_name: handler.name.clone(),
                    exposed_name: handler.exposed_name.clone(),
                    is_registered,
                });
            }
        }
    }
    None
}

fn find_frontend_invokes(
    handler_name: &str,
    fe_commands: &CommandUsage,
    search_variations: &[String],
) -> Vec<FrontendMention> {
    let mut invokes = Vec::new();
    let normalized = normalize_name(handler_name);

    for (cmd_name, locations) in fe_commands {
        let cmd_normalized = normalize_name(cmd_name);

        let matches = cmd_normalized == normalized
            || search_variations
                .iter()
                .any(|v| normalize_name(v) == cmd_normalized);

        if matches {
            for (path, line, _impl_name) in locations {
                invokes.push(FrontendMention {
                    file: path.clone(),
                    line: *line,
                    context: "invoke".to_string(),
                    is_invoke: true,
                    snippet: None,
                });
            }
        }
    }
    invokes
}

fn find_frontend_mentions(
    handler_name: &str,
    analyses: &[FileAnalysis],
    search_variations: &[String],
) -> Vec<FrontendMention> {
    let mut mentions = Vec::new();

    // For now, we can only detect mentions through command_calls
    // A more sophisticated version would grep for string literals
    for analysis in analyses {
        // Skip Rust files (we want frontend mentions)
        if analysis.path.ends_with(".rs") {
            continue;
        }

        // Check if any export names match (might be an allowlist)
        for export in &analysis.exports {
            let export_normalized = normalize_name(&export.name);
            let matches = search_variations.iter().any(|v| {
                normalize_name(v) == export_normalized || export.name.contains(handler_name)
            });

            if matches {
                mentions.push(FrontendMention {
                    file: analysis.path.clone(),
                    line: export.line.unwrap_or(0),
                    context: "export/allowlist".to_string(),
                    is_invoke: false,
                    snippet: Some(format!("export: {}", export.name)),
                });
            }
        }
    }

    mentions
}

fn generate_verdict(
    backend: &Option<BackendDefinition>,
    frontend_invokes: &[FrontendMention],
    frontend_mentions: &[FrontendMention],
    handler_name: &str,
) -> (String, String) {
    match (backend, frontend_invokes.is_empty()) {
        (None, true) => (
            format!("NOT FOUND - '{}' not defined in backend", handler_name),
            "Check spelling or add the handler to your Rust code".to_string(),
        ),
        (None, false) => (
            format!(
                "MISSING HANDLER - FE calls '{}' but BE doesn't have it",
                handler_name
            ),
            "Add #[tauri::command] handler to backend and register it".to_string(),
        ),
        (Some(be), true) => {
            let mention_note = if !frontend_mentions.is_empty() {
                format!(
                    " (found {} non-invoke mentions in FE)",
                    frontend_mentions.len()
                )
            } else {
                String::new()
            };

            let reg_note = if !be.is_registered {
                " (NOT registered in generate_handler!)"
            } else {
                ""
            };

            (
                format!(
                    "UNUSED - defined at {}:{} but never invoked{}{}",
                    be.file, be.line, reg_note, mention_note
                ),
                if be.is_registered {
                    "Either wire up invoke() in frontend or remove the handler".to_string()
                } else {
                    "Handler not in generate_handler![]. Add it there or remove if unused."
                        .to_string()
                },
            )
        }
        (Some(be), false) => (
            format!(
                "CONNECTED - defined at {}:{}, invoked {} time(s)",
                be.file,
                be.line,
                frontend_invokes.len()
            ),
            "Handler is properly connected. No action needed.".to_string(),
        ),
    }
}

/// Print trace result in human-readable format
pub fn print_trace_human(result: &TraceResult) {
    println!("\n=== TRACE: {} ===\n", result.handler_name);

    println!("Search variations: {}", result.search_variations.join(", "));
    println!("Files searched: {}", result.files_searched);
    println!();

    println!("BACKEND:");
    if let Some(be) = &result.backend {
        println!(
            "  Defined: {}:{} (fn {})",
            be.file, be.line, be.function_name
        );
        if let Some(exposed) = &be.exposed_name {
            println!("  Exposed as: {}", exposed);
        }
        println!(
            "  Registered: {}",
            if be.is_registered { "YES" } else { "NO" }
        );
    } else {
        println!("  NOT FOUND in any Rust file");
    }
    println!();

    println!("FRONTEND INVOKES ({}):", result.frontend_invokes.len());
    if result.frontend_invokes.is_empty() {
        println!("  NONE - no invoke() calls found");
    } else {
        for inv in &result.frontend_invokes {
            println!("  {}:{} ({})", inv.file, inv.line, inv.context);
        }
    }
    println!();

    if !result.frontend_mentions.is_empty() {
        println!(
            "FRONTEND MENTIONS (non-invoke) ({}):",
            result.frontend_mentions.len()
        );
        for m in &result.frontend_mentions {
            println!(
                "  {}:{} ({}){}",
                m.file,
                m.line,
                m.context,
                m.snippet
                    .as_ref()
                    .map(|s| format!(" - {}", s))
                    .unwrap_or_default()
            );
        }
        println!();
    }

    println!("VERDICT: {}", result.verdict);
    println!("SUGGESTION: {}", result.suggestion);
    println!();
}

/// Print trace result as JSON
pub fn print_trace_json(result: &TraceResult) {
    let json = serde_json::to_string_pretty(&result).expect("serialize trace result");
    println!("{}", json);
}
