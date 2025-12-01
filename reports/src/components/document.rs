//! Root document component - the complete HTML page
//!
//! Implements the App Shell layout with Sidebar and Main Content areas.

use leptos::prelude::*;
use crate::styles::{REPORT_CSS, CSP};
use crate::types::ReportSection;
use crate::JsAssets;
use super::{ReportSectionView, Icon, ICON_SQUARES_FOUR, ICON_COPY, ICON_LIGHTNING, ICON_TERMINAL, ICON_GRAPH};

/// The complete HTML document for the report
#[component]
pub fn ReportDocument(
    sections: Vec<ReportSection>,
    js_assets: JsAssets,
) -> impl IntoView {
    view! {
        <html>
            <head>
                <meta charset="UTF-8" />
                <meta http-equiv="Content-Security-Policy" content=CSP />
                <title>"loctree report"</title>
                <style>{REPORT_CSS}</style>
            </head>
            <body>
                <div class="app-shell">
                    <aside class="app-sidebar">
                        <div class="sidebar-header">
                            <div class="logo-box">
                                <span style="color:var(--theme-accent)">"loctree"</span>
                                <span style="opacity:0.5">"report"</span>
                            </div>
                            <button class="theme-toggle" data-role="theme-toggle" title="Toggle light/dark mode">
                                <svg class="theme-icon-light" xmlns="http://www.w3.org/2000/svg" width="18" height="18" fill="currentColor" viewBox="0 0 256 256">
                                    <path d="M120,40V16a8,8,0,0,1,16,0V40a8,8,0,0,1-16,0Zm72,88a64,64,0,1,1-64-64A64.07,64.07,0,0,1,192,128Zm-16,0a48,48,0,1,0-48,48A48.05,48.05,0,0,0,176,128ZM58.34,69.66A8,8,0,0,0,69.66,58.34l-16-16A8,8,0,0,0,42.34,53.66Zm0,116.68-16,16a8,8,0,0,0,11.32,11.32l16-16a8,8,0,0,0-11.32-11.32ZM192,72a8,8,0,0,0,5.66-2.34l16-16a8,8,0,0,0-11.32-11.32l-16,16A8,8,0,0,0,192,72Zm5.66,114.34a8,8,0,0,0-11.32,11.32l16,16a8,8,0,0,0,11.32-11.32ZM48,128a8,8,0,0,0-8-8H16a8,8,0,0,0,0,16H40A8,8,0,0,0,48,128Zm80,80a8,8,0,0,0-8,8v24a8,8,0,0,0,16,0V216A8,8,0,0,0,128,208Zm112-88H216a8,8,0,0,0,0,16h24a8,8,0,0,0,0-16Z"></path>
                                </svg>
                                <svg class="theme-icon-dark" xmlns="http://www.w3.org/2000/svg" width="18" height="18" fill="currentColor" viewBox="0 0 256 256">
                                    <path d="M233.54,142.23a8,8,0,0,0-8-2,88.08,88.08,0,0,1-109.8-109.8,8,8,0,0,0-10-10,104.84,104.84,0,0,0-52.91,37A104,104,0,0,0,136,224a103.09,103.09,0,0,0,62.52-20.88,104.84,104.84,0,0,0,37-52.91A8,8,0,0,0,233.54,142.23ZM188.9,190.34A88,88,0,0,1,65.66,67.11a89,89,0,0,1,31.4-26A106,106,0,0,0,96,56,104.11,104.11,0,0,0,200,160a106,106,0,0,0,14.92-1.06A89,89,0,0,1,188.9,190.34Z"></path>
                                </svg>
                            </button>
                        </div>

                        <nav class="sidebar-nav">
                            <button class="nav-item active" data-tab="overview">
                                <Icon path=ICON_SQUARES_FOUR class="icon-sm" />
                                "Overview"
                            </button>
                            <button class="nav-item" data-tab="dups">
                                <Icon path=ICON_COPY class="icon-sm" />
                                "Duplicates"
                            </button>
                            <button class="nav-item" data-tab="dynamic">
                                <Icon path=ICON_LIGHTNING class="icon-sm" />
                                "Dynamic imports"
                            </button>
                            <button class="nav-item" data-tab="commands">
                                <Icon path=ICON_TERMINAL class="icon-sm" />
                                "Tauri coverage"
                            </button>
                            <button class="nav-item" data-tab="graph">
                                <Icon path=ICON_GRAPH class="icon-sm" />
                                "Graph"
                            </button>
                        </nav>

                        <div class="app-footer">
                            "loctree v0.5.6-dev"
                            <br />
                            <span style="color:var(--theme-text-tertiary)">"Snapshot"</span>
                        </div>
                    </aside>

                    <main class="app-main">
                        {sections.into_iter().enumerate().map(|(idx, section)| {
                            let view_id = format!("section-view-{}", idx);
                            let active = idx == 0;
                            view! {
                                <ReportSectionView
                                    section=section
                                    active=active
                                    view_id=view_id
                                />
                            }
                        }).collect::<Vec<_>>()}
                    </main>
                </div>

                <GraphScripts js_assets=js_assets />
            </body>
        </html>
    }
}

