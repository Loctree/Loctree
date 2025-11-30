use leptos::prelude::*;

#[component]
pub fn StackDetection() -> impl IntoView {
    view! {
        <section class="stack-detection">
            <div class="container">
                <div class="section-header">
                    <p class="section-eyebrow">"Auto-detect"</p>
                    <h2 class="section-title">"Zero configuration needed"</h2>
                    <p class="section-description">
                        "loctree detects your project type and configures sensible defaults automatically."
                    </p>
                </div>
                <div class="stack-grid">
                    <StackCard icon="Rs" name="Rust" marker="Cargo.toml" ignores="target/" />
                    <StackCard icon="TS" name="TypeScript" marker="tsconfig.json" ignores="node_modules/" />
                    <StackCard icon="Py" name="Python" marker="pyproject.toml" ignores=".venv/" />
                    <StackCard icon="Ta" name="Tauri" marker="src-tauri/" ignores="all of the above" />
                    <StackCard icon="Vi" name="Vite" marker="vite.config.*" ignores="dist/" />
                </div>
            </div>
        </section>
    }
}

#[component]
fn StackCard(
    icon: &'static str,
    name: &'static str,
    marker: &'static str,
    ignores: &'static str,
) -> impl IntoView {
    view! {
        <div class="stack-card">
            <div class="stack-icon">{icon}</div>
            <div class="stack-name">{name}</div>
            <div class="stack-marker">{marker}</div>
            <div class="stack-ignores">"-> ignores "{ignores}</div>
        </div>
    }
}
