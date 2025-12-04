//! Dynamic imports table component

use leptos::prelude::*;

/// Table showing dynamic import patterns
#[component]
pub fn DynamicImportsTable(imports: Vec<(String, Vec<String>)>) -> impl IntoView {
    let count = imports.len();
    view! {
        <h3>"Dynamic imports"</h3>
        {if imports.is_empty() {
            view! { <p class="muted">"None"</p> }.into_any()
        } else {
            view! {
                <p class="muted">{format!("{} files with dynamic imports", count)}</p>
                <table>
                    <tr>
                        <th>"File"</th>
                        <th>"Sources"</th>
                    </tr>
                    {imports.into_iter().map(|(file, sources)| {
                        view! {
                            <tr>
                                <td><code>{file}</code></td>
                                <td>{sources.join(", ")}</td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            }.into_any()
        }}
    }
}
