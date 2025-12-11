// Home page - landing hero + highlights
use crate::sections::{Hero, InstallSection, RealWorldResults, SliceDemo, StackDetection};
use leptos::prelude::*;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <Hero />
        <SliceDemo />
        <StackDetection />
        <RealWorldResults />
        <InstallSection />
    }
}
