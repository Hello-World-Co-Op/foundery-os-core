//! API functions for Discussion module
//!
//! Story FOS-4.1.2: Discussion Thread Data Model & Storage
//!
//! This module provides the core business logic for discussion operations.
//! These functions are called by the canister's endpoint handlers in lib.rs.

use candid::Principal;

use super::hash::generate_discussion_hash;
use super::state::{with_discussion_state, with_discussion_state_mut};
use super::types::*;
use super::validation::{
    check_quality_gates, validate_comment, validate_create_discussion, validate_stage_transition,
};

// =============================================================================
// Discussion Operations
// =============================================================================

/// Create a new discussion thread
/// AC-4.1.2.1: Discussion threads can be created with title, description, and category
pub fn create_discussion(
    caller: Principal,
    args: CreateDiscussionArgs,
    now: u64,
) -> Result<DiscussionId, String> {
    // Validate input
    validate_create_discussion(&args)?;

    with_discussion_state_mut(|state| {
        let id = state.next_discussion_id();

        let discussion = Discussion {
            id,
            title: args.title,
            description: args.description,
            category: args.category,
            proposer: caller,
            contributors: vec![],
            stage: DiscussionStage::Brainstorm,
            created_at: now,
            stage_changed_at: now,
            comment_count: 0,
            participant_count: 1, // Proposer counts as first participant
            is_archived: false,
        };

        state.discussions.insert(id, discussion);
        state.discussion_comments.insert(id, vec![]);
        state.discussion_participants.entry(id).or_default().insert(caller);
        state.user_discussions.entry(caller).or_default().push(id);

        Ok(id)
    })
}

/// Get a discussion by ID
pub fn get_discussion(discussion_id: DiscussionId) -> Option<Discussion> {
    with_discussion_state(|state| state.get_discussion(discussion_id).cloned())
}

/// List discussions with optional filter and pagination
pub fn list_discussions(
    filter: Option<DiscussionFilter>,
    pagination: Option<DiscussionPaginationParams>,
) -> PaginatedDiscussionResponse {
    with_discussion_state(|state| state.list_discussions(filter, pagination))
}

/// Archive a discussion (soft delete)
pub fn archive_discussion(
    caller: Principal,
    discussion_id: DiscussionId,
    now: u64,
) -> Result<(), String> {
    with_discussion_state_mut(|state| {
        let discussion = state
            .get_discussion_mut(discussion_id)
            .ok_or_else(|| "Discussion not found".to_string())?;

        // Only proposer can archive
        if discussion.proposer != caller {
            return Err("Only the proposer can archive this discussion".to_string());
        }

        if discussion.is_archived {
            return Err("Discussion is already archived".to_string());
        }

        discussion.is_archived = true;
        let _ = now; // Could track archived_at if needed

        Ok(())
    })
}

// =============================================================================
// Comment Operations
// =============================================================================

/// Add a comment to a discussion
/// AC-4.1.2.2: Comments are append-only with immutable timestamps
/// AC-4.1.2.6: Agent comments are tagged distinctly
pub fn add_comment(
    caller: Principal,
    args: AddCommentArgs,
    now: u64,
) -> Result<CommentId, String> {
    // Validate input
    validate_comment(&args)?;

    with_discussion_state_mut(|state| {
        // Verify discussion exists and is not archived
        let discussion = state
            .get_discussion(args.discussion_id)
            .ok_or_else(|| "Discussion not found".to_string())?;

        if discussion.is_archived {
            return Err("Cannot comment on an archived discussion".to_string());
        }

        // Check if caller can comment
        if !state.can_comment(args.discussion_id, &caller) {
            return Err(
                "You are not authorized to comment on this discussion in its current stage"
                    .to_string(),
            );
        }

        let comment_id = state.next_comment_id();

        let comment = Comment {
            id: comment_id,
            discussion_id: args.discussion_id,
            author: caller,
            content: args.content,
            author_type: args.author_type.clone(),
            created_at: now,
            is_retracted: false,
            retracted_at: None,
        };

        state.comments.insert(comment_id, comment);
        state
            .discussion_comments
            .entry(args.discussion_id)
            .or_default()
            .push(comment_id);

        // Track human participants for quality gates
        // AC-4.1.2.6: Agent comments excluded from participant count
        let new_participant_count = if args.author_type == AuthorType::Human {
            state
                .discussion_participants
                .entry(args.discussion_id)
                .or_default()
                .insert(caller);
            Some(state.get_participant_count(args.discussion_id))
        } else {
            None
        };

        // Update discussion comment count
        if let Some(discussion) = state.get_discussion_mut(args.discussion_id) {
            discussion.comment_count += 1;

            if let Some(count) = new_participant_count {
                discussion.participant_count = count;
            }
        }

        Ok(comment_id)
    })
}

/// Get comments for a discussion with pagination
pub fn get_comments(
    discussion_id: DiscussionId,
    offset: u64,
    limit: u64,
) -> Vec<Comment> {
    with_discussion_state(|state| state.get_discussion_comments(discussion_id, offset, limit))
}

/// Retract a comment (mark as retracted, don't delete)
/// AC-4.1.2.2: Comments are append-only (retraction only, not deletion)
pub fn retract_comment(
    caller: Principal,
    comment_id: CommentId,
    now: u64,
) -> Result<(), String> {
    with_discussion_state_mut(|state| {
        let comment = state
            .get_comment_mut(comment_id)
            .ok_or_else(|| "Comment not found".to_string())?;

        // Only the author can retract their own comment
        if comment.author != caller {
            return Err("Only the comment author can retract this comment".to_string());
        }

        if comment.is_retracted {
            return Err("Comment is already retracted".to_string());
        }

        comment.is_retracted = true;
        comment.retracted_at = Some(now);

        Ok(())
    })
}

