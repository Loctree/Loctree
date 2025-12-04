use leptos::prelude::*;

#[component]
pub fn InstallSection() -> impl IntoView {
    let (copied, set_copied) = signal(false);

    let install_command = "cargo install loctree";

    let copy_cmd = move |_| {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(install_command);
            set_copied.set(true);
            set_timeout(
                move || set_copied.set(false),
                std::time::Duration::from_millis(2000),
            );
        }
    };

    view! {
        <section id="install" class="install-section">
            <div class="container">
                <div class="install-box-simple">
                    <div class="install-label">"INSTALL"</div>
                    <div class="install-command-box">
                        <code class="install-cmd">{install_command}</code>
                        <button class="copy-btn-small" on:click=copy_cmd>
                            {move || if copied.get() { "OK" } else { "COPY" }}
                        </button>
                    </div>
                    <div class="install-links">
                        <a href="https://github.com/Loctree/Loctree" target="_blank">"GitHub"</a>
                        <span class="sep">"|"</span>
                        <a href="https://crates.io/crates/loctree" target="_blank">"crates.io"</a>
                        <span class="sep">"|"</span>
                        <a href="https://docs.rs/loctree" target="_blank">"API Docs"</a>
                        <span class="sep">"|"</span>
                        <a href="https://github.com/Loctree/Loctree#readme" target="_blank">"README"</a>
                    </div>
                </div>
            </div>
        </section>
    }
}
