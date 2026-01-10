//! Filter functions for dead export detection

use globset::GlobSet;

use crate::types::{ExportSymbol, FileAnalysis};

use super::DeadFilterConfig;

pub(super) fn should_skip_dead_export_check(
    analysis: &FileAnalysis,
    config: &DeadFilterConfig,
    example_globs: Option<&GlobSet>,
) -> bool {
    let path = &analysis.path;
    let lower_path = path.to_ascii_lowercase();

    // Go exports are primarily public API; static import graph is insufficient (FP-heavy)
    // Skip dead-export detection for Go to avoid noise.
    if analysis.language == "go" {
        return true;
    }

    // JSX runtime files - exports consumed by TypeScript/Babel compiler, not by imports
    // Files matching: *jsx-runtime*, jsx-runtime.js, jsx-runtime/index.js, jsx-dev-runtime.js, etc.
    if (analysis.language == "ts" || analysis.language == "js")
        && (lower_path.contains("jsx-runtime")
            || lower_path.contains("jsx_runtime")
            || lower_path.contains("jsx-dev-runtime"))
    {
        return true;
    }

    // Test files and fixtures
    if analysis.is_test && !config.include_tests {
        return true;
    }

    // Flutter generated/plugin registrant files
    if path.ends_with("generated_plugin_registrant.dart")
        || path.contains("/generated_plugin_registrant.dart")
    {
        return true;
    }

    // Test-related directories
    const TEST_DIRS: &[&str] = &[
        "stories",
        "__tests__",
        "__mocks__",
        "__fixtures__",
        "/cypress/",
        "/e2e/",
        "/playwright/",
        "/test/",
        "/tests/",
        "/spec/",
    ];
    if TEST_DIRS.iter().any(|d| lower_path.contains(d)) && !config.include_tests {
        return true;
    }

    // Example/demo/fixture packages (library-mode noise)
    const EXAMPLE_DIRS: &[&str] = &[
        "/examples/",
        "/example/",
        "/samples/",
        "/sample/",
        "/demo/",
        "/demos/",
        "/playground/",
        "/showcase/",
    ];
    if EXAMPLE_DIRS.iter().any(|d| lower_path.contains(d)) {
        return true;
    }
    if config.library_mode {
        if let Some(globs) = example_globs
            && (globs.is_match(path) || globs.is_match(&lower_path))
        {
            return true;
        }
        const LIBRARY_NOISE_DIRS: &[&str] = &[
            "/kitchen-sink/",
            "/kitchensink/",
            "/sandbox/",
            "/sandboxes/",
            "/cookbook/",
            "/gallery/",
            "/examples-",
            "/examples_",
            "/docs/examples/",
            "/documentation/examples/",
        ];
        if LIBRARY_NOISE_DIRS.iter().any(|d| lower_path.contains(d))
            || lower_path.starts_with("examples/")
            || lower_path.starts_with("example/")
            || lower_path.starts_with("demo/")
            || lower_path.contains("/examples/")
        {
            return true;
        }
    }
    // Lowercase check for testfixtures (common in codemods)
    if lower_path.contains("testfixtures") {
        return true;
    }

    // TypeScript declaration files (.d.ts) - only contain type declarations
    if path.ends_with(".d.ts") {
        return true;
    }

    // Dart/Flutter generated artifacts
    if path.ends_with(".g.dart")
        || path.ends_with(".freezed.dart")
        || path.ends_with(".gr.dart")
        || path.ends_with(".pb.dart")
        || path.ends_with(".pbjson.dart")
        || path.ends_with(".pbenum.dart")
        || path.ends_with(".pbserver.dart")
        || path.ends_with(".config.dart")
    {
        return true;
    }

    // Config files loaded dynamically by build tools (Vite, Jest, Cypress, etc.)
    if path.contains(".config.") || path.ends_with(".config.ts") || path.ends_with(".config.js") {
        return true;
    }

    if !config.include_helpers {
        const HELPER_DIRS: &[&str] = &["/scripts/", "/script/", "/tools/", "/docs/"];
        if HELPER_DIRS.iter().any(|d| path.contains(d))
            || path.starts_with("scripts/")
            || path.starts_with("script/")
            || path.starts_with("tools/")
            || path.starts_with("docs/")
        {
            return true;
        }
    }

    // Framework routing/entry point conventions
    // SvelteKit: +page.ts, +layout.ts, +server.ts, +page.server.ts, hooks.*.ts
    // Next.js: page.tsx, layout.tsx, route.ts (in app/ directory)
    const FRAMEWORK_ENTRY_PATTERNS: &[&str] = &[
        "+page.",
        "+layout.",
        "+server.",
        "+error.",
        "/page.tsx",
        "/page.ts",
        "/layout.tsx",
        "/layout.ts",
        "/route.ts",
        "/route.tsx",
        "/error.tsx",
        "/loading.tsx",
        "/not-found.tsx",
        // SvelteKit hooks (auto-loaded by framework)
        "hooks.client.",
        "hooks.server.",
        "/hooks.",
    ];
    if FRAMEWORK_ENTRY_PATTERNS.iter().any(|p| path.contains(p)) {
        return true;
    }

    // Virtual/module-runtime entrypoints (framework-provided consumers)
    const RUNTIME_PATTERNS: &[&str] =
        &["/.svelte-kit/", "/runtime/", "/app/router/", "/app/routes/"];
    if (analysis.language == "ts" || analysis.language == "js")
        && RUNTIME_PATTERNS.iter().any(|p| path.contains(p))
    {
        return true;
    }

    // Library barrels and public API surfaces (avoid flagging public exports)
    if (path.ends_with("/index.ts")
        || path.ends_with("/index.tsx")
        || path.ends_with("/index.js")
        || path.ends_with("/index.mjs")
        || path.ends_with("/index.cjs")
        || path.ends_with("/mod.ts")
        || path.ends_with("/mod.js"))
        && (analysis.language == "ts" || analysis.language == "js")
        && (path.contains("/packages/") || path.contains("/libs/") || path.contains("/library/"))
    {
        return true;
    }

    false
}

