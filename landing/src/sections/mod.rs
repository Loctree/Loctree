// Landing page sections
// Created by M&K (c)2025 The LibraxisAI Team

/// Version string used across the landing page (single source of truth)
pub const VERSION: &str = "v0.5.6";

mod nav;
mod hero;
mod features;
mod slice_demo;
mod stack_detection;
mod real_world;
mod for_agents;
mod cli_reference;
mod install;
mod footer;
mod easter_eggs;

pub use nav::Nav;
pub use hero::Hero;
pub use features::Features;
pub use slice_demo::SliceDemo;
pub use stack_detection::StackDetection;
pub use real_world::RealWorldResults;
pub use for_agents::ForAgents;
pub use cli_reference::CliReference;
pub use install::InstallSection;
pub use footer::Footer;
pub use easter_eggs::EasterEggs;
