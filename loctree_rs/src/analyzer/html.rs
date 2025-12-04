use std::fs;
use std::io;
use std::path::Path;

use super::ReportSection;
use super::assets::{
    COSE_BASE_JS, CYTOSCAPE_COSE_BILKENT_JS, CYTOSCAPE_DAGRE_JS, CYTOSCAPE_JS, DAGRE_JS,
    LAYOUT_BASE_JS,
};

/// Render HTML report using Leptos SSR
pub(crate) fn render_html_report(path: &Path, sections: &[ReportSection]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        write_js_assets(dir)?;
    }

    // Convert loctree types to report-leptos types via JSON serialization
    // JSON bridge enables clean type separation between the analyzer and renderer
    let json = serde_json::to_string(sections).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize sections: {}", e),
        )
    })?;

    let leptos_sections: Vec<report_leptos::types::ReportSection> = serde_json::from_str(&json)
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to deserialize to Leptos types: {}", e),
            )
        })?;

    // Configure JS asset paths (relative to output file)
    // These match the files written by write_js_assets below
    let js_assets = report_leptos::JsAssets {
        cytoscape_path: "loctree-cytoscape.min.js".into(),
        dagre_path: "loctree-dagre.min.js".into(),
        cytoscape_dagre_path: "loctree-cytoscape-dagre.js".into(),
        layout_base_path: "loctree-layout-base.js".into(),
        cose_base_path: "loctree-cose-base.js".into(),
        cytoscape_cose_bilkent_path: "loctree-cytoscape-cose-bilkent.js".into(),
        ..Default::default()
    };

    let html = report_leptos::render_report(&leptos_sections, &js_assets);
    fs::write(path, html)
}

/// Write JS assets to output directory
fn write_js_assets(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    // Core Cytoscape library
    let js_path = dir.join("loctree-cytoscape.min.js");
    if !js_path.exists() {
        fs::write(&js_path, CYTOSCAPE_JS)?;
    }
    // Dagre layout library (dependency for cytoscape-dagre)
    let dagre_path = dir.join("loctree-dagre.min.js");
    if !dagre_path.exists() {
        fs::write(&dagre_path, DAGRE_JS)?;
    }
    // Cytoscape-dagre extension (hierarchical layout)
    let cy_dagre_path = dir.join("loctree-cytoscape-dagre.js");
    if !cy_dagre_path.exists() {
        fs::write(&cy_dagre_path, CYTOSCAPE_DAGRE_JS)?;
    }
    // layout-base (dependency for cose-base)
    let layout_base_path = dir.join("loctree-layout-base.js");
    if !layout_base_path.exists() {
        fs::write(&layout_base_path, LAYOUT_BASE_JS)?;
    }
    // cose-base (dependency for cytoscape-cose-bilkent)
    let cose_base_path = dir.join("loctree-cose-base.js");
    if !cose_base_path.exists() {
        fs::write(&cose_base_path, COSE_BASE_JS)?;
    }
    // Cytoscape-cose-bilkent extension (improved force-directed layout)
    let cy_cose_bilkent_path = dir.join("loctree-cytoscape-cose-bilkent.js");
    if !cy_cose_bilkent_path.exists() {
        fs::write(&cy_cose_bilkent_path, CYTOSCAPE_COSE_BILKENT_JS)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::render_html_report;
    use crate::analyzer::report::{AiInsight, RankedDup, ReportSection};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn renders_basic_report() {
        let tmp_dir = tempdir().expect("tmp dir");
        let out_path = tmp_dir.path().join("report.html");

        let dup = RankedDup {
            name: "Foo".into(),
            files: vec!["a.ts".into(), "b.ts".into()],
            locations: vec![],
            score: 2,
            prod_count: 2,
            dev_count: 0,
            canonical: "a.ts".into(),
            canonical_line: None,
            refactors: vec!["b.ts".into()],
        };

        let section = ReportSection {
            root: "test-root".into(),
            files_analyzed: 2,
            total_loc: 100,
            reexport_files_count: 1,
            dynamic_imports_count: 1,
            ranked_dups: vec![dup],
            cascades: vec![("a.ts".into(), "b.ts".into())],
            dynamic: vec![("dyn.ts".into(), vec!["./lazy".into()])],
            analyze_limit: 5,
            missing_handlers: Vec::new(),
            unregistered_handlers: Vec::new(),
            unused_handlers: Vec::new(),
            command_counts: (0, 0),
            command_bridges: Vec::new(),
            open_base: None,
            tree: None,
            graph: None,
            graph_warning: None,
            insights: vec![AiInsight {
                title: "Hint".into(),
                severity: "medium".into(),
                message: "Message".into(),
            }],
            git_branch: None,
            git_commit: None,
        };

        render_html_report(&out_path, &[section]).expect("render html");
        let html = fs::read_to_string(&out_path).expect("read html");

        // Verify key parts exist in the Leptos-rendered output
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("loctree report")); // Title in new Vista design

        // The output format might differ slightly from legacy, check for content
        assert!(html.contains("Hint"));
        assert!(html.contains("Foo"));
        assert!(html.contains("test-root"));
    }

    #[test]
    fn escapes_html_entities() {
        let tmp_dir = tempdir().expect("tmp dir");
        let out_path = tmp_dir.path().join("report.html");
        let malicious = r#"<script>alert('x')</script>"#;
        let section = ReportSection {
            root: malicious.into(),
            files_analyzed: 0,
            total_loc: 0,
            reexport_files_count: 0,
            dynamic_imports_count: 0,
            ranked_dups: Vec::new(),
            cascades: Vec::new(),
            dynamic: Vec::new(),
            analyze_limit: 1,
            missing_handlers: Vec::new(),
            unregistered_handlers: Vec::new(),
            unused_handlers: Vec::new(),
            command_counts: (0, 0),
            command_bridges: Vec::new(),
            open_base: None,
            tree: None,
            graph: None,
            graph_warning: None,
            insights: Vec::new(),
            git_branch: None,
            git_commit: None,
        };

        render_html_report(&out_path, &[section]).expect("render html");
        let html = fs::read_to_string(&out_path).expect("read html");

        // Security: raw script must not appear
        assert!(
            !html.contains(malicious),
            "XSS: raw script tag should be escaped"
        );

        // Leptos escapes content automatically
        // We check that both opening and closing tags are safely escaped
        assert!(html.contains("&lt;script&gt;") && html.contains("&lt;/script&gt;"));
    }
}
