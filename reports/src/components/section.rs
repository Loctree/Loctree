//! Report section component - wrapper for each analyzed root
//!
//! Implements the "Section View" layout with a sticky header and scrollable content.

use leptos::prelude::*;
use crate::types::ReportSection;
use super::{
    TabBar, TabContent, AiInsightsPanel, DuplicateExportsTable, 
    CascadesList, DynamicImportsTable, TauriCommandCoverage, GraphContainer
};

/// A complete report section for one analyzed root
#[component]
pub fn ReportSectionView(
    section: ReportSection,
    active: bool,
    view_id: String,
) -> impl IntoView {
    let root_id = section.root
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();

    let section_clone = section.clone();
    let view_class = if active { "section-view active" } else { "section-view" };
    
    // Stats for header
    let file_count = section.files_analyzed;

    view! {
        <div id=view_id class=view_class>
            <header class="app-header">
                <div class="header-title">
                    <h1>{section.root.clone()}</h1>
                    <p>"Analysis • " {file_count} " files"</p>
                </div>
                <TabBar root_id=root_id.clone() />
            </header>

            <div class="app-content">
                <TabContent 
                    root_id=root_id.clone() 
                    tab_name="overview" 
                    active=true
                >
                    <div class="content-container">
                        <AiInsightsPanel insights=section.insights.clone() />
                    </div>
                </TabContent>

                <TabContent 
                    root_id=root_id.clone() 
                    tab_name="dups" 
                    active=false
                >
                    <div class="content-container">
                        <DuplicateExportsTable
                            dups=section.ranked_dups.clone()
                            limit=section.analyze_limit
                        />
                        <CascadesList cascades=section.cascades.clone() />
                    </div>
                </TabContent>

                <TabContent 
                    root_id=root_id.clone() 
                    tab_name="dynamic" 
                    active=false
                >
                    <div class="content-container">
                        <DynamicImportsTable
                            imports=section.dynamic.clone()
                            limit=section.analyze_limit
                        />
                    </div>
                </TabContent>

                <TabContent 
                    root_id=root_id.clone() 
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
                    root_id=root_id.clone() 
                    tab_name="graph" 
                    active=false
                >
                    // Graph takes full width/height, so no content-container
                    <GraphContainer
                        section=section_clone
                        root_id=root_id
                    />
                </TabContent>
            </div>
        </div>
    }
}
