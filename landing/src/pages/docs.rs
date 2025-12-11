// Docs page - Manual + CLI reference + For Agents
use crate::sections::{CliReference, ForAgents, Manual};
use leptos::prelude::*;

#[component]
pub fn DocsPage() -> impl IntoView {
    view! {
        <section class="page-header">
            <div class="container">
                <h1 class="page-title">"Documentation"</h1>
                <p class="page-description">
                    "Everything you need to know about loctree"
                </p>
            </div>
        </section>
        <Manual />
        <ForAgents />
        <CliReference />
    }
}
