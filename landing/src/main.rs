// loctree Landing Page — Leptos 0.8 Edition
// Developed with 💀 by The Loctree Team (c)2025

mod components;
mod pages;
mod sections;

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;
use pages::*;
use sections::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| view! { <App/> });
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <EasterEggs />
            <Nav />
            <main>
                <Routes fallback=|| view! { <NotFound /> }>
                    <Route path=path!("/") view=HomePage />
                    <Route path=path!("/features") view=FeaturesPage />
                    <Route path=path!("/docs") view=DocsPage />
                    <Route path=path!("/blog") view=BlogPage />
                </Routes>
            </main>
            <Footer />
        </Router>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <section class="not-found">
            <div class="container">
                <h1>"404"</h1>
                <p>"Page not found. Maybe it's a Dead Parrot?"</p>
                <a href="/" class="btn btn-primary">"Back to Home"</a>
            </div>
        </section>
    }
}
