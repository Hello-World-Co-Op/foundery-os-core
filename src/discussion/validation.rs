//! Validation logic for Discussion module
//!
//! Story FOS-4.1.2: Discussion Thread Data Model & Storage

use super::state::DiscussionState;
use super::types::*;

/// Validate discussion creation arguments
/// AC-4.1.2.1: Discussions can be created with title, description, and category
pub fn validate_create_discussion(args: &CreateDiscussionArgs) -> Result<(), String> {
    // Title validation
    if args.title.is_empty() {
        return Err("Title cannot be empty".to_string());
    }
    if args.title.len() > MAX_TITLE_LEN {
        return Err(format!(
            "Title too long (max {} characters)",
            MAX_TITLE_LEN
        ));
    }

    // Description validation
    if args.description.is_empty() {
        return Err("Description cannot be empty".to_string());
    }
    if args.description.len() > MAX_DESCRIPTION_LEN {
        return Err(format!(
            "Description too long (max {} bytes)",
            MAX_DESCRIPTION_LEN
        ));
    }

    Ok(())
}

/// Validate comment content
/// AC-4.1.2.2: Comments are append-only
pub fn validate_comment(args: &AddCommentArgs) -> Result<(), String> {
    if args.content.is_empty() {
        return Err("Comment content cannot be empty".to_string());
    }
    if args.content.len() > MAX_COMMENT_LEN {
        return Err(format!(
            "Comment too long (max {} bytes)",
            MAX_COMMENT_LEN
        ));
    }
    Ok(())
}

/// Validate stage transition
/// AC-4.1.2.3: Discussions have explicit stages with valid transitions only
pub fn validate_stage_transition(
    current: &DiscussionStage,
    target: &DiscussionStage,
) -> Result<(), String> {
    match (current, target) {
        // Brainstorm can only go to Refining
        (DiscussionStage::Brainstorm, DiscussionStage::Refining) => Ok(()),
        // Refining can only go to Ready
        (DiscussionStage::Refining, DiscussionStage::Ready) => Ok(()),
        // Ready is terminal - cannot transition
        (DiscussionStage::Ready, _) => {
            Err("Cannot transition from Ready stage".to_string())
        }
        // Same stage is not a valid transition
        (a, b) if a == b => Err(format!("Already in {:?} stage", a)),
        // All other transitions are invalid
        (a, b) => Err(format!("Invalid transition: {:?} → {:?}", a, b)),
    }
}

/// Check quality gates for Refining → Ready transition
/// AC-4.1.2.5: Quality gates prevent Ready without meeting thresholds
pub fn check_quality_gates(state: &DiscussionState, discussion_id: DiscussionId, now: u64) -> QualityGateStatus {
    let discussion = match state.get_discussion(discussion_id) {
        Some(d) => d,
        None => {
            return QualityGateStatus {
                participants_met: false,
                participants_count: 0,
                comments_met: false,
                substantive_comments: 0,
                duration_met: false,
                hours_in_refining: 0,
                all_met: false,
            };
        }
    };

    // Count unique human participants
    let participants_count = state.get_participant_count(discussion_id);
    let participants_met = participants_count >= MIN_PARTICIPANTS;

    // Count substantive comments (human only, 50+ chars, not retracted)
    let substantive_comments = state.count_substantive_comments(discussion_id);
    let comments_met = substantive_comments >= MIN_SUBSTANTIVE_COMMENTS;

    // Check time in Refining stage
    let (duration_met, hours_in_refining) = if discussion.stage == DiscussionStage::Refining {
        let time_in_stage_ns = now.saturating_sub(discussion.stage_changed_at);
        let hours = time_in_stage_ns / NS_PER_HOUR;
        (time_in_stage_ns >= MIN_REFINING_DURATION_NS, hours)
    } else if discussion.stage == DiscussionStage::Ready {
        // Already passed Ready, so duration was met
        (true, 48)
    } else {
        // Not yet in Refining
        (false, 0)
    };

    let all_met = participants_met && comments_met && duration_met;

    QualityGateStatus {
        participants_met,
        participants_count,
        comments_met,
        substantive_comments,
        duration_met,
        hours_in_refining,
        all_met,
    }
}

