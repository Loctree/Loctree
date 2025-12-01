// loctree Landing Page — Leptos 0.8 Edition
// Created by M&K (c)2025 The LibraxisAI Team

mod sections;

use leptos::prelude::*;
use sections::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| view! { <App/> });
}

#[component]
fn App() -> impl IntoView {
    view! {
        <EasterEggs />
        <Nav />
        <main>
            <Hero />
            <Features />
            <SliceDemo />
            <StackDetection />
            <RealWorldResults />
            <ForAgents />
            <CliReference />
            <InstallSection />
        </main>
        <Footer />
    }
}
