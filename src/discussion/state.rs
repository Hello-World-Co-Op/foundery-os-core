//! State management for the Discussion module
//!
//! Story FOS-4.1.2: Discussion Thread Data Model & Storage

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use super::types::*;

/// State structure for discussion threads
#[derive(Default)]
pub struct DiscussionState {
    /// All discussions by ID
    pub discussions: BTreeMap<DiscussionId, Discussion>,
    /// All comments by ID
    pub comments: BTreeMap<CommentId, Comment>,
    /// Discussion ID -> Comment IDs (ordered by creation)
    pub discussion_comments: BTreeMap<DiscussionId, Vec<CommentId>>,
    /// Discussion ID + Invitee Principal -> Invite
    pub invites: BTreeMap<(DiscussionId, Principal), ContributorInvite>,
    /// Discussion ID -> Set of unique human participant principals
    pub discussion_participants: BTreeMap<DiscussionId, BTreeSet<Principal>>,
    /// User principal -> Discussion IDs they created
    pub user_discussions: BTreeMap<Principal, Vec<DiscussionId>>,
    /// Next discussion ID counter
    pub next_discussion_id: DiscussionId,
    /// Next comment ID counter
    pub next_comment_id: CommentId,
}

impl DiscussionState {
    /// Create a new empty state
    pub fn new() -> Self {
        Self {
            discussions: BTreeMap::new(),
            comments: BTreeMap::new(),
            discussion_comments: BTreeMap::new(),
            invites: BTreeMap::new(),
            discussion_participants: BTreeMap::new(),
            user_discussions: BTreeMap::new(),
            next_discussion_id: 1,
            next_comment_id: 1,
        }
    }

    /// Get next discussion ID and increment counter
    pub fn next_discussion_id(&mut self) -> DiscussionId {
        let id = self.next_discussion_id;
        self.next_discussion_id += 1;
        id
    }

    /// Get next comment ID and increment counter
    pub fn next_comment_id(&mut self) -> CommentId {
        let id = self.next_comment_id;
        self.next_comment_id += 1;
        id
    }

    /// Get a discussion by ID
    pub fn get_discussion(&self, id: DiscussionId) -> Option<&Discussion> {
        self.discussions.get(&id)
    }

    /// Get a mutable discussion by ID
    pub fn get_discussion_mut(&mut self, id: DiscussionId) -> Option<&mut Discussion> {
        self.discussions.get_mut(&id)
    }

    /// Get a comment by ID
    pub fn get_comment(&self, id: CommentId) -> Option<&Comment> {
        self.comments.get(&id)
    }

    /// Get a mutable comment by ID
    pub fn get_comment_mut(&mut self, id: CommentId) -> Option<&mut Comment> {
        self.comments.get_mut(&id)
    }

