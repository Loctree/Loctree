use leptos::prelude::*;

/// Blog post metadata
struct BlogPost {
    title: &'static str,
    subtitle: &'static str,
    tag: &'static str,
    tag_class: &'static str,
    date: &'static str,
    read_time: &'static str,
    url: &'static str,
}

const POSTS: &[BlogPost] = &[
    BlogPost {
        title: "Dogfooding: Loctree Fixes Its Own Bug",
        subtitle: "AI agent runs loctree on loctree, discovers false positive in cycle detection, implements fix",
        tag: "META",
        tag_class: "tag-meta",
        date: "2025-12-01",
        read_time: "5 min",
        url: "https://github.com/nicosj/loctree/blob/main/docs/use-cases/dogfooding-false-positive-fix.md",
    },
    BlogPost {
        title: "AI Agent Implements 6 Tauri Handlers",
        subtitle: "Graph-aware navigation enables 120x speedup over grep-based workflows",
        tag: "TAURI",
        tag_class: "tag-tauri",
        date: "2025-12-01",
        read_time: "8 min",
        url: "https://github.com/nicosj/loctree/blob/main/docs/use-cases/ai-agent-feature-implementation.md",
    },
    BlogPost {
        title: "Fixing 6 Circular Imports in AnythingLLM",
        subtitle: "Real session transcript of AI agent using loct cycles to diagnose and fix dependency cycles",
        tag: "CYCLES",
        tag_class: "tag-cycles",
        date: "2025-11-30",
        read_time: "6 min",
        url: "https://github.com/nicosj/loctree/blob/main/docs/use-cases/ai-agent-circular-imports-fix.md",
    },
    BlogPost {
        title: "Vista: Tauri Contract Analysis",
        subtitle: "Finding missing handlers, unused commands, and FE↔BE mismatches in production app",
        tag: "CONTRACTS",
        tag_class: "tag-contracts",
        date: "2025-11-28",
        read_time: "10 min",
        url: "https://github.com/nicosj/loctree/blob/main/docs/use-cases/ai-agent-vista-tauri-contract.md",
    },
];

#[component]
pub fn Blog() -> impl IntoView {
    view! {
        <section id="blog" class="blog">
            <div class="container">
                <div class="section-header">
                    <p class="section-eyebrow">"From the Trenches"</p>
                    <h2 class="section-title">"AI Agent Case Studies"</h2>
                    <p class="section-description">
                        "Real transcripts of AI agents using loctree to solve production problems. "
                        "No marketing fluff—just terminal output and results."
                    </p>
                </div>
                <div class="blog-grid">
                    {POSTS.iter().map(|post| {
                        view! {
                            <a href={post.url} target="_blank" rel="noopener" class="blog-card">
                                <div class="blog-card-header">
                                    <span class={format!("blog-tag {}", post.tag_class)}>{post.tag}</span>
                                    <span class="blog-meta">{post.date}" · "{post.read_time}</span>
                                </div>
                                <h3 class="blog-title">{post.title}</h3>
                                <p class="blog-subtitle">{post.subtitle}</p>
                                <div class="blog-cta">
                                    "Read case study"
                                    <span class="arrow">"→"</span>
                                </div>
                            </a>
                        }
                    }).collect::<Vec<_>>()}
                </div>
                <div class="blog-footer">
                    <p class="blog-note">
                        "All case studies are real AI agent sessions. "
                        "Want to contribute your own? "
                        <a href="https://github.com/nicosj/loctree/issues" target="_blank">"Open an issue"</a>
                        " or submit a PR to docs/use-cases/"
                    </p>
                </div>
            </div>
        </section>
    }
}
