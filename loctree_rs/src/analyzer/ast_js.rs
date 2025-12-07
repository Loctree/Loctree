use std::collections::HashSet;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::ImportOrExportKind;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk::walk_expression};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};
use regex::Regex;

use crate::types::{
    CommandPayloadCasing, CommandRef, EventRef, ExportSymbol, FileAnalysis, ImportEntry,
    ImportKind, ImportSymbol, ReexportEntry, ReexportKind, SignatureUse, SignatureUseKind,
    StringLiteral,
};

use super::resolvers::{TsPathResolver, resolve_reexport_target};

#[derive(Clone, Debug)]
pub struct CommandDetectionConfig {
    pub dom_exclusions: HashSet<String>,
    pub non_invoke_exclusions: HashSet<String>,
    pub invalid_command_names: HashSet<String>,
}

// Known DOM APIs to exclude from Tauri command detection
const DOM_EXCLUSIONS: &[&str] = &[
    "execCommand",
    "queryCommandState",
    "queryCommandEnabled",
    "queryCommandSupported",
    "queryCommandValue",
];

// Functions that ARE NOT Tauri invokes - ignore completely (project heuristics)
// These happen to match "invoke" or "Command" but are not actual Tauri calls
const NON_INVOKE_EXCLUSIONS: &[&str] = &[
    // React hooks that happen to have "Command" in name
    "useVoiceCommands",
    "useAssistantToolCommands",
    "useNewVisitVoiceCommands",
    "useAiTopicCommands",
    // Build tools / CLI commands (not Tauri)
    "runGitCommand",
    "executeCommand",
    "buildCommandString",
    "buildCommandArgs",
    "classifyCommand",
    // Internal tracking/context functions
    "onCommandContext",
    "enqueueCommandContext",
    "setLastCommand",
    "setCommandError",
    "recordCommandInvokeStart",
    "recordCommandInvokeFinish",
    "handleInvokeFailure",
    "isCommandMissingError",
    "isRetentionCommandMissing",
    // Collection/analysis utilities
    "collectInvokeCommands",
    "collectUsedCommandsFromRoamLogs",
    "extractInvokeCommandsFromText",
    "scanCommandsInFiles",
    "parseBackendCommands",
    "buildSessionCommandPayload",
    // Mention/slash command handlers (UI, not Tauri)
    "onMentionCommand",
    "onSlashCommand",
    // Mock/test utilities
    "invokeFallbackMock",
    "resolveMockCommand",
];

// Command names that are clearly not Tauri commands (CLI tools / tests)
const INVALID_COMMAND_NAMES: &[&str] = &[
    // CLI tools / shell commands
    "node", "npm", "pnpm", "yarn", "bun", "cargo", "rustc", "rustup", "git", "gh", "python",
    "python3", "pip", "brew", "apt", "yum", "sh", "bash", "zsh", "curl", "wget", "docker",
    "kubectl", // Generic/test names
    "test", "mock", "stub", "fake",
];

impl CommandDetectionConfig {
    pub fn new(
        dom_exclusions: &[String],
        non_invoke_exclusions: &[String],
        invalid_command_names: &[String],
    ) -> Self {
        let mut dom: HashSet<String> = DOM_EXCLUSIONS.iter().map(|s| s.to_string()).collect();
        dom.extend(dom_exclusions.iter().cloned());

        let mut non_invoke: HashSet<String> = NON_INVOKE_EXCLUSIONS
            .iter()
            .map(|s| s.to_string())
            .collect();
        non_invoke.extend(non_invoke_exclusions.iter().cloned());

        let mut invalid: HashSet<String> = INVALID_COMMAND_NAMES
            .iter()
            .map(|s| s.to_string())
            .collect();
        invalid.extend(invalid_command_names.iter().cloned());

        Self {
            dom_exclusions: dom,
            non_invoke_exclusions: non_invoke,
            invalid_command_names: invalid,
        }
    }
}

impl Default for CommandDetectionConfig {
    fn default() -> Self {
        Self::new(&[], &[], &[])
    }
}

/// Extract script content from a Svelte file
/// Handles both `<script>` and `<script lang="ts">` variants
fn extract_svelte_script(content: &str) -> String {
    extract_sfc_script(content)
}

/// Extract script content from a Vue Single File Component (SFC)
/// Handles `<script>`, `<script setup>`, `<script lang="ts">` variants
fn extract_vue_script(content: &str) -> String {
    extract_sfc_script(content)
}

