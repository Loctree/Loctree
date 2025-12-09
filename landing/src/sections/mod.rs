// Landing page sections
// Developed with 💀 by The Loctree Team (c)2025

/// Version string used across the landing page (single source of truth)
pub const VERSION: &str = "v0.6.2-dev";

mod blog;
mod cli_reference;
mod easter_eggs;
mod features;
mod footer;
mod for_agents;
mod hero;
mod install;
mod manual;
mod nav;
mod real_world;
mod slice_demo;
mod stack_detection;

pub use blog::Blog;
pub use cli_reference::CliReference;
pub use easter_eggs::EasterEggs;
pub use features::Features;
pub use footer::Footer;
pub use for_agents::ForAgents;
pub use hero::Hero;
pub use install::InstallSection;
pub use manual::Manual;
pub use nav::Nav;
pub use real_world::RealWorldResults;
pub use slice_demo::SliceDemo;
pub use stack_detection::StackDetection;
