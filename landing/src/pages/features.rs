// Features page - full feature list
use crate::sections::Features;
use leptos::prelude::*;

#[component]
pub fn FeaturesPage() -> impl IntoView {
    view! {
        <section class="page-header">
            <div class="container">
                <h1 class="page-title">"Features"</h1>
                <p class="page-description">
                    "Everything loctree can do for your codebase"
                </p>
            </div>
        </section>
        <Features />
    }
}
