// Blog page
use crate::sections::Blog;
use leptos::prelude::*;

#[component]
pub fn BlogPage() -> impl IntoView {
    view! {
        <section class="page-header">
            <div class="container">
                <h1 class="page-title">"Blog"</h1>
                <p class="page-description">
                    "Updates, tutorials, and insights from the Loctree team"
                </p>
            </div>
        </section>
        <Blog />
    }
}