/// JavaScript for graph initialization and UI interactivity
#[component]
fn GraphScripts(js_assets: JsAssets) -> impl IntoView {
    let has_graph_assets = !js_assets.cytoscape_path.is_empty();

    view! {
        // App navigation script FIRST (must run even if graph fails)
        <script>{APP_SCRIPT}</script>
        // Graph-specific scripts (only when assets are provided)
        // Load order matters: layout-base -> cose-base -> cytoscape-cose-bilkent
        {has_graph_assets.then(|| view! {
            <script src=js_assets.cytoscape_path.clone()></script>
            <script src=js_assets.dagre_path.clone()></script>
            <script src=js_assets.cytoscape_dagre_path.clone()></script>
            <script src=js_assets.layout_base_path.clone()></script>
            <script src=js_assets.cose_base_path.clone()></script>
            <script src=js_assets.cytoscape_cose_bilkent_path.clone()></script>
            <script>{include_str!("../graph_bootstrap.js")}</script>
        })}
    }
}

/// Application logic (Navigation, Tabs, Resize, Theme Toggle)
const APP_SCRIPT: &str = r#"
(() => {
  // 0. Theme Initialization & Toggle
  const initTheme = () => {
      const stored = localStorage.getItem('loctree-theme');
      if (stored === 'dark') {
          document.documentElement.classList.add('dark');
          document.documentElement.classList.remove('light');
      } else if (stored === 'light') {
          document.documentElement.classList.add('light');
          document.documentElement.classList.remove('dark');
      } else {
          if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
              document.documentElement.classList.add('dark');
          }
      }
  };

  const toggleTheme = () => {
      const isDark = document.documentElement.classList.contains('dark') ||
          (!document.documentElement.classList.contains('light') &&
           window.matchMedia('(prefers-color-scheme: dark)').matches);

      if (isDark) {
          document.documentElement.classList.remove('dark');
          document.documentElement.classList.add('light');
          localStorage.setItem('loctree-theme', 'light');
      } else {
          document.documentElement.classList.add('dark');
          document.documentElement.classList.remove('light');
          localStorage.setItem('loctree-theme', 'dark');
      }

      document.querySelectorAll('[data-role="dark"]').forEach(chk => {
          chk.checked = document.documentElement.classList.contains('dark');
      });
  };

  initTheme();

  const themeToggle = document.querySelector('[data-role="theme-toggle"]');
  if (themeToggle) {
      themeToggle.addEventListener('click', toggleTheme);
  }

  // 1. Sidebar Navigation (Tab Switching)
  document.querySelectorAll('.sidebar-nav .nav-item[data-tab]').forEach(btn => {
      btn.addEventListener('click', () => {
          const tabName = btn.dataset.tab;

          // Update Sidebar buttons
          document.querySelectorAll('.sidebar-nav .nav-item').forEach(b => b.classList.remove('active'));
          btn.classList.add('active');

          // Update all tab panels across all sections
          document.querySelectorAll('.tab-panel').forEach(p => {
              const isActive = p.dataset.tabName === tabName;
              p.classList.toggle('active', isActive);

              if (isActive && tabName === 'graph') {
                  window.dispatchEvent(new Event('resize'));
              }
          });

          // Also update header tab-bar buttons if present (for visual sync)
          document.querySelectorAll('.tab-bar .tab-btn').forEach(b => {
              b.classList.toggle('active', b.dataset.tab === tabName);
          });
      });
  });

  // 2. Header Tab Switching (if still present, syncs with sidebar)
  document.querySelectorAll('.tab-bar .tab-btn').forEach(btn => {
      btn.addEventListener('click', () => {
          const tabName = btn.dataset.tab;

          // Trigger sidebar button click to keep everything in sync
          const sidebarBtn = document.querySelector(`.sidebar-nav .nav-item[data-tab="${tabName}"]`);
          if (sidebarBtn) {
              sidebarBtn.click();
          }
      });
  });
})();
"#;