/// Common SFC script extraction used by both Svelte and Vue
fn extract_sfc_script(content: &str) -> String {
    // Match <script> or <script lang="ts"> or <script module> etc.
    // Use lazy matching to capture all script blocks
    let script_regex = Regex::new(r#"<script[^>]*>([\s\S]*?)</script>"#).ok();

    if let Some(re) = script_regex {
        let mut scripts = Vec::new();
        for caps in re.captures_iter(content) {
            if let Some(script_content) = caps.get(1) {
                scripts.push(script_content.as_str().to_string());
            }
        }
        scripts.join("\n")
    } else {
        String::new()
    }
}

/// Extract template content from a Svelte file (everything outside <script> and <style>)
fn extract_svelte_template(content: &str) -> String {
    let mut result = content.to_string();
    if let Ok(script_re) = Regex::new(r#"<script[^>]*>[\s\S]*?</script>"#) {
        result = script_re.replace_all(&result, "").to_string();
    }
    if let Ok(style_re) = Regex::new(r#"<style[^>]*>[\s\S]*?</style>"#) {
        result = style_re.replace_all(&result, "").to_string();
    }
    result
}

/// Extract template content from a Vue file (everything inside <template> tags)
fn extract_vue_template(content: &str) -> String {
    let template_regex = Regex::new(r#"<template[^>]*>([\s\S]*?)</template>"#).ok();

    if let Some(re) = template_regex {
        let mut templates = Vec::new();
        for caps in re.captures_iter(content) {
            if let Some(template_content) = caps.get(1) {
                templates.push(template_content.as_str().to_string());
            }
        }
        templates.join("\n")
    } else {
        String::new()
    }
}

/// Parse Svelte template for function calls and variable references
fn parse_svelte_template_usages(template: &str) -> Vec<String> {
    let mut usages = Vec::new();

    // Pattern 1: Function calls {funcName()} or {funcName(args)}
    if let Ok(re) = Regex::new(r#"\{[^}]*?\b([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\("#) {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !is_svelte_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 2: Event handlers on:click={handler}
    // Also captures arrow functions: on:click={() => save(item)}
    if let Ok(re) = Regex::new(r#"on:\w+\s*=\s*\{(?:\([^)]*\)\s*=>)?\s*([a-zA-Z_$][a-zA-Z0-9_$]*)"#)
    {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !is_svelte_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 2b: Extract function names from arrow function bodies
    // Matches: on:click={() => functionName(...)} or on:click={(e) => handler(e)}
    if let Ok(re) =
        Regex::new(r#"on:\w+\s*=\s*\{(?:\([^)]*\))?\s*=>\s*([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\("#)
    {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !is_svelte_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 3: Bind directives bind:value={varName}
    if let Ok(re) = Regex::new(r#"bind:\w+\s*=\s*\{([a-zA-Z_$][a-zA-Z0-9_$]*)"#) {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !is_svelte_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 4: Use directives use:action
    if let Ok(re) = Regex::new(r#"use:([a-zA-Z_$][a-zA-Z0-9_$]*)"#) {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !is_svelte_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 5: Transition directives transition:fade, in:fly, out:slide
    if let Ok(re) = Regex::new(r#"(?:transition|in|out|animate):([a-zA-Z_$][a-zA-Z0-9_$]*)"#) {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !is_svelte_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 6: Component usage <ComponentName />
    if let Ok(re) = Regex::new(r#"<([A-Z][a-zA-Z0-9_$]*)"#) {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 7: Prop passing propName={value}
    if let Ok(re) =
        Regex::new(r#"\s[a-zA-Z_$][a-zA-Z0-9_$]*\s*=\s*\{([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\}"#)
    {
        for caps in re.captures_iter(template) {
            if let Some(value) = caps.get(1) {
                let ident = value.as_str().to_string();
                if !is_svelte_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    usages
}

/// Parse Vue template for function calls and variable references
fn parse_vue_template_usages(template: &str) -> Vec<String> {
    let mut usages = Vec::new();

    // Pattern 1: Mustache interpolations {{ functionName(...) }} - function calls
    if let Ok(re) = Regex::new(r#"\{\{[^}]*?\b([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\("#) {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !is_vue_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 1b: Mustache interpolations {{ variable.property }} - variable references
    // Captures the root variable name (before the dot)
    if let Ok(re) = Regex::new(r#"\{\{\s*([a-zA-Z_$][a-zA-Z0-9_$]*)\.?"#) {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !is_vue_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 2: Event handlers @click="handler" or v-on:click="handler"
    if let Ok(re) = Regex::new(r#"(?:@|v-on:)\w+\s*=\s*"([a-zA-Z_$][a-zA-Z0-9_$]*)"#) {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !is_vue_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 3: Prop bindings :prop="computedValue" or v-bind:prop="value"
    if let Ok(re) = Regex::new(r#"(?::|v-bind:)\w+\s*=\s*"([a-zA-Z_$][a-zA-Z0-9_$]*)"#) {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !is_vue_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 4: v-model bindings
    if let Ok(re) = Regex::new(r#"v-model\s*=\s*"([a-zA-Z_$][a-zA-Z0-9_$]*)"#) {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !is_vue_builtin(&ident) && !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    // Pattern 5: Component usage <ComponentName />
    if let Ok(re) = Regex::new(r#"<([A-Z][a-zA-Z0-9_$]*)"#) {
        for caps in re.captures_iter(template) {
            if let Some(name) = caps.get(1) {
                let ident = name.as_str().to_string();
                if !usages.contains(&ident) {
                    usages.push(ident);
                }
            }
        }
    }

    usages
}

/// Check if an identifier is a Vue built-in or control flow keyword
fn is_vue_builtin(name: &str) -> bool {
    matches!(
        name,
        "if" | "else"
            | "for"
            | "slot"
            | "component"
            | "transition"
            | "keep-alive"
            | "teleport"
            | "suspense"
            | "console"
            | "window"
            | "document"
            | "Array"
            | "Object"
            | "String"
            | "Number"
            | "Boolean"
            | "Date"
            | "Math"
            | "JSON"
            | "Promise"
            | "Error"
            | "undefined"
            | "null"
            | "true"
            | "false"
            | "this"
    )
}

/// Check if an identifier is a Svelte built-in or control flow keyword
fn is_svelte_builtin(name: &str) -> bool {
    matches!(
        name,
        "if" | "else"
            | "each"
            | "await"
            | "then"
            | "catch"
            | "key"
            | "html"
            | "debug"
            | "const"
            | "let"
            | "var"
            | "console"
            | "window"
            | "document"
            | "Array"
            | "Object"
            | "String"
            | "Number"
            | "Boolean"
            | "Date"
            | "Math"
            | "JSON"
            | "Promise"
            | "Error"
            | "undefined"
            | "null"
            | "true"
            | "false"
            | "this"
            | "slot"
            | "svelte"
    )
}

/// Analyze JS/TS file using OXC AST parser
pub(crate) fn analyze_js_file_ast(
    content: &str,
    path: &Path,
    root: &Path,
    extensions: Option<&HashSet<String>>,
    ts_resolver: Option<&TsPathResolver>,
    relative: String,
    command_cfg: &CommandDetectionConfig,
) -> FileAnalysis {
    let allocator = Allocator::default();

    // Determine source type from file extension
    // Only enable JSX for .tsx/.jsx files to avoid conflicts with TypeScript generics
    // (e.g., `const fn = <T>(...) =>` would be parsed as JSX tag with JSX enabled)
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let is_jsx_file = ext == "tsx" || ext == "jsx";
    let is_svelte_file = ext == "svelte";
    let is_vue_file = ext == "vue";
    let is_sfc_file = is_svelte_file || is_vue_file;

    // For SFC files (Svelte/Vue), extract script content first
    let parsed_content: String;
    let content_to_parse = if is_svelte_file {
        parsed_content = extract_svelte_script(content);
        parsed_content.as_str()
    } else if is_vue_file {
        parsed_content = extract_vue_script(content);
        parsed_content.as_str()
    } else {
        content
    };

    // For SFC files (Svelte/Vue), parse as TypeScript
    let source_type = if is_sfc_file {
        SourceType::tsx().with_typescript(true)
    } else {
        SourceType::from_path(path)
            .unwrap_or_default()
            .with_typescript(true)
            .with_jsx(is_jsx_file)
    };

    let ret = Parser::new(&allocator, content_to_parse, source_type).parse();

    // Log parser errors for debugging (verbose mode only)
    if !ret.errors.is_empty() && std::env::var("LOCTREE_VERBOSE").is_ok() {
        eprintln!(
            "[loctree][debug] Parser errors in {}: {} errors",
            path.display(),
            ret.errors.len()
        );
        for (i, err) in ret.errors.iter().take(5).enumerate() {
            // Get line number from error span using the labels field
            let line_info = err
                .labels
                .as_ref()
                .and_then(|labels| labels.first())
                .map(|label| {
                    let offset = label.offset();
                    let line = content[..offset].bytes().filter(|b| *b == b'\n').count() + 1;
                    format!(" (line {}, col {})", line, label.offset())
                })
                .unwrap_or_default();
            eprintln!("  [{}]{} {}", i + 1, line_info, err);
        }
    }

    let mut visitor = JsVisitor {
        analysis: FileAnalysis::new(relative),
        path,
        root,
        extensions,
        ts_resolver,
        source_text: content_to_parse,
        command_cfg,
    };

    visitor.visit_program(&ret.program);

    // Use oxc_semantic to track local symbol references
    // This helps detect when exported symbols are used internally (not dead)
    let semantic_ret = SemanticBuilder::new().build(&ret.program);
    if semantic_ret.errors.is_empty() {
        let semantic = semantic_ret.semantic;

        // Build set of exported symbol names for quick lookup
        let exported_names: HashSet<&str> = visitor
            .analysis
            .exports
            .iter()
            .map(|e| e.name.as_str())
            .collect();

        // Check each symbol - if it's exported AND has references, it's used locally
        for symbol_id in semantic.scoping().symbol_ids() {
            let name = semantic.scoping().symbol_name(symbol_id);
            if exported_names.contains(name) {
                // Check if this symbol has any references (beyond its declaration)
                let ref_ids = semantic.scoping().get_resolved_reference_ids(symbol_id);
                if !ref_ids.is_empty() {
                    visitor.analysis.local_uses.push(name.to_string());
                }
            }
        }
    }

    // For Svelte files, also parse the template section to detect function calls
    // This prevents false positives where exported functions are used in the template
    // e.g., {badgeText(account)} or on:click={handleClick}
    if is_svelte_file {
        let template = extract_svelte_template(content);
        let template_usages = parse_svelte_template_usages(&template);
        for usage in template_usages {
            if !visitor.analysis.local_uses.contains(&usage) {
                visitor.analysis.local_uses.push(usage);
            }
        }
    }

    // For Vue files, also parse the template section to detect function calls
    // This prevents false positives where exported functions are used in the template
    // e.g., {{ formatDate(value) }} or @click="handleClick"
    if is_vue_file {
        let template = extract_vue_template(content);
        let template_usages = parse_vue_template_usages(&template);
        for usage in template_usages {
            if !visitor.analysis.local_uses.contains(&usage) {
                visitor.analysis.local_uses.push(usage);
            }
        }
    }

    visitor.analysis
}

struct JsVisitor<'a> {
    analysis: FileAnalysis,
    path: &'a Path,
    root: &'a Path,
    extensions: Option<&'a HashSet<String>>,
    ts_resolver: Option<&'a TsPathResolver>,
    source_text: &'a str,
    command_cfg: &'a CommandDetectionConfig,
}

impl<'a> JsVisitor<'a> {
    fn resolve_path(&self, source: &str) -> Option<String> {
        let file_ext = self
            .path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        // For TS/JS files, skip resolve_reexport_target (uses Python logic)
        // Go straight to TS resolver or JS relative resolution
        let skip_python = matches!(
            file_ext.as_deref(),
            Some("ts") | Some("tsx") | Some("js") | Some("jsx") | Some("mjs") | Some("cjs")
        );

        let initial = if skip_python {
            None
        } else {
            resolve_reexport_target(self.path, self.root, source, self.extensions)
        };

        initial
            .or_else(|| {
                self.ts_resolver
                    .and_then(|r| r.resolve(source, self.extensions))
            })
            .or_else(|| {
                super::resolvers::resolve_js_relative(self.path, self.root, source, self.extensions)
            })
    }

    fn get_line(&self, span: Span) -> usize {
        let start = span.start as usize;
        let capped = std::cmp::min(start, self.source_text.len());
        self.source_text[..capped]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            + 1
    }

    fn push_string_literal(&mut self, value: &str, span: Span) {
        let line = self.get_line(span);
        self.analysis.string_literals.push(StringLiteral {
            value: value.to_string(),
            line,
        });
    }

    /// Extract basic type representation from TSType
    fn type_to_string(ty: &TSType<'a>) -> String {
        match ty {
            TSType::TSTypeReference(r) => JsVisitor::type_name_to_string(&r.type_name),
            // When the type is a complex union/inline construct, return a neutral label
            // so we don't bloat command payloads with full type ASTs.
            _ => "Type".to_string(),
        }
    }

    fn type_name_to_string(name: &TSTypeName<'a>) -> String {
        match name {
            TSTypeName::IdentifierReference(id) => id.name.to_string(),
            TSTypeName::QualifiedName(q) => {
                format!(
                    "{}.{}",
                    JsVisitor::type_name_to_string(&q.left),
                    q.right.name
                )
            }
            TSTypeName::ThisExpression(_) => "This".to_string(),
        }
    }

    fn record_type_use(
        &mut self,
        fn_name: &str,
        usage: SignatureUseKind,
        ty: &TSType<'a>,
        span: Span,
    ) {
        let type_name = JsVisitor::type_to_string(ty);
        if type_name.is_empty() || type_name == "Type" {
            return;
        }
        let line = self.get_line(span);
        if !self.analysis.local_uses.contains(&type_name) {
            self.analysis.local_uses.push(type_name.clone());
        }
        self.analysis.signature_uses.push(SignatureUse {
            function: fn_name.to_string(),
            usage,
            type_name,
            line: Some(line),
        });
    }

    fn record_param_types(&mut self, fn_name: &str, params: &FormalParameters<'a>) {
        for param in params.items.iter() {
            if let Some(ann) = &param.pattern.type_annotation {
                self.record_type_use(
                    fn_name,
                    SignatureUseKind::Parameter,
                    &ann.type_annotation,
                    ann.span,
                );
            }
        }
        if let Some(rest) = &params.rest
            && let Some(ann) = &rest.argument.type_annotation
        {
            self.record_type_use(
                fn_name,
                SignatureUseKind::Parameter,
                &ann.type_annotation,
                ann.span,
            );
        }
    }

    fn record_function_signature(&mut self, fn_name: &str, func: &Function<'a>) {
        if let Some(ret) = &func.return_type {
            self.record_type_use(
                fn_name,
                SignatureUseKind::Return,
                &ret.type_annotation,
                ret.span,
            );
        }
        self.record_param_types(fn_name, &func.params);
    }

    fn record_arrow_signature(&mut self, fn_name: &str, func: &ArrowFunctionExpression<'a>) {
        if let Some(ret) = &func.return_type {
            self.record_type_use(
                fn_name,
                SignatureUseKind::Return,
                &ret.type_annotation,
                ret.span,
            );
        }
        self.record_param_types(fn_name, &func.params);
    }
}

impl<'a> Visit<'a> for JsVisitor<'a> {
    fn visit_expression(&mut self, expr: &Expression<'a>) {
        match expr {
            Expression::StringLiteral(lit) => {
                self.push_string_literal(&lit.value, lit.span);
            }
            Expression::TemplateLiteral(tpl) => {
                if tpl.expressions.is_empty()
                    && tpl.quasis.len() == 1
                    && let Some(cooked) = &tpl.quasis[0].value.cooked
                {
                    self.push_string_literal(cooked, tpl.span);
                } else if tpl.expressions.is_empty() && tpl.quasis.len() == 1 {
                    self.push_string_literal(&tpl.quasis[0].value.raw, tpl.span);
                }
            }
            _ => {}
        }
        walk_expression(self, expr);
    }

    // --- IMPORTS ---

    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        let source = decl.source.value.to_string();
        let mut entry = ImportEntry::new(source.clone(), ImportKind::Static);
        entry.resolved_path = self.resolve_path(&source);
        entry.is_bare = !source.starts_with('.') && !source.starts_with('/');
        if matches!(decl.import_kind, ImportOrExportKind::Type) {
            entry.kind = ImportKind::Type;
        }

        if let Some(specifiers) = &decl.specifiers {
            for spec in specifiers {
                match spec {
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                        entry.symbols.push(ImportSymbol {
                            name: s.local.name.to_string(),
                            alias: None,
                            is_default: true,
                        });
                    }
                    ImportDeclarationSpecifier::ImportSpecifier(s) => {
                        let name = match &s.imported {
                            ModuleExportName::IdentifierName(id) => id.name.to_string(),
                            ModuleExportName::IdentifierReference(id) => id.name.to_string(),
                            ModuleExportName::StringLiteral(str) => str.value.to_string(),
                        };

                        // Fix cmp_owned: compare &str directly
                        let alias = if s.local.name != name {
                            Some(s.local.name.to_string())
                        } else {
                            None
                        };

                        entry.symbols.push(ImportSymbol {
                            name,
                            alias,
                            is_default: false,
                        });
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                        entry.symbols.push(ImportSymbol {
                            name: "*".to_string(),
                            alias: Some(s.local.name.to_string()),
                            is_default: false,
                        });
                    }
                }
            }
        } else {
            // Side-effect import
            entry.kind = ImportKind::SideEffect;
        }
        self.analysis.imports.push(entry);
    }

    // --- EXPORTS ---

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        let line = self.get_line(decl.span);

        if let Some(src) = &decl.source {
            // Re-export: export { foo } from 'bar';
            let source = src.value.to_string();
            let resolved = self.resolve_path(&source);
            let mut names = Vec::new();

            for spec in &decl.specifiers {
                let name = match &spec.local {
                    ModuleExportName::IdentifierName(id) => id.name.to_string(),
                    ModuleExportName::IdentifierReference(id) => id.name.to_string(),
                    ModuleExportName::StringLiteral(str) => str.value.to_string(),
                };
                names.push(name);
            }

            self.analysis.reexports.push(ReexportEntry {
                source,
                kind: ReexportKind::Named(names.clone()),
                resolved,
            });

            // Also track as exports
            for name in names {
                self.analysis.exports.push(ExportSymbol::new(
                    name,
                    "reexport",
                    "named",
                    Some(line),
                ));
            }
        } else {
            // Named export: export const x = 1;
            if let Some(declaration) = &decl.declaration {
                match declaration {
                    Declaration::VariableDeclaration(var) => {
                        for d in &var.declarations {
                            if let BindingPatternKind::BindingIdentifier(id) = &d.id.kind {
                                let name = id.name.to_string();
                                self.analysis.exports.push(ExportSymbol::new(
                                    name,
                                    "var",
                                    "named",
                                    Some(line),
                                ));
                                if let Some(init) = &d.init {
                                    if let Expression::FunctionExpression(fun) = init {
                                        self.record_function_signature(id.name.as_str(), fun);
                                    } else if let Expression::ArrowFunctionExpression(fun) = init {
                                        self.record_arrow_signature(id.name.as_str(), fun);
                                    }
                                }
                            }
                        }
                        // Continue traversal
                        self.visit_variable_declaration(var);
                    }
                    Declaration::FunctionDeclaration(f) => {
                        if let Some(id) = &f.id {
                            let name = id.name.to_string();
                            self.analysis.exports.push(ExportSymbol::new(
                                name,
                                "function",
                                "named",
                                Some(line),
                            ));
                            self.record_function_signature(id.name.as_str(), f);
                        }
                        // Continue traversal
                        if let Some(body) = &f.body {
                            self.visit_function_body(body);
                        }
                    }
                    Declaration::ClassDeclaration(c) => {
                        if let Some(id) = &c.id {
                            let name = id.name.to_string();
                            self.analysis.exports.push(ExportSymbol::new(
                                name,
                                "class",
                                "named",
                                Some(line),
                            ));
                        }
                        // Continue traversal
                        self.visit_class(c);
                    }
                    Declaration::TSInterfaceDeclaration(i) => {
                        let name = i.id.name.to_string();
                        self.analysis.exports.push(ExportSymbol::new(
                            name,
                            "interface",
                            "named",
                            Some(line),
                        ));
                    }
                    Declaration::TSTypeAliasDeclaration(t) => {
                        let name = t.id.name.to_string();
                        self.analysis.exports.push(ExportSymbol::new(
                            name,
                            "type",
                            "named",
                            Some(line),
                        ));
                    }
                    Declaration::TSEnumDeclaration(e) => {
                        let name = e.id.name.to_string();
                        self.analysis.exports.push(ExportSymbol::new(
                            name,
                            "enum",
                            "named",
                            Some(line),
                        ));
                    }
                    _ => {}
                }
            }

            // export { foo };
            for spec in &decl.specifiers {
                let name = match &spec.exported {
                    ModuleExportName::IdentifierName(id) => id.name.to_string(),
                    ModuleExportName::IdentifierReference(id) => id.name.to_string(),
                    ModuleExportName::StringLiteral(str) => str.value.to_string(),
                };
                self.analysis
                    .exports
                    .push(ExportSymbol::new(name, "named", "named", Some(line)));
            }
        }
    }

    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        let line = self.get_line(decl.span);
        // Default exports are always named "default" for matching with `import X from './file'`
        // The actual function/class name is stored in export_type for debugging only
        match &decl.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                let original_name = f.id.as_ref().map(|i| i.name.to_string());
                self.analysis.exports.push(ExportSymbol::new(
                    "default".to_string(),
                    "default",
                    original_name.as_deref().unwrap_or("default"),
                    Some(line),
                ));
                if let Some(name) = &original_name {
                    self.record_function_signature(name, f);
                }

                // Continue traversal
                if let Some(body) = &f.body {
                    self.visit_function_body(body);
                }
            }
            ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                let original_name = c.id.as_ref().map(|i| i.name.to_string());
                self.analysis.exports.push(ExportSymbol::new(
                    "default".to_string(),
                    "default",
                    original_name.as_deref().unwrap_or("default"),
                    Some(line),
                ));

                // Continue traversal
                self.visit_class(c);
            }
            ExportDefaultDeclarationKind::TSInterfaceDeclaration(i) => {
                self.analysis.exports.push(ExportSymbol::new(
                    "default".to_string(),
                    "default",
                    &i.id.name,
                    Some(line),
                ));
                // Interfaces don't have executable code bodies (calls), so no need to traverse deep for commands
            }
            _ => {
                self.analysis.exports.push(ExportSymbol::new(
                    "default".to_string(),
                    "default",
                    "default",
                    Some(line),
                ));
            }
        };
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        let source = decl.source.value.to_string();
        let resolved = self.resolve_path(&source);
        self.analysis.reexports.push(ReexportEntry {
            source,
            kind: ReexportKind::Star,
            resolved,
        });
    }

    // --- DYNAMIC IMPORTS (import("...")) ---

    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
        // Handle import("./foo")
        if let Expression::StringLiteral(s) = &expr.source {
            let source = s.value.to_string();

            // Track in dynamic_imports for backward compatibility
            if !self.analysis.dynamic_imports.contains(&source) {
                self.analysis.dynamic_imports.push(source.clone());
            }

            // NEW: Also create an ImportEntry with Dynamic kind for graph edges
            let mut entry = ImportEntry::new(source.clone(), ImportKind::Dynamic);
            entry.resolved_path = self.resolve_path(&source);
            entry.is_bare = !source.starts_with('.') && !source.starts_with('/');
            // Dynamic imports don't have specific symbols - they import the whole module
            self.analysis.imports.push(entry);

            // Also track resolved path in dynamic_imports if available
            if let Some(resolved) = self.resolve_path(&source)
                && !self.analysis.dynamic_imports.contains(&resolved)
            {
                self.analysis.dynamic_imports.push(resolved);
            }
        }

        // Continue visiting children
        self.visit_expression(&expr.source);
        if let Some(opts) = &expr.options {
            self.visit_expression(opts);
        }
    }

    // --- CALL EXPRESSIONS (invoke, etc) ---

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        // Continue visiting children (callee/args may contain nested invocations)
        self.visit_arguments(&call.arguments);
        self.visit_expression(&call.callee);

        let callee_name = match &call.callee {
            Expression::Identifier(ident) => Some(ident.name.to_string()),

            Expression::StaticMemberExpression(member) => {
                // Handle obj.emit(...)

                Some(member.property.name.to_string())
            }

            _ => None,
        };

        if let Some(name) = callee_name {
            // Commands - detect Tauri invoke patterns
            let name_lower = name.to_lowercase();
            let is_potential_command = name_lower.contains("invoke") || name.contains("Command");

            if is_potential_command
                && !self.command_cfg.dom_exclusions.contains(&name)
                && !self.command_cfg.non_invoke_exclusions.contains(&name)
                && let Some(arg) = call.arguments.first()
            {
                // Extract command name from first argument (string literal or template literal)
                let cmd_name = match arg {
                    Argument::StringLiteral(s) => Some(s.value.to_string()),
                    Argument::TemplateLiteral(t) => {
                        // Only extract if it's a simple template without expressions
                        if t.quasis.len() == 1 && t.expressions.is_empty() {
                            t.quasis.first().map(|q| q.value.raw.to_string())
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                // Only record command if we have an actual command name (from the argument).
                // Skip if cmd_name is None - that means we couldn't extract the command name
                // (e.g., dynamic command name or wrapper function definition).
                if let Some(cmd_name) = cmd_name {
                    // Filter out command names that are clearly not Tauri commands
                    // (e.g., CLI tools, shell commands found in scripts/config files)
                    if self.command_cfg.invalid_command_names.contains(&cmd_name) {
                        // Skip - not a real Tauri command
                    } else {
                        // Payload casing drift: if command name looks snake_case and payload keys are camelCase
                        let mut casing_issues: Vec<CommandPayloadCasing> = Vec::new();
                        if cmd_name.contains('_')
                            && let Some(Argument::ObjectExpression(obj)) = call.arguments.first()
                        {
                            for prop in &obj.properties {
                                if let ObjectPropertyKind::ObjectProperty(p) = prop
                                    && let PropertyKey::Identifier(id) = &p.key
                                {
                                    let key = id.name.to_string();
                                    if key.chars().any(|c| c.is_uppercase()) {
                                        casing_issues.push(CommandPayloadCasing {
                                            command: cmd_name.clone(),
                                            key,
                                            path: self.path.to_string_lossy().to_string(),
                                            line: self.get_line(p.span),
                                        });
                                    }
                                }
                            }
                        }
                        self.analysis.command_payload_casing.extend(casing_issues);

                        let generic = call.type_arguments.as_ref().and_then(|params| {
                            params.params.first().map(JsVisitor::type_to_string)
                        });

                        let line = self.get_line(call.span);

                        self.analysis.command_calls.push(CommandRef {
                            name: cmd_name,
                            exposed_name: None,
                            line,
                            generic_type: generic,
                            payload: None,
                        });
                    }
                }
            }

            // Events: emit / listen
            // Heuristic: function name contains "emit" or "listen"
            let is_emit = name == "emit" || name.ends_with("emit"); // e.g. window.emit, appWindow.emit
            let is_listen = name == "listen" || name.contains("listen"); // e.g. appWindow.listen, listenTo

            // Fix collapsible_if
            if (is_emit || is_listen)
                && let Some(arg) = call.arguments.first()
            {
                // Resolve event name from argument (literal or constant)
                let (event_name, raw_name, kind) = match arg {
                    Argument::StringLiteral(s) => {
                        (s.value.to_string(), Some(s.value.to_string()), "literal")
                    }
                    Argument::TemplateLiteral(t) => {
                        if t.quasis.len() == 1 && t.expressions.is_empty() {
                            if let Some(q) = t.quasis.first() {
                                (
                                    q.value.raw.to_string(),
                                    Some(q.value.raw.to_string()),
                                    "literal",
                                )
                            } else {
                                ("?".to_string(), None, "unknown")
                            }
                        } else {
                            ("?".to_string(), None, "unknown")
                        }
                    }
                    Argument::Identifier(id) => {
                        let id_name = id.name.to_string();
                        if let Some(val) = self.analysis.event_consts.get(&id_name) {
                            (val.clone(), Some(id_name), "const")
                        } else {
                            (id_name.clone(), Some(id_name), "ident")
                        }
                    }
                    _ => ("?".to_string(), None, "unknown"),
                };

                let line = self.get_line(call.span);

                if is_emit {
                    let payload = call.arguments.get(1).map(|_| "payload".to_string()); // Simplified payload detection
                    self.analysis.event_emits.push(EventRef {
                        raw_name,
                        name: event_name,
                        line,
                        kind: format!("emit_{}", kind),
                        awaited: false, // Todo: check await parent
                        payload,
                    });
                } else {
                    // listen
                    // Todo: check await parent
                    self.analysis.event_listens.push(EventRef {
                        raw_name,
                        name: event_name,
                        line,
                        kind: format!("listen_{}", kind),
                        awaited: false,
                        payload: None,
                    });
                }
            }
        }
    }

    // Capture constants for event resolution: const MY_EVENT = "event-name";
    fn visit_variable_declarator(&mut self, decl: &VariableDeclarator<'a>) {
        // Fix collapsible_match and collapsible_if
        if let BindingPatternKind::BindingIdentifier(id) = &decl.id.kind
            && let Some(init) = &decl.init
            && let Expression::StringLiteral(s) = init
        {
            // Store const name -> value mapping
            self.analysis
                .event_consts
                .insert(id.name.to_string(), s.value.to_string());
        }

        // IMPORTANT: Continue visiting children (e.g. init expression might contain dynamic imports)
        // Manually visit children since we overrode the default implementation
        self.visit_binding_pattern(&decl.id);
        if let Some(init) = &decl.init {
            self.visit_expression(init);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_ast_parsing_basic() {
        let content = r#" 
            import { Foo } from "./bar";
            import Default, { Named } from "./baz";
            import * as NS from "./ns";
            
            export const myVar = 1;
            export function myFunc() {}
            export default class MyClass {}
            export { reexported } from "./other";
            
            invoke("my_command");
            safeInvoke("another_command");
        "#;

        let analysis = analyze_js_file_ast(
            content,
            Path::new("src/test.ts"),
            Path::new("src"),
            None,
            None,
            "test.ts".to_string(),
            &CommandDetectionConfig::default(),
        );

        // Imports
        assert_eq!(analysis.imports.len(), 3);

        let bar = analysis
            .imports
            .iter()
            .find(|i| i.source == "./bar")
            .unwrap();
        assert_eq!(bar.symbols[0].name, "Foo");
        assert!(!bar.symbols[0].is_default);

        let baz = analysis
            .imports
            .iter()
            .find(|i| i.source == "./baz")
            .unwrap();
        assert_eq!(baz.symbols.len(), 2);
        assert!(
            baz.symbols
                .iter()
                .any(|s| s.name == "Default" && s.is_default)
        );
        assert!(
            baz.symbols
                .iter()
                .any(|s| s.name == "Named" && !s.is_default)
        );

        let ns = analysis
            .imports
            .iter()
            .find(|i| i.source == "./ns")
            .unwrap();
        assert_eq!(ns.symbols[0].name, "*");
        assert_eq!(ns.symbols[0].alias.as_deref(), Some("NS"));

        // Exports
        let exports: Vec<_> = analysis.exports.iter().map(|e| e.name.as_str()).collect();
        assert!(exports.contains(&"myVar"));
        assert!(exports.contains(&"myFunc"));
        // Default exports are now named "default" for proper import matching
        assert!(exports.contains(&"default"));
        // The original class name is preserved in export_type
        assert!(
            analysis
                .exports
                .iter()
                .any(|e| e.name == "default" && e.export_type == "MyClass")
        );
        assert!(exports.contains(&"reexported"));

        // Commands
        let commands: Vec<_> = analysis
            .command_calls
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(commands.contains(&"my_command"));
        assert!(commands.contains(&"another_command"));
    }

    #[test]
    fn test_ast_events_and_consts() {
        let content = r#" 
            const MY_EVENT = "user-login";
            const ANOTHER_EVENT = "data-update";
            
            // Literal emit
            emit("literal-event", { id: 1 });
            
            // Constant emit
            emit(MY_EVENT, "payload");
            
            // Listen
            listen(ANOTHER_EVENT, () => {});
            appWindow.listen("window-event", handler);
        "#;

        let analysis = analyze_js_file_ast(
            content,
            Path::new("src/events.ts"),
            Path::new("src"),
            None,
            None,
            "events.ts".to_string(),
            &CommandDetectionConfig::default(),
        );

        // Constants
        assert_eq!(
            analysis.event_consts.get("MY_EVENT").map(|s| s.as_str()),
            Some("user-login")
        );
        assert_eq!(
            analysis
                .event_consts
                .get("ANOTHER_EVENT")
                .map(|s| s.as_str()),
            Some("data-update")
        );

        // Emits
        let emits: Vec<_> = analysis
            .event_emits
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert!(emits.contains(&"literal-event"));
        assert!(emits.contains(&"user-login")); // Resolved from const

        // Listens
        let listens: Vec<_> = analysis
            .event_listens
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert!(listens.contains(&"data-update")); // Resolved from const
        assert!(listens.contains(&"window-event"));
    }

    #[test]
    fn test_dynamic_imports_added_to_imports_list() {
        let content = r#"
            // Regular static import
            import { Button } from './Button';

            // Dynamic import with import()
            const LazyComponent = import('./LazyComponent');

            // React.lazy with dynamic import
            const LazyPage = React.lazy(() => import('./pages/Home'));
        "#;

        let analysis = analyze_js_file_ast(
            content,
            Path::new("src/App.tsx"),
            Path::new("src"),
            None,
            None,
            "App.tsx".to_string(),
            &CommandDetectionConfig::default(),
        );

        // Check that static import is captured
        assert!(analysis.imports.iter().any(|i| i.source == "./Button"));

        // Check that dynamic imports are captured in imports list (not just dynamic_imports)
        assert!(
            analysis
                .imports
                .iter()
                .any(|i| i.source == "./LazyComponent")
        );
        assert!(analysis.imports.iter().any(|i| i.source == "./pages/Home"));

        // Verify they're marked as Dynamic kind
        let lazy_import = analysis
            .imports
            .iter()
            .find(|i| i.source == "./LazyComponent")
            .unwrap();
        assert!(matches!(lazy_import.kind, ImportKind::Dynamic));
    }

    #[test]
    fn test_vue_sfc_script_extraction() {
        // Vue SFC with script setup (Composition API)
        let content = r#"
<script setup lang="ts">
import { ref, computed } from 'vue'

const count = ref(0)

export function increment() {
    count.value++
}
</script>

<template>
  <div>{{ count }}</div>
</template>
        "#;

        let analysis = analyze_js_file_ast(
            content,
            Path::new("src/Counter.vue"),
            Path::new("src"),
            None,
            None,
            "Counter.vue".to_string(),
            &CommandDetectionConfig::default(),
        );

        // Verify imports are detected
        assert!(
            analysis.imports.iter().any(|i| i.source == "vue"),
            "Should detect vue import"
        );

        // Verify exports are detected
        assert!(
            analysis
                .exports
                .iter()
                .any(|e| e.name == "increment" && e.kind == "function"),
            "Should detect increment export"
        );
    }

    #[test]
    fn test_vue_sfc_options_api() {
        // Vue SFC with Options API
        let content = r#"
<script lang="ts">
import { defineComponent } from 'vue'

export default defineComponent({
    data() {
        return { count: 0 }
    }
})
</script>

<template>
  <div>{{ count }}</div>
</template>
        "#;

        let analysis = analyze_js_file_ast(
            content,
            Path::new("src/Counter.vue"),
            Path::new("src"),
            None,
            None,
            "Counter.vue".to_string(),
            &CommandDetectionConfig::default(),
        );

        // Verify import is detected
        assert!(
            analysis.imports.iter().any(|i| i.source == "vue"),
            "Should detect vue import"
        );

        // Verify default export is detected
        assert!(
            analysis.exports.iter().any(|e| e.export_type == "default"),
            "Should detect default export"
        );
    }

    #[test]
    fn test_vue_script_extraction_basic() {
        let vue_content = r#"
<script>
const message = 'Hello'
export const greeting = message + ' World'
</script>

<template>
  <div>{{ greeting }}</div>
</template>
        "#;

        let extracted = extract_vue_script(vue_content);
        assert!(extracted.contains("const message = 'Hello'"));
        assert!(extracted.contains("export const greeting"));
        assert!(!extracted.contains("<template>"));
    }

    #[test]
    fn test_svelte_template_function_calls() {
        let template = r#"
            <div>
                <span>{badgeText(account)}</span>
                <p>{formatDate(date, 'short')}</p>
            </div>
        "#;

        let usages = parse_svelte_template_usages(template);
        assert!(usages.contains(&"badgeText".to_string()));
        assert!(usages.contains(&"formatDate".to_string()));
    }

    #[test]
    fn test_svelte_template_event_handlers() {
        let template = r#"
            <button on:click={handleClick}>Click me</button>
            <input on:input={onInputChange} />
            <form on:submit={submitForm}>...</form>
        "#;

        let usages = parse_svelte_template_usages(template);
        assert!(usages.contains(&"handleClick".to_string()));
        assert!(usages.contains(&"onInputChange".to_string()));
        assert!(usages.contains(&"submitForm".to_string()));
    }

    #[test]
    fn test_svelte_template_bind_directives() {
        let template = r#"
            <input bind:value={inputValue} />
            <select bind:value={selectedOption}>...</select>
        "#;

        let usages = parse_svelte_template_usages(template);
        assert!(usages.contains(&"inputValue".to_string()));
        assert!(usages.contains(&"selectedOption".to_string()));
    }

    #[test]
    fn test_svelte_template_use_directives() {
        let template = r#"
            <div use:clickOutside use:tooltip={tooltipParams}>
                Content
            </div>
        "#;

        let usages = parse_svelte_template_usages(template);
        assert!(usages.contains(&"clickOutside".to_string()));
        assert!(usages.contains(&"tooltip".to_string()));
    }

    #[test]
    fn test_svelte_template_transitions() {
        let template = r#"
            <div transition:fade in:fly out:slide animate:flip>
                Animated content
            </div>
        "#;

        let usages = parse_svelte_template_usages(template);
        assert!(usages.contains(&"fade".to_string()));
        assert!(usages.contains(&"fly".to_string()));
        assert!(usages.contains(&"slide".to_string()));
        assert!(usages.contains(&"flip".to_string()));
    }

    #[test]
    fn test_svelte_template_components() {
        let template = r#"
            <MyComponent prop={value} />
            <AnotherWidget />
            <div><NestedComponent /></div>
        "#;

        let usages = parse_svelte_template_usages(template);
        assert!(usages.contains(&"MyComponent".to_string()));
        assert!(usages.contains(&"AnotherWidget".to_string()));
        assert!(usages.contains(&"NestedComponent".to_string()));
    }

    #[test]
    fn test_svelte_template_control_flow_with_functions() {
        let template = r#"
            {#if hasConflicts()}
                <Warning />
            {/if}
            {#each getItems() as item}
                <Item {item} />
            {/each}
            {:else if checkCondition()}
                <Fallback />
            {/if}
        "#;

        let usages = parse_svelte_template_usages(template);
        assert!(usages.contains(&"hasConflicts".to_string()));
        assert!(usages.contains(&"getItems".to_string()));
        assert!(usages.contains(&"checkCondition".to_string()));
    }

    #[test]
    fn test_svelte_template_prop_values() {
        let template = r#"
            <Component value={myValue} handler={myHandler} />
        "#;

        let usages = parse_svelte_template_usages(template);
        assert!(usages.contains(&"myValue".to_string()));
        assert!(usages.contains(&"myHandler".to_string()));
    }

    #[test]
    fn test_svelte_file_full_analysis() {
        let content = r#"
<script lang="ts">
    import type { Account } from './types';

    export function badgeText(account: Account): string {
        return account.name;
    }

    export let account: Account;
</script>

<div class="badge">
    <span>{badgeText(account)}</span>
</div>

<style>
    .badge { color: blue; }
</style>
        "#;

        let analysis = analyze_js_file_ast(
            content,
            Path::new("src/GitHubAccountBadge.svelte"),
            Path::new("src"),
            None,
            None,
            "GitHubAccountBadge.svelte".to_string(),
            &CommandDetectionConfig::default(),
        );

        assert!(
            analysis.local_uses.contains(&"badgeText".to_string()),
            "badgeText should be in local_uses, found: {:?}",
            analysis.local_uses
        );

        assert!(
            analysis.local_uses.contains(&"account".to_string()),
            "account should be in local_uses, found: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn test_svelte_template_extraction() {
        let content = r#"
<script>
    let count = 0;
</script>

<button on:click={() => count++}>
    {count}
</button>

<style>
    button { color: red; }
</style>
        "#;

        let template = extract_svelte_template(content);
        assert!(!template.contains("let count = 0"));
        assert!(!template.contains("button { color: red; }"));
        assert!(template.contains("on:click"));
        assert!(template.contains("{count}"));
    }

    #[test]
    fn test_svelte_builtins_not_detected() {
        let template = r#"
            {#if condition}
                {#each items as item}
                    {#await promise then value}
                        {console.log(value)}
                    {:catch error}
                        {error}
                    {/await}
                {/each}
            {/if}
        "#;

        let usages = parse_svelte_template_usages(template);
        assert!(!usages.contains(&"if".to_string()));
        assert!(!usages.contains(&"each".to_string()));
        assert!(!usages.contains(&"await".to_string()));
        assert!(!usages.contains(&"then".to_string()));
        assert!(!usages.contains(&"catch".to_string()));
        assert!(!usages.contains(&"console".to_string()));
    }

    // ========== SVELTE ARROW FUNCTION TESTS ==========

    #[test]
    fn test_svelte_arrow_function_event_handlers() {
        let template = r#"
            <button on:click={() => save()}>Save</button>
            <button on:click={(e) => handleClick(e)}>Click</button>
            <input on:input={() => validate(value)}>
        "#;

        let usages = parse_svelte_template_usages(template);
        assert!(
            usages.contains(&"save".to_string()),
            "Should detect 'save' from arrow function, found: {:?}",
            usages
        );
        assert!(
            usages.contains(&"handleClick".to_string()),
            "Should detect 'handleClick' from arrow function with param, found: {:?}",
            usages
        );
        assert!(
            usages.contains(&"validate".to_string()),
            "Should detect 'validate' from arrow function, found: {:?}",
            usages
        );
    }

    #[test]
    fn test_svelte_mixed_event_handlers() {
        let template = r#"
            <button on:click={directHandler}>Direct</button>
            <button on:click={() => arrowHandler()}>Arrow</button>
            <button on:click={(event) => complexHandler(event, data)}>Complex</button>
        "#;

        let usages = parse_svelte_template_usages(template);
        assert!(usages.contains(&"directHandler".to_string()));
        assert!(usages.contains(&"arrowHandler".to_string()));
        assert!(usages.contains(&"complexHandler".to_string()));
    }

    // ========== VUE TEMPLATE TESTS ==========

    #[test]
    fn test_vue_template_function_calls() {
        let template = r#"
            <div>
                <span>{{ formatDate(date) }}</span>
                <p>{{ computeTotal(items, tax) }}</p>
            </div>
        "#;

        let usages = parse_vue_template_usages(template);
        assert!(usages.contains(&"formatDate".to_string()));
        assert!(usages.contains(&"computeTotal".to_string()));
    }

    #[test]
    fn test_vue_template_event_handlers() {
        let template = r#"
            <button @click="handleClick">Click</button>
            <input @input="onInputChange" />
            <form v-on:submit="submitForm">...</form>
        "#;

        let usages = parse_vue_template_usages(template);
        assert!(usages.contains(&"handleClick".to_string()));
        assert!(usages.contains(&"onInputChange".to_string()));
        assert!(usages.contains(&"submitForm".to_string()));
    }

    #[test]
    fn test_vue_template_prop_bindings() {
        let template = r#"
            <Component :value="computedValue" :data="myData" />
            <div v-bind:class="dynamicClass">Content</div>
        "#;

        let usages = parse_vue_template_usages(template);
        assert!(usages.contains(&"computedValue".to_string()));
        assert!(usages.contains(&"myData".to_string()));
        assert!(usages.contains(&"dynamicClass".to_string()));
    }

    #[test]
    fn test_vue_template_v_model() {
        let template = r#"
            <input v-model="username" />
            <select v-model="selectedOption">...</select>
        "#;

        let usages = parse_vue_template_usages(template);
        assert!(usages.contains(&"username".to_string()));
        assert!(usages.contains(&"selectedOption".to_string()));
    }

    #[test]
    fn test_vue_template_components() {
        let template = r#"
            <MyComponent :prop="value" />
            <AnotherWidget />
            <div><NestedComponent /></div>
        "#;

        let usages = parse_vue_template_usages(template);
        assert!(usages.contains(&"MyComponent".to_string()));
        assert!(usages.contains(&"AnotherWidget".to_string()));
        assert!(usages.contains(&"NestedComponent".to_string()));
    }

    #[test]
    fn test_vue_file_full_analysis() {
        let content = r#"
<script setup lang="ts">
    import type { Product } from './types';

    export function formatPrice(price: number): string {
        return `$${price.toFixed(2)}`;
    }

    export const product: Product = { name: 'Widget', price: 29.99 };
</script>

<template>
    <div class="product">
        <h3>{{ product.name }}</h3>
        <p>{{ formatPrice(product.price) }}</p>
    </div>
</template>

<style scoped>
    .product { border: 1px solid #ccc; }
</style>
        "#;

        let analysis = analyze_js_file_ast(
            content,
            Path::new("src/ProductCard.vue"),
            Path::new("src"),
            None,
            None,
            "ProductCard.vue".to_string(),
            &CommandDetectionConfig::default(),
        );

        assert!(
            analysis.local_uses.contains(&"formatPrice".to_string()),
            "formatPrice should be in local_uses, found: {:?}",
            analysis.local_uses
        );

        assert!(
            analysis.local_uses.contains(&"product".to_string()),
            "product should be in local_uses, found: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn test_vue_template_extraction() {
        let content = r#"
<script>
    const count = 0;
</script>

<template>
    <button @click="increment">
        {{ count }}
    </button>
</template>

<style scoped>
    button { color: red; }
</style>
        "#;

        let template = extract_vue_template(content);
        assert!(!template.contains("const count = 0"));
        assert!(!template.contains("button { color: red; }"));
        assert!(template.contains("@click"));
        assert!(template.contains("{{ count }}"));
    }

    #[test]
    fn test_vue_builtins_not_detected() {
        let template = r#"
            <div v-if="condition">
                <component :is="dynamicComponent" />
                <transition name="fade">
                    <keep-alive>
                        <component />
                    </keep-alive>
                </transition>
            </div>
        "#;

        let usages = parse_vue_template_usages(template);
        assert!(!usages.contains(&"if".to_string()));
        assert!(!usages.contains(&"component".to_string()));
        assert!(!usages.contains(&"transition".to_string()));
        assert!(!usages.contains(&"console".to_string()));
    }
}
