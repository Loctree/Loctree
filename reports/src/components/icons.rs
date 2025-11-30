//! SVG icon components using Phosphor Icons.
//!
//! This module provides inline SVG icons for the report UI.
//! All icons are from the [Phosphor Icons](https://phosphoricons.com/) library (Regular weight).

use leptos::prelude::*;

/// Renders an inline SVG icon from a path data string.
///
/// # Props
///
/// * `path` - SVG path data (d attribute)
/// * `size` - Icon size in pixels (default: "20")
/// * `color` - Fill color (default: "currentColor")
/// * `class` - Additional CSS classes (default: "")
///
/// # Example
///
/// ```rust,ignore
/// view! { <Icon path=ICON_FOLDER size="24" /> }
/// ```
#[component]
pub fn Icon(
    /// SVG path data (the `d` attribute value)
    #[prop(into)]
    path: &'static str,
    /// Icon size in pixels
    #[prop(default = "20")]
    size: &'static str,
    /// Fill color (CSS color value)
    #[prop(default = "currentColor")]
    color: &'static str,
    /// Additional CSS class names
    #[prop(default = "")]
    class: &'static str,
) -> impl IntoView {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width=size
            height=size
            fill=color
            viewBox="0 0 256 256"
            class=class
        >
            <path d=path></path>
        </svg>
    }
}

// =============================================================================
// Phosphor Icons (Regular weight) - https://phosphoricons.com/
// =============================================================================

/// Grid/dashboard icon (SquaresFour)
pub const ICON_SQUARES_FOUR: &str = "M104,48H48A16,16,0,0,0,32,64v56a16,16,0,0,0,16,16h56a16,16,0,0,0,16-16V64A16,16,0,0,0,104,48Zm0,72H48V64h56Zm104-72H152a16,16,0,0,0-16,16v56a16,16,0,0,0,16,16h56a16,16,0,0,0,16-16V64A16,16,0,0,0,208,48Zm0,72H152V64h56ZM104,152H48a16,16,0,0,0-16,16v56a16,16,0,0,0,16,16h56a16,16,0,0,0,16-16V168A16,16,0,0,0,104,152Zm0,72H48V168h56Zm104-72H152a16,16,0,0,0-16,16v56a16,16,0,0,0,16,16h56a16,16,0,0,0,16-16V168A16,16,0,0,0,208,152Zm0,72H152V168h56Z";

/// Dependency graph/network icon
pub const ICON_GRAPH: &str = "M208,152a32.06,32.06,0,0,0-25.87,13.26l-52.3-29.06a32,32,0,0,0,0-16.4l52.3-29.06A32.06,32.06,0,0,0,208,104a32,32,0,1,0-31.71-28.29L124,104.78a32,32,0,1,0,0,46.44l52.3,29.06A32,32,0,1,0,208,152ZM208,56a16,16,0,1,1-16,16A16,16,0,0,1,208,56ZM80,128a16,16,0,1,1,16,16A16,16,0,0,1,80,128Zm128,88a16,16,0,1,1,16-16A16,16,0,0,1,208,216Z";

/// Copy/duplicate files icon
pub const ICON_COPY: &str = "M216,32H88a8,8,0,0,0-8,8V80H40a8,8,0,0,0-8,8V216a8,8,0,0,0,8,8H168a8,8,0,0,0,8-8V176h40a8,8,0,0,0,8-8V40A8,8,0,0,0,216,32ZM160,208H48V96H160Zm48-48H176V88a8,8,0,0,0-8-8H96V48H208Z";

/// Warning/alert circle icon
pub const ICON_WARNING_CIRCLE: &str = "M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm0,192a88,88,0,1,1,88-88A88.1,88.1,0,0,1,128,216Zm-8-80V80a8,8,0,0,1,16,0v56a8,8,0,0,1-16,0Zm8,40a12,12,0,1,1,12-12A12,12,0,0,1,128,176Z";

/// Robot/AI icon
pub const ICON_ROBOT: &str = "M224,64H186.34A48.11,48.11,0,0,0,144,32.23V24a8,8,0,0,0-16,0v8.23A48.11,48.11,0,0,0,85.66,64H48A16,16,0,0,0,32,80v40a16,16,0,0,0,16,16h7.53A56.06,56.06,0,0,0,48,160a56,56,0,0,0,112,0,56.06,56.06,0,0,0-7.53-24H208a16,16,0,0,0,16-16V80A16,16,0,0,0,224,64ZM48,120V80H81.43A48.16,48.16,0,0,0,80,88v32Zm56,88a40,40,0,1,1,40-40A40,40,0,0,1,104,208Zm64-88H128V88a48,48,0,0,0-1.43-8h82.86A48.16,48.16,0,0,0,208,120Zm40-16H178.09a56.25,56.25,0,0,0,3.91-16h26v32ZM84,160a12,12,0,1,1,12,12A12,12,0,0,1,84,160Zm56,0a12,12,0,1,1,12,12A12,12,0,0,1,140,160Z";

/// Lightning bolt icon (dynamic imports)
pub const ICON_LIGHTNING: &str = "M215.79,118.17a8,8,0,0,0-5-5.66L153.18,90.9l14.66-73.33a8,8,0,0,0-13.69-7L37.71,143.17A8,8,0,0,0,44.22,156l57.6,11.52L87.16,240.83A8,8,0,0,0,95,248a7.72,7.72,0,0,0,1.57-.16l116.67-46.67a8,8,0,0,0,2.55-14.5ZM96.82,224,116,128a8,8,0,0,0-6.51-9.54L52.22,107,159.18,32,140,128a8,8,0,0,0,6.51,9.54l57.27,11.45Z";

/// Terminal/command line icon
pub const ICON_TERMINAL: &str = "M216,48H40A16,16,0,0,0,24,64V192a16,16,0,0,0,16,16H216a16,16,0,0,0,16-16V64A16,16,0,0,0,216,48ZM40,64H216V192H40V64Zm84,84H92a8,8,0,0,1-5.66-13.66l32-32a8,8,0,0,1,11.32,11.32L103.31,140l26.35,26.34A8,8,0,0,1,124,148Zm92,0H152a8,8,0,0,1,0-16h64a8,8,0,0,1,0,16Z";

/// Caret/chevron down icon
pub const ICON_CARET_DOWN: &str = "M213.66,101.66l-80,80a8,8,0,0,1-11.32,0l-80-80A8,8,0,0,1,53.66,90.34L128,164.69l74.34-74.35a8,8,0,0,1,11.32,11.32Z";

/// Folder icon
pub const ICON_FOLDER: &str = "M216,72H130.67L102.93,35.06A20,20,0,0,0,86.93,27.21H40A20,20,0,0,0,20,47.21V208.79A20,20,0,0,0,40,228.79H216a20,20,0,0,0,20-20V92A20,20,0,0,0,216,72Zm4,136.79a4,4,0,0,1-4,4H40a4,4,0,0,1-4-4V47.21a4,4,0,0,1,4-4H86.93a4,4,0,0,1,3.2,1.57L118,82.39A20,20,0,0,0,134,88H216a4,4,0,0,1,4,4Z";
