//! Root document component - the complete HTML page
//!
//! Implements the App Shell layout with Sidebar and Main Content areas.

use leptos::prelude::*;
use crate::styles::{REPORT_CSS, CSP};
use crate::types::ReportSection;
use crate::JsAssets;
use super::{ReportSectionView, Icon, ICON_FOLDER};

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
                            <div class="nav-section-title">"Project Roots"</div>
                            {sections.iter().enumerate().map(|(idx, section)| {
                                // Use index for robust ID generation
                                let view_id = format!("section-view-{}", idx);
                                let label = section.root.clone();
                                let active = idx == 0;
                                let class = if active { "nav-item active" } else { "nav-item" };
                                
                                view! {
                                    <a href="#" class=class data-view=view_id>
                                        <Icon path=ICON_FOLDER class="icon-sm" />
                                        {label}
                                    </a>
                                }
                            }).collect::<Vec<_>>()}
                        </nav>

                        <div class="app-footer">
                            "loctree v0.5.2"
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
        // Graph-specific scripts (only when assets are provided)
        {has_graph_assets.then(|| view! {
            <script src=js_assets.cytoscape_path.clone()></script>
            <script src=js_assets.dagre_path.clone()></script>
            <script src=js_assets.cytoscape_dagre_path.clone()></script>
            <script src=js_assets.cytoscape_cose_bilkent_path.clone()></script>
            <script>{include_str!("../graph_bootstrap.js")}</script>
        })}
        // App navigation script (always runs for section/tab switching)
        <script>{APP_SCRIPT}</script>
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
          // Follow system preference (CSS handles this via media query)
          // But we need to set class for JS-based checks
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
      
      // Sync with graph's dark mode checkboxes if present
      document.querySelectorAll('[data-role="dark"]').forEach(chk => {
          chk.checked = document.documentElement.classList.contains('dark');
      });
  };
  
  initTheme();
  
  const themeToggle = document.querySelector('[data-role="theme-toggle"]');
  if (themeToggle) {
      themeToggle.addEventListener('click', toggleTheme);
  }

  // 1. Section Switching (Sidebar)
  document.querySelectorAll('.nav-item[data-view]').forEach(btn => {
      btn.addEventListener('click', () => {
          const targetId = btn.dataset.view;
          
          // Update Sidebar
          document.querySelectorAll('.nav-item').forEach(b => b.classList.remove('active'));
          btn.classList.add('active');
          
          // Update Main View
          document.querySelectorAll('.section-view').forEach(v => v.classList.remove('active'));
          const view = document.getElementById(targetId);
          if(view) {
              view.classList.add('active');
              // Trigger resize if current tab is graph
              const activeTab = view.querySelector('.tab-panel.active');
              if(activeTab && activeTab.dataset.tabName === 'graph') {
                  window.dispatchEvent(new Event('resize'));
              }
          }
      });
  });

  // 2. Tab Switching (Header)
  document.querySelectorAll('.tab-bar').forEach(bar => {
     const scope = bar.dataset.tabScope;
     
     bar.querySelectorAll('.tab-btn').forEach(btn => {
         btn.addEventListener('click', () => {
             // Update Buttons
             bar.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
             btn.classList.add('active');
             
             // Update Panels
             const tabName = btn.dataset.tab;
             // Find panels within the same scope
             // We look for panels with matching data-tab-scope
             const panels = document.querySelectorAll(`.tab-panel[data-tab-scope="${scope}"]`);
             panels.forEach(p => {
                 const isActive = p.dataset.tabName === tabName;
                 p.classList.toggle('active', isActive);
                 
                 if (isActive && tabName === 'graph') {
                     // Trigger resize for Cytoscape
                     window.dispatchEvent(new Event('resize'));
                 }
             });
         });
     });
  });
})();
"#;
