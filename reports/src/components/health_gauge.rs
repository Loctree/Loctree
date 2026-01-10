//! Health score gauge component - circular SVG visualization

use leptos::prelude::*;

/// Circular gauge displaying health score 0-100
///
/// Color coding:
/// - Red (0-40): Critical issues
/// - Orange (41-69): Needs attention
/// - Green (70-100): Healthy
#[component]
pub fn HealthScoreGauge(score: u8) -> impl IntoView {
    let score = score.min(100); // Clamp to 0-100

    // Color based on score range
    let color = match score {
        0..=40 => "#e74c3c",  // Red - critical
        41..=69 => "#e67e22", // Orange - warning
        _ => "#27ae60",       // Green - healthy
    };

    // SVG arc calculation
    // Circle parameters: center (60,60), radius 50
    let radius = 50.0_f64;
    let circumference = 2.0 * std::f64::consts::PI * radius;
    let progress = (score as f64 / 100.0) * circumference;
    let dash_offset = circumference - progress;

    // Background track color (darker version)
    let track_color = "#2a2a2a";

    view! {
        <div class="health-gauge">
            <svg
                viewBox="0 0 120 120"
                width="140"
                height="140"
                class="gauge-svg"
            >
                // Background circle (track)
                <circle
                    cx="60"
                    cy="60"
                    r={radius.to_string()}
                    fill="none"
                    stroke={track_color}
                    stroke-width="10"
                    stroke-linecap="round"
                />

                // Progress arc
                <circle
                    cx="60"
                    cy="60"
                    r={radius.to_string()}
                    fill="none"
                    stroke={color}
                    stroke-width="10"
                    stroke-linecap="round"
                    stroke-dasharray={circumference.to_string()}
                    stroke-dashoffset={dash_offset.to_string()}
                    transform="rotate(-90 60 60)"
                    class="gauge-progress"
                />

                // Score text in center
                <text
                    x="60"
                    y="55"
                    text-anchor="middle"
                    dominant-baseline="middle"
                    fill={color}
                    font-size="32"
                    font-weight="700"
                    font-family="system-ui, -apple-system, sans-serif"
                >
                    {score.to_string()}
                </text>

                // "Health" label below score
                <text
                    x="60"
                    y="78"
                    text-anchor="middle"
                    dominant-baseline="middle"
                    fill="#888"
                    font-size="12"
                    font-weight="500"
                    font-family="system-ui, -apple-system, sans-serif"
                >
                    "Health"
                </text>
            </svg>

            // Status text below gauge
            <div class="gauge-status" style=format!("color: {}", color)>
                {status_text(score)}
            </div>
        </div>
    }
}

/// Returns status text based on score
fn status_text(score: u8) -> &'static str {
    match score {
        0..=20 => "Critical",
        21..=40 => "Poor",
        41..=55 => "Fair",
        56..=69 => "Moderate",
        70..=84 => "Good",
        85..=94 => "Great",
        _ => "Excellent",
    }
}

/// Compact inline health indicator for use in tables/lists
#[component]
pub fn HealthIndicator(score: u8) -> impl IntoView {
    let score = score.min(100);

    let color = match score {
        0..=40 => "#e74c3c",
        41..=69 => "#e67e22",
        _ => "#27ae60",
    };

    view! {
        <span class="health-indicator" style=format!("color: {}", color)>
            <svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;">
                <circle cx="12" cy="12" r="10" fill="none" stroke="#333" stroke-width="2"/>
                <circle
                    cx="12"
                    cy="12"
                    r="10"
                    fill="none"
                    stroke={color}
                    stroke-width="2"
                    stroke-dasharray={format!("{} 63", (score as f64 / 100.0) * 62.83)}
                    transform="rotate(-90 12 12)"
                />
            </svg>
            <span style="font-weight: 600;">{score.to_string()}</span>
        </span>
    }
}
