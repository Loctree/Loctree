// Docs page - CLI reference + For Agents
use crate::sections::{CliReference, ForAgents};
use leptos::prelude::*;

#[component]
pub fn DocsPage() -> impl IntoView {
    view! {
        <section class="page-header">
            <div class="container">
                <h1 class="page-title">"Documentation"</h1>
                <p class="page-description">
                    "CLI reference and AI agent integration guides"
                </p>
            </div>
        </section>
        <ForAgents />
        <CliReference />
    }
}