    /// Get comments for a discussion with pagination
    pub fn get_discussion_comments(
        &self,
        discussion_id: DiscussionId,
        offset: u64,
        limit: u64,
    ) -> Vec<Comment> {
        self.discussion_comments
            .get(&discussion_id)
            .map(|comment_ids| {
                comment_ids
                    .iter()
                    .skip(offset as usize)
                    .take(limit as usize)
                    .filter_map(|id| self.comments.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all comments for a discussion (for hash generation)
    pub fn get_all_discussion_comments(&self, discussion_id: DiscussionId) -> Vec<Comment> {
        self.discussion_comments
            .get(&discussion_id)
            .map(|comment_ids| {
                comment_ids
                    .iter()
                    .filter_map(|id| self.comments.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get an invite by discussion ID and invitee principal
    pub fn get_invite(
        &self,
        discussion_id: DiscussionId,
        invitee: &Principal,
    ) -> Option<&ContributorInvite> {
        self.invites.get(&(discussion_id, *invitee))
    }

    /// Get a mutable invite
    pub fn get_invite_mut(
        &mut self,
        discussion_id: DiscussionId,
        invitee: &Principal,
    ) -> Option<&mut ContributorInvite> {
        self.invites.get_mut(&(discussion_id, *invitee))
    }

    /// Check if a principal is the proposer or a contributor
    pub fn is_proposer_or_contributor(&self, discussion_id: DiscussionId, principal: &Principal) -> bool {
        if let Some(discussion) = self.discussions.get(&discussion_id) {
            discussion.proposer == *principal || discussion.contributors.contains(principal)
        } else {
            false
        }
    }

    /// Check if a principal can comment (proposer, contributor, or has accepted invite)
    pub fn can_comment(&self, discussion_id: DiscussionId, principal: &Principal) -> bool {
        if let Some(discussion) = self.discussions.get(&discussion_id) {
            // Proposer can always comment
            if discussion.proposer == *principal {
                return true;
            }
            // Contributors can comment
            if discussion.contributors.contains(principal) {
                return true;
            }
            // Anyone can comment in Brainstorm stage
            if discussion.stage == DiscussionStage::Brainstorm {
                return true;
            }
            // In Refining/Ready, only proposer and contributors
            false
        } else {
            false
        }
    }

    /// Add a participant to the discussion's participant set
    pub fn add_participant(&mut self, discussion_id: DiscussionId, principal: Principal) {
        self.discussion_participants
            .entry(discussion_id)
            .or_default()
            .insert(principal);
    }

    /// Get participant count for a discussion
    pub fn get_participant_count(&self, discussion_id: DiscussionId) -> u64 {
        self.discussion_participants
            .get(&discussion_id)
            .map(|set| set.len() as u64)
            .unwrap_or(0)
    }

    /// Count substantive human comments for a discussion
    /// AC-4.1.2.5: Substantive = 50+ chars, human only
    /// AC-4.1.2.6: Agent comments excluded from quality gate counts
    pub fn count_substantive_comments(&self, discussion_id: DiscussionId) -> u64 {
        self.discussion_comments
            .get(&discussion_id)
            .map(|comment_ids| {
                comment_ids
                    .iter()
                    .filter_map(|id| self.comments.get(id))
                    .filter(|c| {
                        !c.is_retracted
                            && c.author_type == AuthorType::Human
                            && c.content.chars().count() >= SUBSTANTIVE_COMMENT_MIN_CHARS
                    })
                    .count() as u64
            })
            .unwrap_or(0)
    }

    /// List discussions with optional filter
    pub fn list_discussions(&self, filter: Option<DiscussionFilter>) -> Vec<Discussion> {
        let mut discussions: Vec<Discussion> = self.discussions.values().cloned().collect();

        if let Some(ref f) = filter {
            if let Some(ref stage) = f.stage {
                discussions.retain(|d| &d.stage == stage);
            }
            if let Some(ref category) = f.category {
                discussions.retain(|d| &d.category == category);
            }
            if let Some(ref proposer) = f.proposer {
                discussions.retain(|d| &d.proposer == proposer);
            }
            if !f.include_archived.unwrap_or(false) {
                discussions.retain(|d| !d.is_archived);
            }
        } else {
            // Default: exclude archived
            discussions.retain(|d| !d.is_archived);
        }

        discussions
    }
}

thread_local! {
    pub static DISCUSSION_STATE: RefCell<DiscussionState> = RefCell::new(DiscussionState::new());
}

/// Helper function to access discussion state
pub fn with_discussion_state<F, R>(f: F) -> R
where
    F: FnOnce(&DiscussionState) -> R,
{
    DISCUSSION_STATE.with(|state| f(&state.borrow()))
}

/// Helper function to mutably access discussion state
pub fn with_discussion_state_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut DiscussionState) -> R,
{
    DISCUSSION_STATE.with(|state| f(&mut state.borrow_mut()))
}

// =============================================================================
// Stable Storage Types
// =============================================================================

/// Serializable state for canister upgrades
#[derive(CandidType, Deserialize, Serialize, Clone, Default)]
pub struct StableDiscussionState {
    pub discussions: Vec<(DiscussionId, Discussion)>,
    pub comments: Vec<(CommentId, Comment)>,
    pub discussion_comments: Vec<(DiscussionId, Vec<CommentId>)>,
    pub invites: Vec<((DiscussionId, Principal), ContributorInvite)>,
    pub discussion_participants: Vec<(DiscussionId, Vec<Principal>)>,
    pub user_discussions: Vec<(Principal, Vec<DiscussionId>)>,
    pub next_discussion_id: DiscussionId,
    pub next_comment_id: CommentId,
}

impl From<&DiscussionState> for StableDiscussionState {
    fn from(state: &DiscussionState) -> Self {
        StableDiscussionState {
            discussions: state.discussions.iter().map(|(k, v)| (*k, v.clone())).collect(),
            comments: state.comments.iter().map(|(k, v)| (*k, v.clone())).collect(),
            discussion_comments: state
                .discussion_comments
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            invites: state.invites.iter().map(|(k, v)| (*k, v.clone())).collect(),
            discussion_participants: state
                .discussion_participants
                .iter()
                .map(|(k, v)| (*k, v.iter().cloned().collect()))
                .collect(),
            user_discussions: state
                .user_discussions
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            next_discussion_id: state.next_discussion_id,
            next_comment_id: state.next_comment_id,
        }
    }
}

impl From<StableDiscussionState> for DiscussionState {
    fn from(stable: StableDiscussionState) -> Self {
        DiscussionState {
            discussions: stable.discussions.into_iter().collect(),
            comments: stable.comments.into_iter().collect(),
            discussion_comments: stable.discussion_comments.into_iter().collect(),
            invites: stable.invites.into_iter().collect(),
            discussion_participants: stable
                .discussion_participants
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().collect()))
                .collect(),
            user_discussions: stable.user_discussions.into_iter().collect(),
            next_discussion_id: stable.next_discussion_id,
            next_comment_id: stable.next_comment_id,
        }
    }
}
