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
//! └── ReportSectionView (per analyzed directory)
//!     ├── TabBar
//!     ├── TabContent: Insights
//!     │   └── AiInsightsPanel
//!     ├── TabContent: Duplicates
//!     │   └── DuplicateExportsTable
//!     ├── TabContent: Cascades
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
//! use report_leptos::components::{TabBar, TabContent, AiInsightsPanel};
//!
//! view! {
//!     <TabBar section_idx=0 tabs=vec!["Insights", "Graph"] />
//!     <TabContent section_idx=0 tab_idx=0 active=true>
//!         <AiInsightsPanel insights=my_insights />
//!     </TabContent>
//! }
//! ```

mod cascades;
mod commands;
mod document;
mod duplicates;
mod dynamic_imports;
mod for_ai;
mod graph;
mod icons;
mod insights;
mod section;
mod tabs;

pub use cascades::CascadesList;
pub use commands::TauriCommandCoverage;
pub use document::ReportDocument;
pub use duplicates::DuplicateExportsTable;
pub use dynamic_imports::DynamicImportsTable;
pub use for_ai::AiSummaryPanel;
pub use graph::GraphContainer;
pub use icons::*;
pub use insights::AiInsightsPanel;
pub use section::ReportSectionView;
pub use tabs::{TabBar, TabContent};
