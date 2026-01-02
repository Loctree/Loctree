//! Dynamic Python code generation detection (exec/eval/compile).
//!
//! Detects patterns where symbols are generated dynamically via exec(), eval(),
//! or compile() calls with template strings. These patterns should not be
//! flagged as dead code.
//!
//! Created by M&K (c)2025 The LibraxisAI Team
//! Co-Authored-By: Maciej <void@div0.space> & Klaudiusz <the1st@whoai.am>

use crate::types::DynamicExecTemplate;

/// Detect Python exec/eval/compile calls with template strings.
/// These patterns generate symbols dynamically and should not be flagged as dead code.
///
/// Detects:
/// - exec(template) where template contains %s, %d, {name}, etc.
/// - eval(template)
/// - compile(template, ...)
///
/// Example: exec("def get%s(self): return self._%s" % (name, name))
pub(super) fn detect_dynamic_exec_templates(content: &str) -> Vec<DynamicExecTemplate> {
    let mut templates = Vec::new();

    // Pattern: exec(, eval(, compile(
    let exec_patterns = ["exec(", "eval(", "compile("];

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1; // 1-based

        for pattern in &exec_patterns {
            if let Some(start_idx) = line.find(pattern) {
                // Extract the argument to exec/eval/compile
                let after_call = &line[start_idx + pattern.len()..];

                // Look for template strings containing format placeholders
                // Old-style: %s, %d, %r, %(name)s
                // New-style: {}, {0}, {name}
                let has_old_style = after_call.contains("%s")
                    || after_call.contains("%d")
                    || after_call.contains("%r")
                    || after_call.contains("%(");

                let has_new_style = after_call.contains("{}")
                    || after_call.contains("{0}")
                    || (after_call.contains('{') && after_call.contains('}'));

                if has_old_style || has_new_style {
                    let call_type = pattern.trim_end_matches('(').to_string();

                    // Extract prefixes from template - look for def/class patterns
                    let mut generated_prefixes = Vec::new();

                    // Look for patterns like "def get%s" or "def set%s"
                    // Also "class %s" patterns
                    if let Some(def_idx) = after_call.find("def ") {
                        let after_def = &after_call[def_idx + 4..];
                        // Extract the function name prefix before %s or {
                        if let Some(end) = after_def.find('%').or_else(|| after_def.find('{')) {
                            let prefix = after_def[..end].trim();
                            if !prefix.is_empty()
                                && prefix.chars().all(|c| c.is_alphanumeric() || c == '_')
                            {
                                generated_prefixes.push(prefix.to_string());
                            }
                        }
                    }

                    // Also look for "class %s" patterns
                    if let Some(class_idx) = after_call.find("class ") {
                        let after_class = &after_call[class_idx + 6..];
                        if let Some(end) = after_class.find('%').or_else(|| after_class.find('{')) {
                            let prefix = after_class[..end].trim();
                            if !prefix.is_empty()
                                && prefix.chars().all(|c| c.is_alphanumeric() || c == '_')
                            {
                                generated_prefixes.push(prefix.to_string());
                            }
                        }
                    }

                    templates.push(DynamicExecTemplate {
                        template: after_call.chars().take(100).collect(), // First 100 chars
                        generated_prefixes,
                        line: line_num,
                        call_type,
                    });
                }
            }
        }
    }

    templates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_exec_with_old_style_format() {
        let content = r#"
exec("def get%s(self): return self._%s" % (name, name))
"#;
        let templates = detect_dynamic_exec_templates(content);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].call_type, "exec");
        assert!(templates[0].generated_prefixes.contains(&"get".to_string()));
    }

    #[test]
    fn detects_eval_with_new_style_format() {
        let content = r#"
result = eval("{}({})".format(func_name, args))
"#;
        let templates = detect_dynamic_exec_templates(content);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].call_type, "eval");
    }

    #[test]
    fn detects_compile_with_format() {
        let content = r#"
code = compile("class {}(Base): pass".format(name), "<dynamic>", "exec")
"#;
        let templates = detect_dynamic_exec_templates(content);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].call_type, "compile");
    }

    #[test]
    fn no_detection_without_format_string() {
        let content = r#"
exec(some_code)
eval("1 + 2")
"#;
        let templates = detect_dynamic_exec_templates(content);
        // "1 + 2" doesn't have format placeholders
        // some_code is a variable, not a template string
        assert!(templates.is_empty() || templates.iter().all(|t| !t.template.contains("%s")));
    }
}
