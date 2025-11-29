//! Re-export cascades list component

use leptos::prelude::*;

/// List showing re-export cascade chains
#[component]
pub fn CascadesList(cascades: Vec<(String, String)>) -> impl IntoView {
    view! {
        <h3>"Re-export cascades"</h3>
        {if cascades.is_empty() {
            view! { <p class="muted">"None"</p> }.into_any()
        } else {
            view! {
                <ul>
                    {cascades.into_iter().map(|(from, to)| {
                        view! {
                            <li>
                                <code>{from}</code>
                                " → "
                                <code>{to}</code>
                            </li>
                        }
                    }).collect::<Vec<_>>()}
                </ul>
            }.into_any()
        }}
    }
}