// =============================================================================
// Stage Transition Operations
// =============================================================================

/// Advance discussion to the next stage
/// AC-4.1.2.3: Discussions have explicit stages with valid transitions only
/// AC-4.1.2.5: Quality gates prevent Ready status without meeting thresholds
pub fn advance_stage(
    caller: Principal,
    discussion_id: DiscussionId,
    now: u64,
) -> Result<DiscussionStage, String> {
    with_discussion_state_mut(|state| {
        let discussion = state
            .get_discussion(discussion_id)
            .ok_or_else(|| "Discussion not found".to_string())?;

        // Only proposer or contributor can advance stage
        if !state.is_proposer_or_contributor(discussion_id, &caller) {
            return Err("Only the proposer or a contributor can advance the stage".to_string());
        }

        if discussion.is_archived {
            return Err("Cannot advance stage of an archived discussion".to_string());
        }

        let current_stage = discussion.stage.clone();
        let target_stage = match current_stage {
            DiscussionStage::Brainstorm => DiscussionStage::Refining,
            DiscussionStage::Refining => DiscussionStage::Ready,
            DiscussionStage::Ready => {
                return Err("Discussion is already in Ready stage".to_string());
            }
        };

        // Validate the transition
        validate_stage_transition(&current_stage, &target_stage)?;

        // For Refining → Ready, check quality gates
        if current_stage == DiscussionStage::Refining && target_stage == DiscussionStage::Ready {
            let gate_status = check_quality_gates(state, discussion_id, now);
            if !gate_status.all_met {
                let mut missing = vec![];
                if !gate_status.participants_met {
                    missing.push(format!(
                        "participants ({}/{})",
                        gate_status.participants_count, MIN_PARTICIPANTS
                    ));
                }
                if !gate_status.comments_met {
                    missing.push(format!(
                        "substantive comments ({}/{})",
                        gate_status.substantive_comments, MIN_SUBSTANTIVE_COMMENTS
                    ));
                }
                if !gate_status.duration_met {
                    missing.push(format!(
                        "time in Refining ({}h/48h)",
                        gate_status.hours_in_refining
                    ));
                }
                return Err(format!(
                    "Quality gates not met: {}",
                    missing.join(", ")
                ));
            }
        }

        // Perform the transition
        let discussion = state
            .get_discussion_mut(discussion_id)
            .ok_or_else(|| "Discussion not found".to_string())?;

        discussion.stage = target_stage.clone();
        discussion.stage_changed_at = now;

        Ok(target_stage)
    })
}

// =============================================================================
// Contributor Operations
// =============================================================================

/// Invite a contributor to the discussion
/// AC-4.1.2.4: Proposer can invite Contributors
pub fn invite_contributor(
    caller: Principal,
    discussion_id: DiscussionId,
    invitee: Principal,
    now: u64,
) -> Result<(), String> {
    with_discussion_state_mut(|state| {
        let discussion = state
            .get_discussion(discussion_id)
            .ok_or_else(|| "Discussion not found".to_string())?;

        // Only proposer or existing contributor can invite
        if !state.is_proposer_or_contributor(discussion_id, &caller) {
            return Err("Only the proposer or a contributor can invite others".to_string());
        }

        if discussion.is_archived {
            return Err("Cannot invite to an archived discussion".to_string());
        }

        // Check if already invited or already a contributor
        if discussion.proposer == invitee {
            return Err("Cannot invite the proposer".to_string());
        }
        if discussion.contributors.contains(&invitee) {
            return Err("User is already a contributor".to_string());
        }
        if state.get_invite(discussion_id, &invitee).is_some() {
            return Err("User already has a pending invitation".to_string());
        }

        let invite = ContributorInvite {
            discussion_id,
            invitee,
            invited_by: caller,
            invited_at: now,
            status: InviteStatus::Pending,
        };

        state.invites.insert((discussion_id, invitee), invite);

        Ok(())
    })
}

/// Respond to a contributor invitation
/// AC-4.1.2.4: Contributors can comment and trigger extraction
pub fn respond_to_invite(
    caller: Principal,
    discussion_id: DiscussionId,
    accept: bool,
    now: u64,
) -> Result<(), String> {
    with_discussion_state_mut(|state| {
        let invite = state
            .get_invite_mut(discussion_id, &caller)
            .ok_or_else(|| "No invitation found for you".to_string())?;

        if invite.status != InviteStatus::Pending {
            return Err("Invitation has already been responded to".to_string());
        }

        let _ = now; // Could track response time if needed

        if accept {
            invite.status = InviteStatus::Accepted;

            // Add to contributors list
            if let Some(discussion) = state.get_discussion_mut(discussion_id) {
                if !discussion.contributors.contains(&caller) {
                    discussion.contributors.push(caller);
                }
            }
        } else {
            invite.status = InviteStatus::Declined;
        }

        Ok(())
    })
}

// =============================================================================
// Query Operations
// =============================================================================

/// Get quality gate status for a discussion
/// AC-4.1.2.5: Quality gates visible to users
pub fn get_quality_gate_status(discussion_id: DiscussionId, now: u64) -> Option<QualityGateStatus> {
    with_discussion_state(|state| {
        if state.get_discussion(discussion_id).is_some() {
            Some(check_quality_gates(state, discussion_id, now))
        } else {
            None
        }
    })
}

/// Get discussion hash for proposal verification
/// AC-4.1.2.7: Discussion state hash for proposal verification
pub fn get_discussion_hash(discussion_id: DiscussionId) -> Option<String> {
    with_discussion_state(|state| {
        let discussion = state.get_discussion(discussion_id)?;
        let comments = state.get_all_discussion_comments(discussion_id);
        Some(generate_discussion_hash(discussion, &comments))
    })
}
