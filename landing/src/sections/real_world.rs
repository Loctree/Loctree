use leptos::prelude::*;

#[component]
pub fn RealWorldResults() -> impl IntoView {
    view! {
        <section class="real-world">
            <div class="container">
                <div class="section-header">
                    <p class="section-eyebrow">"Real World Results"</p>
                    <h2 class="section-title">"Tested on production Tauri app"</h2>
                    <p class="section-description">
                        "1308 files. 204 Tauri commands. Here's what loctree found."
                    </p>
                </div>
                <div class="results-grid">
                    <div class="result-card highlight">
                        <div class="result-number">"51"</div>
                        <div class="result-label">"Unused Handlers"</div>
                        <div class="result-detail">"BE commands not called by FE"</div>
                    </div>
                    <div class="result-card">
                        <div class="result-number">"36"</div>
                        <div class="result-label">"Duplicate Exports"</div>
                        <div class="result-detail">"Same symbol exported multiple times"</div>
                    </div>
                    <div class="result-card">
                        <div class="result-number">"58"</div>
                        <div class="result-label">"Dynamic Imports"</div>
                        <div class="result-detail">"React.lazy() and import() tracked"</div>
                    </div>
                    <div class="result-card">
                        <div class="result-number">"0"</div>
                        <div class="result-label">"Missing Handlers"</div>
                        <div class="result-detail">"FE->BE integrity verified"</div>
                    </div>
                </div>
                <div class="results-breakdown">
                    <h3 class="breakdown-title">"Unused handlers breakdown"</h3>
                    <div class="breakdown-grid">
                        <div class="breakdown-item keep">
                            <span class="breakdown-count">"13"</span>
                            <span class="breakdown-label">"System Menu"</span>
                            <span class="breakdown-status">"MUST KEEP"</span>
                        </div>
                        <div class="breakdown-item wip">
                            <span class="breakdown-count">"12"</span>
                            <span class="breakdown-label">"Gateway/Realtime"</span>
                            <span class="breakdown-status">"WIP"</span>
                        </div>
                        <div class="breakdown-item backend">
                            <span class="breakdown-count">"8"</span>
                            <span class="breakdown-label">"Backend-only"</span>
                            <span class="breakdown-status">"INTERNAL"</span>
                        </div>
                        <div class="breakdown-item dead">
                            <span class="breakdown-count">"18"</span>
                            <span class="breakdown-label">"Dead Parrots"</span>
                            <span class="breakdown-status">"TO REVIEW"</span>
                        </div>
                    </div>
                    <p class="breakdown-note">
                        "loctree doesn't just find problems—it helps you categorize them."
                    </p>
                </div>
            </div>
        </section>
    }
}
