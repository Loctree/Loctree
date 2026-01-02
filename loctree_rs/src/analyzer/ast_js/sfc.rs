//! Single File Component (SFC) script and template extraction.
//!
//! This module handles extraction of script and template content from
//! Svelte (.svelte) and Vue (.vue) Single File Components.
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use regex::Regex;

/// Extract script content from a Svelte file.
///
/// Handles both `<script>` and `<script lang="ts">` variants.
pub(super) fn extract_svelte_script(content: &str) -> String {
    extract_sfc_script(content)
}

/// Extract script content from a Vue Single File Component (SFC).
///
/// Handles `<script>`, `<script setup>`, `<script lang="ts">` variants.
pub(super) fn extract_vue_script(content: &str) -> String {
    extract_sfc_script(content)
}

/// Common SFC script extraction used by both Svelte and Vue.
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

/// Extract template content from a Svelte file (everything outside <script> and <style>).
pub(super) fn extract_svelte_template(content: &str) -> String {
    let mut result = content.to_string();
    if let Ok(script_re) = Regex::new(r#"<script[^>]*>[\s\S]*?</script>"#) {
        result = script_re.replace_all(&result, "").to_string();
    }
    if let Ok(style_re) = Regex::new(r#"<style[^>]*>[\s\S]*?</style>"#) {
        result = style_re.replace_all(&result, "").to_string();
    }
    result
}

/// Extract template content from a Vue file (everything inside <template> tags).
pub(super) fn extract_vue_template(content: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
