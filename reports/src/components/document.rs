//! Root document component - the complete HTML page

use leptos::prelude::*;
use crate::styles::{REPORT_CSS, CSP};
use crate::types::ReportSection;
use crate::JsAssets;
use super::{AiSummaryPanel, ReportSectionView};

/// The complete HTML document for the report
#[component]
pub fn ReportDocument(
    sections: Vec<ReportSection>,
    js_assets: JsAssets,
) -> impl IntoView {
    view! {
        <html>
            <head>
                <meta charset="UTF-8" />
                <meta http-equiv="Content-Security-Policy" content=CSP />
                <title>"loctree import/export report"</title>
                <style>{REPORT_CSS}</style>
            </head>
            <body>
                <h1>"loctree import/export analysis"</h1>

                <AiSummaryPanel sections=sections.clone() />

                {sections.into_iter().map(|section| {
                    view! { <ReportSectionView section=section.clone() /> }
                }).collect::<Vec<_>>()}

                <GraphScripts js_assets=js_assets />
            </body>
        </html>
    }
}

/// JavaScript for graph initialization
#[component]
fn GraphScripts(js_assets: JsAssets) -> impl IntoView {
    // Only render script tags if paths are provided
    let has_assets = !js_assets.cytoscape_path.is_empty();

    view! {
        {has_assets.then(|| view! {
            <script src=js_assets.cytoscape_path.clone()></script>
            <script src=js_assets.dagre_path.clone()></script>
            <script src=js_assets.cytoscape_dagre_path.clone()></script>
            <script src=js_assets.cytoscape_cose_bilkent_path.clone()></script>
            <script>{include_str!("../graph_bootstrap.js")}</script>
            <script>{TAB_SCRIPT}</script>
        })}
    }
}

/// Tab switching script (vanilla JS)
const TAB_SCRIPT: &str = r#"
(() => {
  document.querySelectorAll(".tab-bar").forEach((bar) => {
    const scope = bar.dataset.tabScope || "";
    bar.querySelectorAll("button").forEach((btn) => {
      btn.addEventListener("click", () => {
        bar.querySelectorAll("button").forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");
        const target = btn.dataset.tab;
        document.querySelectorAll(`[data-tab-panel^="${scope}-"]`).forEach((panel) => {
          panel.classList.toggle("active", panel.dataset.tabPanel === target);
        });
      });
    });
  });
})();
"#;
