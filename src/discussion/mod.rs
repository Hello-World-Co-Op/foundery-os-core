//! Discussion Thread Module for Governance Proposal Preparation
//!
//! Story FOS-4.1.2: Discussion Thread Data Model & Storage
//!
//! This module provides the foundational data layer for the proposal
//! preparation system, enabling DAO members to create discussion threads
//! that collect feedback before formal proposal submission.
//!
//! Key Features:
//! - Append-only comment storage with retraction support
//! - Three-stage lifecycle: Brainstorm → Refining → Ready
//! - Quality gates for stage transitions
//! - Contributor invitation system
//! - Discussion hash for proposal verification

pub mod api;
pub mod hash;
pub mod state;
pub mod types;
pub mod validation;

// Re-export types for external use
pub use types::{
    AddCommentArgs, Comment, CreateDiscussionArgs, Discussion, DiscussionFilter, DiscussionStage,
    QualityGateStatus,
};
