//! Report section component - wrapper for each analyzed root
//!
//! Implements the "Section View" layout with a sticky header and scrollable content.

use super::{
    AiInsightsPanel, AnalysisSummary, CascadesList, Coverage, Crowds, Cycles, DeadCode,
    DuplicateExportsTable, DynamicImportsTable, GraphContainer, Pipelines, QuickCommandsPanel,
    TabContent, TauriCommandCoverage, TreeView, Twins,
};
use crate::types::ReportSection;
use leptos::prelude::*;

/// Shorten a path for display: "vista/src" instead of "/Users/maciej/hosted/vista/src"
fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    // Take last 2-3 meaningful parts
    if parts.len() <= 3 {
        path.to_string()
    } else {
        parts
            .iter()
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// A complete report section for one analyzed root
#[component]
pub fn ReportSectionView(section: ReportSection, active: bool, view_id: String) -> impl IntoView {
    let root_id = section
        .root
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let root_id_value = root_id.clone();
    let root_id_overview = root_id_value.clone();
    let root_id_dups = root_id_value.clone();
    let root_id_dynamic = root_id_value.clone();
    let root_id_commands = root_id_value.clone();
    let root_id_pipelines = root_id_value.clone();
    let root_id_crowds = root_id_value.clone();
    let root_id_cycles = root_id_value.clone();
    let root_id_dead = root_id_value.clone();
    let root_id_twins = root_id_value.clone();
    let root_id_coverage = root_id_value.clone();
    let root_id_graph = root_id_value.clone();
    let root_id_graph_tab = root_id_graph.clone();
    let root_id_graph_container = root_id_graph.clone();
    let root_id_tree = root_id_value.clone();

    let section_clone = section.clone();
    let view_class = if active {
        "section-view active"
    } else {
        "section-view"
    };

    // Stats for header - extract before view! macro to avoid borrow issues
    let file_count = section.files_analyzed;
    let total_loc = section.total_loc;
    let duplicate_exports_count = section.ranked_dups.len();
    let reexport_files_count = section.reexport_files_count;
    let dynamic_imports_count = section.dynamic_imports_count;

    // QuickCommands panel flags (computed before view! to avoid move issues)
    let has_duplicates = duplicate_exports_count > 0;
    let has_command_issues =
        !section.missing_handlers.is_empty() || !section.unused_handlers.is_empty();

    let short_path = shorten_path(&section.root);
    let git_label = match (section.git_branch.clone(), section.git_commit.clone()) {
        (Some(b), Some(c)) => format!("{}@{}", b, c),
        (Some(b), None) => b,
        _ => String::new(),
    };

    view! {
        <div id=view_id class=view_class>
            <header class="app-header">
                <div class="header-title">
                    <h1>{short_path}</h1>
                    <p class="header-path" title=section.root.clone()>{section.root.clone()}</p>
                    {(!git_label.is_empty()).then(|| view! {
                        <p class="header-path" style="margin-top:4px;color:var(--theme-text-tertiary)" title="git branch @ commit">
                            {git_label.clone()}
                        </p>
                    })}
                </div>
                <div class="header-stats">
                    <span class="stat-badge">
                        <span class="stat-badge-value">{file_count}</span>
                        <span class="stat-badge-label">"files"</span>
                    </span>
                    <span class="stat-badge">
                        <span class="stat-badge-value">{total_loc}</span>
                        <span class="stat-badge-label">"LOC"</span>
                    </span>
                    <span class="stat-badge">
                        <span class="stat-badge-value">{duplicate_exports_count}</span>
                        <span class="stat-badge-label">"dups"</span>
                    </span>
                </div>
            </header>

            <div class="app-content">
                <TabContent
                    root_id=root_id_overview
                    tab_name="overview"
                    active=true
                >
                    <div class="content-container">
                        <AnalysisSummary
                            files_analyzed=file_count
                            total_loc=total_loc
                            duplicate_exports=duplicate_exports_count
                            reexport_files=reexport_files_count
                            dynamic_imports=dynamic_imports_count
                        />
                        <AiInsightsPanel insights=section.insights.clone() />
                        <QuickCommandsPanel
                            root=section.root.clone()
                            has_duplicates=has_duplicates
                            has_command_issues=has_command_issues
                        />
                    </div>
                </TabContent>

                <TabContent
                    root_id=root_id_dups
                    tab_name="dups"
                    active=false
                >
                    <div class="content-container">
                        <DuplicateExportsTable
                            dups=section.ranked_dups.clone()
                        />
                        <CascadesList cascades=section.cascades.clone() />
                    </div>
                </TabContent>

                <TabContent
                    root_id=root_id_dynamic
                    tab_name="dynamic"
                    active=false
                >
                    <div class="content-container">
                        <DynamicImportsTable
                            imports=section.dynamic.clone()
                        />
                    </div>
                </TabContent>

                <TabContent
                    root_id=root_id_commands
                    tab_name="commands"
                    active=false
                >
                    <div class="content-container">
                        <TauriCommandCoverage
                            missing=section.missing_handlers.clone()
                            unused=section.unused_handlers.clone()
                            unregistered=section.unregistered_handlers.clone()
                            counts=section.command_counts
                            open_base=section.open_base.clone()
                        />
                    </div>
                </TabContent>

                <TabContent
                    root_id=root_id_pipelines
                    tab_name="pipelines"
                    active=false
                >
                    <div class="content-container">
                        <Pipelines bridges=section.command_bridges.clone() />
                    </div>
                </TabContent>

                <TabContent
                    root_id=root_id_crowds
                    tab_name="crowds"
                    active=false
                >
                    <Crowds crowds=section.crowds.clone() />
                </TabContent>

                <TabContent
                    root_id=root_id_cycles
                    tab_name="cycles"
                    active=false
                >
                    <div class="content-container">
                        <Cycles
                            strict_cycles=section.circular_imports.clone()
                            lazy_cycles=section.lazy_circular_imports.clone()
                        />
                    </div>
                </TabContent>

                <TabContent
                    root_id=root_id_dead
                    tab_name="dead"
                    active=false
                >
                    <div class="content-container">
                        <DeadCode dead_exports=section.dead_exports.clone() />
                    </div>
                </TabContent>

                <TabContent
                    root_id=root_id_twins
                    tab_name="twins"
                    active=false
                >
                    <div class="content-container">
                        <Twins twins=section.twins.clone() />
                    </div>
                </TabContent>

                <TabContent
                    root_id=root_id_coverage
                    tab_name="coverage"
                    active=false
                >
                    <div class="content-container">
                        <Coverage coverage_gaps=section.coverage_gaps.clone() />
                    </div>
                </TabContent>

                <TabContent
                    root_id=root_id_graph_tab
                    tab_name="graph"
                    active=false
                >
                    // Graph takes full width/height, so no content-container
                    <GraphContainer
                        section=section_clone
                        root_id=root_id_graph_container
                    />
                </TabContent>

                <TabContent
                    root_id=root_id_tree
                    tab_name="tree"
                    active=false
                >
                    <div class="content-container">
                        <TreeView
                            root_id=root_id_value
                            tree=section.tree.clone().unwrap_or_default()
                        />
                    </div>
                </TabContent>
            </div>
        </div>
    }
}
