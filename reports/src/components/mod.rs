//! Leptos UI components for rendering HTML reports.
//!
//! This module contains modular, reusable components for building
//! static HTML reports. Each component is a Leptos `#[component]`
//! function that can be composed to create custom report layouts.
//!
//! # Component Hierarchy
//!
//! ```text
//! ReportDocument
//! ├── Sidebar (navigation via nav-items)
//! └── ReportSectionView (per analyzed directory)
//!     ├── Header (path + stats badges)
//!     ├── TabContent: Overview
//!     │   ├── AnalysisSummary
//!     │   └── AiInsightsPanel
//!     ├── TabContent: Duplicates
//!     │   ├── DuplicateExportsTable
//!     │   └── CascadesList
//!     ├── TabContent: Dynamic Imports
//!     │   └── DynamicImportsTable
//!     ├── TabContent: Commands (Tauri)
//!     │   └── TauriCommandCoverage
//!     └── TabContent: Graph
//!         └── GraphContainer
//! ```
//!
//! # Usage
//!
//! Components are typically used via [`crate::render_report`], but
//! can be used directly for custom layouts:
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use report_leptos::components::{TabContent, AiInsightsPanel};
//!
//! view! {
//!     <TabContent root_id="my-project" tab_name="overview" active=true>
//!         <AiInsightsPanel insights=my_insights />
//!     </TabContent>
//! }
//! ```

mod cascades;
mod commands;
mod crowds;
mod cycles;
mod dead_code;
mod document;
mod duplicates;
mod dynamic_imports;
mod for_ai;
mod graph;
mod icons;
mod insights;
mod quick_commands;
mod section;
mod tabs;
mod tree;
mod twins;

pub use cascades::CascadesList;
pub use commands::TauriCommandCoverage;
pub use crowds::Crowds;
pub use cycles::Cycles;
pub use dead_code::DeadCode;
pub use document::ReportDocument;
pub use duplicates::DuplicateExportsTable;
pub use dynamic_imports::DynamicImportsTable;
pub use for_ai::AiSummaryPanel;
pub use graph::GraphContainer;
pub use icons::*;
pub use insights::{AiInsightsPanel, AnalysisSummary};
pub use quick_commands::QuickCommandsPanel;
pub use section::ReportSectionView;
pub use tabs::TabContent;
pub use tree::TreeView;
pub use twins::Twins;
