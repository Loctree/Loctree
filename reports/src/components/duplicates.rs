//! Duplicate exports table component

use leptos::prelude::*;
use crate::types::RankedDup;

/// Table showing duplicate exports across files
#[component]
pub fn DuplicateExportsTable(
    dups: Vec<RankedDup>,
) -> impl IntoView {
    let count = dups.len();
    view! {
        <h3>"Top duplicate exports"</h3>
        {if dups.is_empty() {
            view! { <p class="muted">"None"</p> }.into_any()
        } else {
            view! {
                <p class="muted">{format!("{} duplicate export groups found", count)}</p>
                <table>
                    <tr>
                        <th>"Symbol"</th>
                        <th>"Files"</th>
                        <th>"Prod"</th>
                        <th>"Dev"</th>
                        <th>"Canonical"</th>
                        <th>"Refactor targets"</th>
                    </tr>
                    {dups.into_iter().map(|dup| {
                        view! {
                            <tr>
                                <td><code>{dup.name}</code></td>
                                <td>{dup.files.len()}</td>
                                <td>{dup.prod_count}</td>
                                <td>{dup.dev_count}</td>
                                <td><code>{dup.canonical}</code></td>
                                <td>{dup.refactors.join(", ")}</td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            }.into_any()
        }}
    }
}
