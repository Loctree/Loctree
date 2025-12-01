use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="footer">
            <div class="container">
                <div class="footer-brand">
                    <span class="footer-logo">
                        <img src="assets/loctree-logo.png" alt="loctree" />
                    </span>
                    <span class="footer-title">"loctree"</span>
                </div>
                <div class="footer-links">
                    <a href="https://github.com/Loctree/Loctree" target="_blank" class="footer-link">"GitHub"</a>
                    <a href="https://crates.io/crates/loctree" target="_blank" class="footer-link">"crates.io"</a>
                    <a href="https://docs.rs/loctree" target="_blank" class="footer-link">"docs.rs"</a>
                    <a href="https://github.com/Loctree/Loctree/blob/main/LICENSE" target="_blank" class="footer-link">"MIT License"</a>
                </div>
                <p class="footer-copyright">
                    "Developed with 💀 by The Loctree Team (c)2025 "
                </p>
            </div>
        </footer>
    }
}
