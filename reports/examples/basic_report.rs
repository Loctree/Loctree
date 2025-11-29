//! Basic report generation example.
//!
//! Run with: `cargo run --example basic_report`

use report_leptos::{render_report, types::ReportSection, JsAssets};

fn main() {
    // Create a simple report section
    let section = ReportSection {
        root: "my-project/src".into(),
        files_analyzed: 42,
        analyze_limit: 100,
        ..Default::default()
    };

    // Use default (empty) JS assets - graph won't be interactive
    let js_assets = JsAssets::default();

    // Render to HTML
    let html = render_report(&[section], &js_assets);

    // Write to file
    let output_path = "basic_report.html";
    std::fs::write(output_path, &html).expect("Failed to write report");

    println!("Report written to: {}", output_path);
    println!("HTML size: {} bytes", html.len());
}
