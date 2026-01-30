//! Type definitions for the Discussion module
//!
//! Story FOS-4.1.2: Discussion Thread Data Model & Storage

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

// =============================================================================
// ID Type Aliases
// =============================================================================

pub type DiscussionId = u64;
pub type CommentId = u64;

// =============================================================================
// Proposal Category (imported from governance)
// =============================================================================

/// Proposal categories for governance decisions
/// Matches governance canister's ProposalCategory enum
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProposalCategory {
    /// Constitutional changes (bylaws, membership rules)
    Constitutional,
    /// Operational decisions (day-to-day management)
    Operational,
    /// Treasury-related proposals (spending, allocations)
    Treasury,
    /// Software development proposals (features, infrastructure)
    SoftwareDevelopment,
}

// =============================================================================
// Discussion Types
// =============================================================================

/// Discussion stage lifecycle
/// AC-4.1.2.3: Discussions have explicit stages with valid transitions only
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum DiscussionStage {
    /// Initial ideation, scope can change freely
    #[default]
    Brainstorm,
    /// Scope locked, refining arguments and details
    Refining,
    /// Quality gates met, ready for extraction to proposal
    Ready,
}

/// Governance discussion thread
/// AC-4.1.2.1: Discussion threads can be created with title, description, and category
#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct Discussion {
    /// Unique discussion identifier
    pub id: DiscussionId,
    /// Discussion title (1-200 characters)
    pub title: String,
    /// Initial idea description
    pub description: String,
    /// Governance category (matches governance canister)
    pub category: ProposalCategory,
    /// Creator/owner of the discussion
    pub proposer: Principal,
    /// Invited collaborators who can comment and trigger extraction
    pub contributors: Vec<Principal>,
    /// Current discussion stage
    pub stage: DiscussionStage,
    /// Timestamp when discussion was created (nanoseconds)
    pub created_at: u64,
    /// Timestamp when current stage started (nanoseconds)
    pub stage_changed_at: u64,
    /// Total comments count (including retracted)
    pub comment_count: u64,
    /// Unique human commenters count
    pub participant_count: u64,
    /// Soft delete / timeout flag
    pub is_archived: bool,
}

// =============================================================================
// Comment Types
// =============================================================================

/// Author type distinguishes human vs AI comments
/// AC-4.1.2.6: Agent comments are tagged distinctly
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum AuthorType {
    /// Comment from a human DAO member
    Human,
    /// Comment from an AI agent
    Agent {
        /// Identifier for the specific agent
        agent_id: String,
    },
}

impl Default for AuthorType {
    fn default() -> Self {
        AuthorType::Human
    }
}

/// Comment on a discussion thread
/// AC-4.1.2.2: Comments are append-only with immutable timestamps
#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct Comment {
    /// Unique comment identifier
    pub id: CommentId,
    /// Parent discussion ID
    pub discussion_id: DiscussionId,
    /// Author's principal
    pub author: Principal,
    /// Comment content (markdown supported)
    pub content: String,
    /// Whether this is a human or agent comment
    pub author_type: AuthorType,
    /// Timestamp when comment was created (nanoseconds)
    pub created_at: u64,
    /// Whether comment has been retracted (content preserved but marked)
    pub is_retracted: bool,
    /// Timestamp when comment was retracted (if applicable)
    pub retracted_at: Option<u64>,
}

// =============================================================================
// Contributor Invite Types
// =============================================================================

/// Invite status for contributor invitations
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum InviteStatus {
    /// Invitation sent, awaiting response
    #[default]
    Pending,
    /// Invitation accepted
    Accepted,
    /// Invitation declined
    Declined,
}

/// Contributor invitation record
/// AC-4.1.2.4: Proposer can invite Contributors
#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct ContributorInvite {
    /// Discussion ID for this invitation
    pub discussion_id: DiscussionId,
    /// Principal being invited
    pub invitee: Principal,
    /// Principal who sent the invitation
    pub invited_by: Principal,
    /// Timestamp when invitation was sent (nanoseconds)
    pub invited_at: u64,
    /// Current status of the invitation
    pub status: InviteStatus,
}

// =============================================================================
// Quality Gate Types
// =============================================================================

/// Quality gate status for a discussion
/// AC-4.1.2.5: Quality gates prevent Ready status without meeting thresholds
#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct QualityGateStatus {
    /// Whether minimum participants threshold is met
    pub participants_met: bool,
    /// Current number of unique human participants
    pub participants_count: u64,
    /// Whether minimum substantive comments threshold is met
    pub comments_met: bool,
    /// Current number of substantive comments
    pub substantive_comments: u64,
    /// Whether minimum time in Refining stage is met
    pub duration_met: bool,
    /// Hours spent in Refining stage
    pub hours_in_refining: u64,
    /// Whether all quality gates are met
    pub all_met: bool,
}

// =============================================================================
// Request/Response Types
// =============================================================================

/// Arguments for creating a new discussion
#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct CreateDiscussionArgs {
    /// Discussion title (1-200 characters)
    pub title: String,
    /// Initial idea description
    pub description: String,
    /// Governance category
    pub category: ProposalCategory,
}

/// Arguments for adding a comment
#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct AddCommentArgs {
    /// Target discussion ID
    pub discussion_id: DiscussionId,
    /// Comment content
    pub content: String,
    /// Author type (human or agent)
    pub author_type: AuthorType,
}

/// Filter options for listing discussions
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, Default)]
pub struct DiscussionFilter {
    /// Filter by stage
    pub stage: Option<DiscussionStage>,
    /// Filter by category
    pub category: Option<ProposalCategory>,
    /// Filter by proposer
    pub proposer: Option<Principal>,
    /// Include archived discussions
    pub include_archived: Option<bool>,
}

/// Pagination parameters for discussion queries
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, Default)]
pub struct DiscussionPaginationParams {
    /// Number of items to skip
    pub offset: Option<u64>,
    /// Maximum number of items to return
    pub limit: Option<u64>,
}

/// Paginated response for discussions
#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct PaginatedDiscussionResponse {
    /// List of discussions
    pub items: Vec<Discussion>,
    /// Total number of discussions matching the filter
    pub total: u64,
    /// Current offset
    pub offset: u64,
    /// Current limit
    pub limit: u64,
}

// =============================================================================
// Constants
// =============================================================================

/// Minimum unique human participants required for Ready stage
/// AC-4.1.2.5: 3+ participants required
pub const MIN_PARTICIPANTS: u64 = 3;

/// Minimum substantive comments required for Ready stage
/// AC-4.1.2.5: 5+ substantive comments required
pub const MIN_SUBSTANTIVE_COMMENTS: u64 = 5;

/// Minimum time in Refining stage before Ready (48 hours in nanoseconds)
/// AC-4.1.2.5: 48+ hours in Refining required
pub const MIN_REFINING_DURATION_NS: u64 = 48 * 60 * 60 * 1_000_000_000;

/// Minimum characters for a comment to be considered substantive
pub const SUBSTANTIVE_COMMENT_MIN_CHARS: usize = 50;

/// Maximum discussion title length
pub const MAX_TITLE_LEN: usize = 200;

/// Maximum comment content length (10KB)
pub const MAX_COMMENT_LEN: usize = 10 * 1024;

/// Maximum description length (50KB)
pub const MAX_DESCRIPTION_LEN: usize = 50 * 1024;

/// Nanoseconds per hour (for duration calculations)
pub const NS_PER_HOUR: u64 = 60 * 60 * 1_000_000_000;
