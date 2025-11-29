use leptos::prelude::*;

#[component]
pub fn SliceDemo() -> impl IntoView {
    view! {
        <section id="slice" class="slice-demo">
            <div class="container">
                <div class="slice-grid">
                    <div class="slice-content">
                        <p class="section-eyebrow">"Holographic Slice"</p>
                        <h2 class="section-title">"3-layer context extraction"</h2>
                        <p class="hero-description">
                            "Give your AI agent exactly what it needs. Core file, its dependencies, "
                            "and everything that consumes it. No more, no less."
                        </p>
                        <div class="hero-actions" style="margin-top: 24px;">
                            <a href="https://github.com/LibraxisAI/loctree#holographic-slice-slice-command" target="_blank" class="btn btn-secondary">
                                "Read the docs"
                            </a>
                        </div>
                    </div>
                    <div class="slice-visual">
                        <div class="slice-layers">
                            <div class="slice-layer core">
                                <div class="slice-layer-header">
                                    <span class="slice-layer-title">"Core"</span>
                                    <span class="slice-layer-stats">"1 file, 150 LOC"</span>
                                </div>
                                <div class="slice-layer-files">
                                    <div>"src/App.tsx"</div>
                                </div>
                            </div>
                            <div class="slice-layer deps">
                                <div class="slice-layer-header">
                                    <span class="slice-layer-title">"Deps"</span>
                                    <span class="slice-layer-stats">"3 files, 420 LOC"</span>
                                </div>
                                <div class="slice-layer-files">
                                    <div>"[d1] src/hooks/useAuth.ts"</div>
                                    <div class="indent-1">"[d2] src/contexts/AuthContext.tsx"</div>
                                    <div class="indent-1">"[d2] src/utils/api.ts"</div>
                                </div>
                            </div>
                            <div class="slice-layer consumers">
                                <div class="slice-layer-header">
                                    <span class="slice-layer-title">"Consumers"</span>
                                    <span class="slice-layer-stats">"2 files, 180 LOC"</span>
                                </div>
                                <div class="slice-layer-files">
                                    <div>"src/main.tsx"</div>
                                    <div>"src/routes/index.tsx"</div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    }
}
