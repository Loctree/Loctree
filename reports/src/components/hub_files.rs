//! Hub files panel - context anchors for fast onboarding.

use leptos::prelude::*;

use crate::components::icons::{ICON_GRAPH, Icon};
use crate::types::HubFile;

/// Panel listing high-connectivity files for context anchoring.
#[component]
pub fn HubFilesPanel(hubs: Vec<HubFile>) -> impl IntoView {
    if hubs.is_empty() {
        return view! { "" }.into_any();
    }

    view! {
        <div class="panel hub-files-panel">
            <h3>
                <Icon path=ICON_GRAPH class="icon-sm" />
                "Context Anchors"
            </h3>
            <p class="muted">"High-connectivity files to start reading or slicing."</p>
            <table class="data-table hub-table">
                <thead>
                    <tr>
                        <th>"File"</th>
                        <th>"LOC"</th>
                        <th>"Imports"</th>
                        <th>"Exports"</th>
                        <th>"Importers"</th>
                        <th>"Commands"</th>
                        <th>"Slice"</th>
                    </tr>
                </thead>
                <tbody>
                    {hubs.into_iter().map(|hub| {
                        let slice_cmd = hub.slice_cmd.clone();
                        view! {
                            <tr>
                                <td><code>{hub.path}</code></td>
                                <td>{hub.loc}</td>
                                <td>{hub.imports_count}</td>
                                <td>{hub.exports_count}</td>
                                <td>{hub.importers_count}</td>
                                <td>{hub.commands_count}</td>
                                <td>
                                    <code>{slice_cmd.clone()}</code>
                                    <button class="copy-btn" data-copy=slice_cmd>"Copy"</button>
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

// Tests live in reports/src/lib.rs via render_report.