/// Check if an export from a Svelte file is likely a component API method.
/// Svelte components expose methods via `export function` that are called via `bind:this`:
///   let modal: MyModal;
///   <MyModal bind:this={modal} />
///   modal.show();  // calling the exported function
///
/// These are NOT imported via ES imports, so they appear as "dead" in static analysis.
pub(super) fn is_jsx_runtime_export(export_name: &str, file_path: &str) -> bool {
    // JSX runtime export names defined by React JSX transform spec
    const JSX_RUNTIME_EXPORTS: &[&str] = &["jsx", "jsxs", "jsxDEV", "jsxsDEV", "Fragment"];

    if !JSX_RUNTIME_EXPORTS.contains(&export_name) {
        return false;
    }

    // Check if file is likely a JSX runtime
    // Patterns: jsx-runtime, jsx_runtime, jsx-dev-runtime (React dev mode)
    let lower_path = file_path.to_ascii_lowercase();
    lower_path.contains("jsx-runtime")
        || lower_path.contains("jsx_runtime")
        || lower_path.contains("jsx-dev-runtime")
}

/// Check if an export is likely a Flow type-only export.
/// Flow files (annotated with @flow) export types that are used via Flow's type system,
/// not via regular import statements. These exports don't appear in static import analysis
pub(super) fn is_flow_type_export(export_symbol: &ExportSymbol, analysis: &FileAnalysis) -> bool {
    if !analysis.is_flow_file {
        return false;
    }

    // Flow type exports: type, interface, opaque type
    // These are type-only and won't appear in runtime imports
    matches!(export_symbol.kind.as_str(), "type" | "interface" | "opaque")
}

/// Check if an export is used in a WeakMap/WeakSet registry pattern.
/// These are common in React and other libraries for storing metadata about objects
/// without causing memory leaks. Exports stored in WeakMap/WeakSet are used dynamically.
pub(super) fn is_weakmap_registry_export(
    _export_symbol: &ExportSymbol,
    analysis: &FileAnalysis,
) -> bool {
    // If a file contains WeakMap/WeakSet usage (detected by AST visitor),
    // conservatively assume all exports might be stored dynamically in the registry.
    // This reduces false positives in React DevTools and similar code where exports
    // are stored in WeakMaps for dynamic lookup.
    analysis.has_weak_collections
}
pub(super) fn is_python_test_export(analysis: &FileAnalysis, exp: &ExportSymbol) -> bool {
    if !analysis.path.ends_with(".py") {
        return false;
    }
    if exp.kind == "class" && exp.name.starts_with("Test") {
        return true;
    }
    if exp.kind == "def" && exp.name.starts_with("test_") {
        return true;
    }
    false
}

pub(super) fn is_python_test_path(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_lowercase();
    lower.contains("/tests/")
        || lower.contains("/test/")
        || lower.ends_with("_test.py")
        || lower.ends_with("_tests.py")
        || lower
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("test_"))
}

