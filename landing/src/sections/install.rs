use leptos::prelude::*;

#[component]
pub fn InstallSection() -> impl IntoView {
    let (copied_cargo, set_copied_cargo) = signal(false);
    let (copied_brew, set_copied_brew) = signal(false);

    let cargo_command = "cargo install loctree";
    let brew_command = "brew install loctree/loctree/loctree";

    let copy_cargo = move |_| {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(cargo_command);
            set_copied_cargo.set(true);
            set_timeout(
                move || set_copied_cargo.set(false),
                std::time::Duration::from_millis(2000),
            );
        }
    };

    let copy_brew = move |_| {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(brew_command);
            set_copied_brew.set(true);
            set_timeout(
                move || set_copied_brew.set(false),
                std::time::Duration::from_millis(2000),
            );
        }
    };

    view! {
        <section id="install" class="install-section">
            <div class="container">
                <div class="install-box-simple">
                    <div class="install-label">"INSTALL"</div>

                    <div class="install-methods">
                        <div class="install-method">
                            <span class="method-label">"Cargo"</span>
                            <div class="install-command-box">
                                <code class="install-cmd">{cargo_command}</code>
                                <button class="copy-btn-small" on:click=copy_cargo>
                                    {move || if copied_cargo.get() { "OK" } else { "COPY" }}
                                </button>
                            </div>
                        </div>

                        <div class="install-method">
                            <span class="method-label">"Homebrew"</span>
                            <div class="install-command-box">
                                <code class="install-cmd">{brew_command}</code>
                                <button class="copy-btn-small" on:click=copy_brew>
                                    {move || if copied_brew.get() { "OK" } else { "COPY" }}
                                </button>
                            </div>
                        </div>
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
