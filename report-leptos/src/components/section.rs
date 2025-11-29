//! Report section component - wrapper for each analyzed root

use leptos::prelude::*;
use crate::types::ReportSection;
use super::{TabBar, TabContent, AiInsightsPanel, DuplicateExportsTable, CascadesList, DynamicImportsTable, TauriCommandCoverage, GraphContainer};

/// A complete report section for one analyzed root
#[component]
pub fn ReportSectionView(section: ReportSection) -> impl IntoView {
    let root_id = section.root
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();

    let section_clone = section.clone();

    view! {
        <section class="report-section">
            <div class="section-head">
                <div>
                    <h2>{section.root.clone()}</h2>
                    <p class="muted">"Files analyzed: " {section.files_analyzed}</p>
                </div>
                <span class="pill">"Graph is docked in the drawer below"</span>
            </div>

            <TabBar root_id=root_id.clone() />

            <TabContent
                root_id=root_id.clone()
                tab_name="overview"
                active=true
            >
                <AiInsightsPanel insights=section.insights.clone() />
            </TabContent>

            <TabContent root_id=root_id.clone() tab_name="dups" active=false>
                <DuplicateExportsTable
                    dups=section.ranked_dups.clone()
                    limit=section.analyze_limit
                />
                <CascadesList cascades=section.cascades.clone() />
            </TabContent>

            <TabContent root_id=root_id.clone() tab_name="dynamic" active=false>
                <DynamicImportsTable
                    imports=section.dynamic.clone()
                    limit=section.analyze_limit
                />
            </TabContent>

            <TabContent root_id=root_id.clone() tab_name="commands" active=false>
                <TauriCommandCoverage
                    missing=section.missing_handlers.clone()
                    unused=section.unused_handlers.clone()
                    unregistered=section.unregistered_handlers.clone()
                    counts=section.command_counts
                    open_base=section.open_base.clone()
                />
            </TabContent>

            <TabContent root_id=root_id.clone() tab_name="graph" active=false>
                <GraphContainer
                    section=section_clone
                    root_id=root_id
                />
            </TabContent>
        </section>
    }
}
