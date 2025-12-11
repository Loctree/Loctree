//! Report with interactive dependency graph.
//!
//! Run with: `cargo run --example with_graph`

use report_leptos::{
    render_report,
    types::{AiInsight, GraphComponent, GraphData, GraphNode, RankedDup, ReportSection},
    JsAssets,
};

fn main() {
    // Create graph nodes
    let nodes = vec![
        GraphNode {
            id: "src/main.ts".into(),
            label: "main.ts".into(),
            loc: 150,
            x: 0.5,
            y: 0.2,
            component: 0,
            degree: 3,
            detached: false,
        },
        GraphNode {
            id: "src/utils/index.ts".into(),
            label: "utils/index.ts".into(),
            loc: 80,
            x: 0.3,
            y: 0.5,
            component: 0,
            degree: 4,
            detached: false,
        },
        GraphNode {
            id: "src/components/App.tsx".into(),
            label: "App.tsx".into(),
            loc: 200,
            x: 0.7,
            y: 0.5,
            component: 0,
            degree: 2,
            detached: false,
        },
        GraphNode {
            id: "src/legacy/old.ts".into(),
            label: "old.ts".into(),
            loc: 50,
            x: 0.9,
            y: 0.9,
            component: 1,
            degree: 0,
            detached: true,
        },
    ];

    // Create edges
    let edges = vec![
        (
            "src/main.ts".into(),
            "src/utils/index.ts".into(),
            "import".into(),
        ),
        (
            "src/main.ts".into(),
            "src/components/App.tsx".into(),
            "import".into(),
        ),
        (
            "src/components/App.tsx".into(),
            "src/utils/index.ts".into(),
            "import".into(),
        ),
    ];

    // Create components
    let components = vec![
        GraphComponent {
            id: 0,
            size: 3,
            edge_count: 3,
            nodes: vec![
                "src/main.ts".into(),
                "src/utils/index.ts".into(),
                "src/components/App.tsx".into(),
            ],
            isolated_count: 0,
            sample: "main.ts".into(),
            loc_sum: 430,
            detached: false,
            tauri_frontend: 3,
            tauri_backend: 0,
        },
        GraphComponent {
            id: 1,
            size: 1,
            edge_count: 0,
            nodes: vec!["src/legacy/old.ts".into()],
            isolated_count: 1,
            sample: "old.ts".into(),
            loc_sum: 50,
            detached: true,
            tauri_frontend: 1,
            tauri_backend: 0,
        },
    ];

    // Create graph data
    let graph = GraphData {
        nodes,
        edges,
        components,
        main_component_id: 0,
    };

    // Create insights
    let insights = vec![
        AiInsight {
            title: "Detached Component Found".into(),
            severity: "medium".into(),
            message: "src/legacy/old.ts is not imported anywhere. Consider removing or integrating it.".into(),
        },
        AiInsight {
            title: "Central Module Detected".into(),
            severity: "low".into(),
            message: "src/utils/index.ts has high connectivity (4 edges). Consider splitting if it grows.".into(),
        },
    ];

    // Create duplicate export
    let ranked_dups = vec![RankedDup {
        name: "formatDate".into(),
        files: vec!["src/utils/date.ts".into(), "src/helpers/format.ts".into()],
        score: 5,
        prod_count: 2,
        dev_count: 0,
        canonical: "src/utils/date.ts".into(),
        refactors: vec!["src/helpers/format.ts".into()],
    }];

    // Create report section
    let section = ReportSection {
        root: "my-app/src".into(),
        files_analyzed: 4,
        ranked_dups,
        cascades: vec![("src/a.ts".into(), "src/b.ts".into())],
        dynamic: vec![("src/main.ts".into(), vec!["./lazy-module".into()])],
        analyze_limit: 100,
        graph: Some(graph),
        insights,
        ..Default::default()
    };

    // Configure JS assets from CDN (with Cytoscape fallback)
    let js_assets = JsAssets {
        cytoscape_path: "https://unpkg.com/cytoscape@3/dist/cytoscape.min.js".into(),
        dagre_path: "https://unpkg.com/dagre@0.8/dist/dagre.min.js".into(),
        cytoscape_dagre_path: "https://unpkg.com/cytoscape-dagre@2/cytoscape-dagre.js".into(),
        layout_base_path: "https://unpkg.com/layout-base@2/layout-base.js".into(),
        cose_base_path: "https://unpkg.com/cose-base@2/cose-base.js".into(),
        cytoscape_cose_bilkent_path:
            "https://unpkg.com/cytoscape-cose-bilkent@4/cytoscape-cose-bilkent.js".into(),
        // WASM assets (None = use Cytoscape fallback)
        wasm_base64: None,
        wasm_js_glue: None,
    };

    // Render
    let html = render_report(&[section], &js_assets, false);

    // Write
    let output_path = "graph_report.html";
    std::fs::write(output_path, &html).expect("Failed to write report");

    println!("Report with graph written to: {}", output_path);
    println!("HTML size: {} bytes", html.len());
    println!(
        "\nOpen {} in a browser to see the interactive graph!",
        output_path
    );
}
