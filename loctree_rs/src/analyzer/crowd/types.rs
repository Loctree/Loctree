//! Types for crowd detection (functional duplicate clustering)
//!
//! A "crowd" is a cluster of files that serve similar purposes or handle the same domain,
//! often indicating fragmentation or duplication in the codebase.
//!
//! Example crowds:
//! - 3 different hooks all managing "message" state
//! - 2 components both rendering messages with overlapping exports
//! - Multiple utils files with similar string manipulation functions

use serde::{Deserialize, Serialize};

/// A detected "crowd" of files clustering around similar functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crowd {
    /// The pattern/keyword this crowd clusters around (e.g., "message", "chat")
    pub pattern: String,
    /// Files in this crowd, sorted by usage (most used first)
    pub members: Vec<CrowdMember>,
    /// Overall crowd score (0-10, higher = more problematic)
    pub score: f32,
    /// Detected issues
    pub issues: Vec<CrowdIssue>,
}

/// A file that's part of a crowd
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrowdMember {
    /// File path relative to project root
    pub file: String,
    /// Why this file is in the crowd
    pub match_reason: MatchReason,
    /// Number of files that import this one
    pub importer_count: usize,
    /// Similarity scores with other crowd members (file_path, similarity_score)
    pub similarity_scores: Vec<(String, f32)>,
}

/// Why a file matched the crowd pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MatchReason {
    /// File/export name matches pattern
    NameMatch {
        /// The matched string (filename, export name, etc.)
        matched: String,
    },
    /// High import similarity with other crowd members
    ImportSimilarity {
        /// Similarity score (0.0-1.0)
        similarity: f32,
    },
    /// Exports similar types/functions
    ExportSimilarity {
        /// File this one is similar to
        similar_to: String,
    },
}

/// Detected issue in a crowd
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrowdIssue {
    /// Multiple files with very similar names
    NameCollision {
        /// Files with colliding names
        files: Vec<String>,
    },
    /// Some files have much lower usage than others
    UsageAsymmetry {
        /// The primary/most-used file
        primary: String,
        /// Underused files that might be redundant
        underused: Vec<String>,
    },
    /// Files export similar things
    ExportOverlap {
        /// Files with overlapping exports
        files: Vec<String>,
        /// Overlapping export names
        overlap: Vec<String>,
    },
    /// Related functionality is scattered
    Fragmentation {
        /// Categories/themes found scattered across crowd
        categories: Vec<String>,
    },
}