/// Check if a TypeScript/JavaScript file contains ambient declaration patterns.
/// Ambient declarations (declare global, declare module, declare namespace) are
/// consumed by the TypeScript compiler, not by imports. Exports inside these
/// blocks are NOT dead code - they extend global types or module augmentations.
///
/// Returns true if the file contains ambient declaration patterns.
pub(super) fn has_ambient_declarations(analysis: &FileAnalysis) -> bool {
    // Only check TypeScript files
    if analysis.language != "ts" {
        return false;
    }

    // If we have raw file content cached, check for ambient patterns
    // Note: This is a heuristic based on file path patterns since we don't
    // have direct access to file content here. A more precise approach would
    // be to track this during AST parsing.

    let path = &analysis.path;
    let lower_path = path.to_ascii_lowercase();

    // Common patterns for files that contain ambient declarations:
    // 1. Files ending with .d.ts are type declaration files (already handled separately)
    // 2. Files in jsx-runtime directories often contain declare global for JSX namespace
    // 3. Files named global*.ts or globals.ts often contain declare global
    // 4. Files with "types" or "typings" in the path often contain ambient declarations

    // Check for jsx-runtime patterns (Vue, React, etc.)
    if lower_path.contains("jsx-runtime") || lower_path.contains("jsx_runtime") {
        return true;
    }

    // Check for common global type definition file names
    let file_name = path
        .rsplit('/')
        .next()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if file_name.starts_with("global")
        || file_name == "globals.ts"
        || file_name == "env.d.ts"
        || file_name == "global.d.ts"
        || file_name == "index.d.ts"
        || file_name == "shim.d.ts"
        || file_name == "shims.d.ts"
        || file_name.contains("augment")
    {
        return true;
    }

    // Check for paths that typically contain type augmentations
    if lower_path.contains("/types/") || lower_path.contains("/typings/") {
        return true;
    }

    false
}

/// Check if a specific export is likely an ambient declaration export.
/// This checks if the export is a type/interface that's commonly found in
/// ambient declaration contexts (like JSX namespace interfaces).
pub(super) fn is_ambient_export(export_symbol: &ExportSymbol, analysis: &FileAnalysis) -> bool {
    // Only applies to TypeScript
    if analysis.language != "ts" {
        return false;
    }

    // Check if file has ambient declaration patterns
    if !has_ambient_declarations(analysis) {
        return false;
    }

    // In ambient declaration contexts, type exports are compiler-consumed
    matches!(
        export_symbol.kind.as_str(),
        "interface" | "type" | "namespace"
    )
}

/// Check if an export name matches a dynamically generated pattern from exec/eval/compile.
/// These are template placeholders like "get%s", "set%s" that generate symbols at runtime.
///
/// This detects patterns from CPython and similar projects that use exec() with template
/// strings to generate accessor methods, classes, etc.
///
/// Example pattern:
/// ```python
/// exec("def get%s(self): return self._%s" % (name, name))
/// ```
/// When `name` could be "foo", "bar", etc., the exports "getfoo", "getbar" are NOT dead code.
pub(super) fn is_dynamic_exec_template(export_name: &str, analysis: &FileAnalysis) -> bool {
    // Only applies to Python files
    if !analysis.path.ends_with(".py") {
        return false;
    }

    // Check if file has any dynamic exec templates
    if analysis.dynamic_exec_templates.is_empty() {
        return false;
    }

    // Check if the export name could match any template pattern
    for template in &analysis.dynamic_exec_templates {
        // Check against generated prefixes (e.g., "get", "set")
        for prefix in &template.generated_prefixes {
            // Export name starts with the prefix (e.g., "getfoo" starts with "get")
            if export_name.starts_with(prefix) {
                return true;
            }
        }

        // Also check if the template pattern itself matches
        // Template contains patterns like "get%s" or "set{name}"
        // The export could be "get_something" or "set_something"
        let template_lower = template.template.to_lowercase();

        // Common patterns: "def get%s", "def set%s", "class %s"
        if template_lower.contains("def ") {
            // Extract the function name pattern
            if let Some(def_pos) = template_lower.find("def ") {
                let after_def = &template_lower[def_pos + 4..];
                // Find the pattern before the format specifier
                if let Some(format_pos) = after_def.find('%').or(after_def.find('{')) {
                    let pattern_prefix = after_def[..format_pos].trim();
                    if !pattern_prefix.is_empty()
                        && export_name.to_lowercase().starts_with(pattern_prefix)
                    {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Check if a file uses sys.modules monkey-patching.
/// If a file injects itself into sys.modules (e.g., `sys.modules['compat'] = wrapper`),
/// ALL exports from that file are accessible at runtime via the injected module name.
/// Therefore, none of its exports should be flagged as dead code.
///
/// Example:
/// ```python
/// # compat.py
/// import sys
/// class CompatWrapper:
///     # ... wrapper logic
/// sys.modules['compat'] = CompatWrapper(sys.modules[__name__])
/// ```
///
/// Even if `CompatWrapper` has no direct imports, it's accessible via `import compat`.
pub(super) fn has_sys_modules_injection(analysis: &FileAnalysis) -> bool {
    // Only applies to Python files
    if !analysis.path.ends_with(".py") {
        return false;
    }

    // If file has any sys.modules injections, all its exports are "alive"
    !analysis.sys_modules_injections.is_empty()
}
