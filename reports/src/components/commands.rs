//! Tauri command coverage component

use crate::types::CommandGap;
use leptos::prelude::*;
use std::collections::BTreeMap;

/// Tauri frontend-backend command coverage display
#[component]
pub fn TauriCommandCoverage(
    missing: Vec<CommandGap>,
    unused: Vec<CommandGap>,
    unregistered: Vec<CommandGap>,
    counts: (usize, usize),
    open_base: Option<String>,
) -> impl IntoView {
    view! {
        <h3>"Tauri command coverage"</h3>
        {if counts == (0, 0) {
            view! {
                <p class="muted">"No Tauri commands detected in this root."</p>
            }.into_any()
        } else if missing.is_empty() && unused.is_empty() && unregistered.is_empty() {
            view! {
                <p class="muted">"All frontend calls have matching registered handlers."</p>
            }.into_any()
        } else {
            view! {
                <table class="command-table">
                    <tr>
                        <th>"Missing handlers (FE→BE)"</th>
                        <th>"Handlers unused by FE"</th>
                        <th>"Handlers not registered in Tauri"</th>
                    </tr>
                    <tr>
                        <td>
                            <CommandGapGroup gaps=missing open_base=open_base.clone() />
                        </td>
                        <td>
                            <CommandGapGroup gaps=unused open_base=open_base.clone() />
                        </td>
                        <td>
                            <CommandGapGroup gaps=unregistered open_base=open_base />
                        </td>
                    </tr>
                </table>
            }.into_any()
        }}
    }
}

/// Grouped display of command gaps by module
#[component]
fn CommandGapGroup(gaps: Vec<CommandGap>, open_base: Option<String>) -> impl IntoView {
    if gaps.is_empty() {
        return view! { <span class="muted">"None"</span> }.into_any();
    }

    // Group by module (first two path segments)
    let mut groups: BTreeMap<String, Vec<(CommandGap, Vec<String>)>> = BTreeMap::new();

    for gap in gaps {
        let module = gap
            .locations
            .first()
            .map(|(p, _)| {
                let parts: Vec<&str> = p.split('/').collect();
                if parts.len() >= 2 {
                    format!("{}/{}", parts[0], parts[1])
                } else {
                    parts.first().unwrap_or(&"").to_string()
                }
            })
            .unwrap_or_default();

        let locs: Vec<String> = gap
            .locations
            .iter()
            .map(|(f, l)| linkify(open_base.as_deref(), f, *l))
            .collect();

        groups.entry(module).or_default().push((gap, locs));
    }

    view! {
        {groups.into_iter().map(|(module, items)| {
            let module_label = if module.is_empty() { "-".to_string() } else { module };
            view! {
                <div class="module-group">
                    <div class="module-header">{module_label}</div>
                    <div>
                        {items.into_iter().map(|(gap, locs)| {
                            let alias_info = gap.implementation_name
                                .filter(|impl_name| impl_name != &gap.name)
                                .map(|impl_name| format!(" (impl: {})", impl_name))
                                .unwrap_or_default();

                            view! {
                                <span class="command-pill">
                                    <code>{gap.name}</code>
                                    {(!alias_info.is_empty()).then(|| view! {
                                        <span class="muted">{alias_info}</span>
                                    })}
                                </span>
                                " "
                                <span class="muted" inner_html=locs.join(", ")></span>
                                <br/>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            }
        }).collect::<Vec<_>>()}
    }
    .into_any()
}

/// Create a link to open file at line (if open_base is set)
fn linkify(base: Option<&str>, file: &str, line: usize) -> String {
    if let Some(base) = base {
        let encoded_file = url_encode(file);
        format!(
            "<a href=\"{}/open?f={}&l={}\">{file}:{line}</a>",
            base, encoded_file, line
        )
    } else {
        format!("{file}:{line}")
    }
}

/// Simple URL encoding for file paths
fn url_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}
