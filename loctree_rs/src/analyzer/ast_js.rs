use std::collections::HashSet;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::types::{
    CommandRef, EventRef, ExportSymbol, FileAnalysis, ImportEntry, ImportKind, ImportSymbol,
    ReexportEntry, ReexportKind,
};

use super::resolvers::{TsPathResolver, resolve_reexport_target};

/// Analyze JS/TS file using OXC AST parser
#[allow(dead_code)]
pub(crate) fn analyze_js_file_ast(
    content: &str,
    path: &Path,
    root: &Path,
    extensions: Option<&HashSet<String>>,
    ts_resolver: Option<&TsPathResolver>,
    relative: String,
) -> FileAnalysis {
    let allocator = Allocator::default();

    // Determine source type from file extension
    let source_type = SourceType::from_path(path)
        .unwrap_or_default()
        .with_typescript(true)
        .with_jsx(true);

    let ret = Parser::new(&allocator, content, source_type).parse();

    let mut visitor = JsVisitor {
        analysis: FileAnalysis::new(relative),
        path,
        root,
        extensions,
        ts_resolver,
        source_text: content,
    };

    visitor.visit_program(&ret.program);

    visitor.analysis
}

struct JsVisitor<'a> {
    analysis: FileAnalysis,
    path: &'a Path,
    root: &'a Path,
    extensions: Option<&'a HashSet<String>>,
    ts_resolver: Option<&'a TsPathResolver>,
    source_text: &'a str,
}

impl<'a> JsVisitor<'a> {
    #[allow(dead_code)]
    fn resolve_path(&self, source: &str) -> Option<String> {
        resolve_reexport_target(self.path, self.root, source, self.extensions)
            .or_else(|| {
                self.ts_resolver
                    .and_then(|r| r.resolve(source, self.extensions))
            })
            .or_else(|| {
                super::resolvers::resolve_js_relative(self.path, self.root, source, self.extensions)
            })
    }

    #[allow(dead_code)]
    fn get_line(&self, span: oxc_span::Span) -> usize {
        self.source_text[..span.start as usize]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            + 1
    }

    /// Extract basic type representation from TSType
    fn type_to_string(&self, ty: &TSType<'a>) -> String {
        match ty {
            TSType::TSTypeReference(r) => self.type_name_to_string(&r.type_name),
            _ => "Type".to_string(), // Fallback for complex types
        }
    }

    fn type_name_to_string(&self, name: &TSTypeName<'a>) -> String {
        match name {
            TSTypeName::IdentifierReference(id) => id.name.to_string(),
            TSTypeName::QualifiedName(q) => {
                format!("{}.{}", self.type_name_to_string(&q.left), q.right.name)
            }
        }
    }
}

impl<'a> Visit<'a> for JsVisitor<'a> {
    // --- IMPORTS ---

    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        let source = decl.source.value.to_string();
        let mut entry = ImportEntry::new(source.clone(), ImportKind::Static);
        entry.resolved_path = self.resolve_path(&source);
        entry.is_bare = !source.starts_with('.') && !source.starts_with('/');

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
                            }
                        }
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
        let name = match &decl.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                f.id.as_ref()
                    .map(|i| i.name.to_string())
                    .unwrap_or("default".to_string())
            }
            ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                c.id.as_ref()
                    .map(|i| i.name.to_string())
                    .unwrap_or("default".to_string())
            }
            ExportDefaultDeclarationKind::TSInterfaceDeclaration(i) => i.id.name.to_string(),
            _ => "default".to_string(),
        };
        self.analysis
            .exports
            .push(ExportSymbol::new(name, "default", "default", Some(line)));
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
            // Track as dynamic import
            if !self.analysis.dynamic_imports.contains(&source) {
                self.analysis.dynamic_imports.push(source.clone());
            }
        }

        // Continue visiting arguments (if any)
        self.visit_expression(&expr.source);
        for arg in &expr.arguments {
            self.visit_expression(arg);
        }
    }

    // --- CALL EXPRESSIONS (invoke, etc) ---

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        // Continue visiting children
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
            // Commands
            // Match legacy behavior: any function containing "invoke" or "Command"
            let name_lower = name.to_lowercase();
            let is_potential_command = name_lower.contains("invoke") || name.contains("Command");

            // Known DOM APIs to exclude (from legacy js.rs)
            const DOM_EXCLUSIONS: &[&str] = &[
                "execCommand",
                "queryCommandState",
                "queryCommandEnabled",
                "queryCommandSupported",
                "queryCommandValue",
            ];

            if is_potential_command
                && !DOM_EXCLUSIONS.iter().any(|ex| name.contains(ex))
                && !DOM_EXCLUSIONS.contains(&name.as_str())
                && let Some(arg) = call.arguments.first()
            {
                let payload = match arg {
                    Argument::StringLiteral(s) => Some(s.value.to_string()),
                    _ => None,
                };

                let generic = call
                    .type_parameters
                    .as_ref()
                    .and_then(|params| params.params.first().map(|p| self.type_to_string(p)));

                let line = self.get_line(call.span);

                self.analysis.command_calls.push(CommandRef {
                    name: name.clone(),
                    exposed_name: None,
                    line,
                    generic_type: generic,
                    payload: payload.clone(),
                });

                // Fix collapsible_if
                if let Some(cmd_name) = payload
                    && let Some(last) = self.analysis.command_calls.last_mut()
                {
                    last.name = cmd_name;
                    last.payload = None;
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
        assert!(exports.contains(&"MyClass"));
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
}
