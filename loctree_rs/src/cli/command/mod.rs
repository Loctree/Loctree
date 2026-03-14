//! CLI command definitions and help text.
//!
//! This module provides the core Command enum and associated types for the
//! `loct <command>` interface. It is modularized for maintainability:
//!
//! - `global`: GlobalOptions struct shared across all commands
//! - `options`: Per-command option structs
//! - `types`: Command enum definition
//! - `help`: Help text generation (impl on Command)
//! - `help_texts`: Static help text constants
//! - `parsed`: ParsedCommand result type
//!
//! VibeCrafted with AI Agents (c)2026 Loctree Team

mod global;
mod help;
mod help_texts;
pub mod options;
mod parsed;
mod types;

// Re-export the main types at the module level
pub use global::GlobalOptions;
pub use options::{
    AuditOptions, AutoOptions, CacheAction, CacheOptions, CommandsOptions, CoverageOptions,
    CrowdOptions, CyclesOptions, DeadOptions, DiffOptions, DistOptions, DoctorOptions,
    EventsOptions, FindOptions, FindingsOptions, FocusOptions, HealthOptions, HelpOptions,
    HotspotsOptions, ImpactCommandOptions, InfoOptions, InsightsOptions, JqQueryOptions,
    LayoutmapOptions, LintOptions, ManifestsOptions, MemexOptions, PipelinesOptions, PlanOptions,
    QueryKind, QueryOptions, ReportOptions, RoutesOptions, ScanOptions, SliceOptions, SniffOptions,
    SuppressOptions, TagmapOptions, TraceOptions, TreeOptions, TwinsOptions, ZombieOptions,
};
pub use parsed::ParsedCommand;
pub use types::Command;
