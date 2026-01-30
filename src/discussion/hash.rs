//! Hash generation for Discussion verification
//!
//! Story FOS-4.1.2: Discussion Thread Data Model & Storage
//! AC-4.1.2.7: Discussion state hash can be generated for proposal verification

use sha2::{Digest, Sha256};

use super::types::*;

/// Generate a deterministic hash for a discussion and its comments
/// This hash is used to link a submitted proposal to its discussion state.
///
/// The hash includes:
/// - Discussion metadata (id, title, description, category, proposer)
/// - All comments (including retracted, for audit trail)
///
/// # Returns
/// Hex-encoded SHA-256 hash string
pub fn generate_discussion_hash(discussion: &Discussion, comments: &[Comment]) -> String {
    let mut hasher = Sha256::new();

    // Include discussion metadata
    hasher.update(discussion.id.to_le_bytes());
    hasher.update(discussion.title.as_bytes());
    hasher.update(discussion.description.as_bytes());
    hasher.update(format!("{:?}", discussion.category).as_bytes());
    hasher.update(discussion.proposer.as_slice());

    // Include all comments (including retracted - for audit trail)
    for comment in comments {
        hasher.update(comment.id.to_le_bytes());
        hasher.update(comment.author.as_slice());
        hasher.update(comment.content.as_bytes());
        hasher.update(comment.created_at.to_le_bytes());
        hasher.update(if comment.is_retracted { [1u8] } else { [0u8] });
    }

    hex::encode(hasher.finalize())
}


#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    #[test]
    fn test_hash_is_deterministic() {
        let discussion = Discussion {
            id: 1,
            title: "Test Discussion".to_string(),
            description: "A test description".to_string(),
            category: ProposalCategory::Operational,
            proposer: Principal::anonymous(),
            contributors: vec![],
            stage: DiscussionStage::Brainstorm,
            created_at: 1000000,
            stage_changed_at: 1000000,
            comment_count: 0,
            participant_count: 0,
            is_archived: false,
        };

        let hash1 = generate_discussion_hash(&discussion, &[]);
        let hash2 = generate_discussion_hash(&discussion, &[]);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_changes_with_comment() {
        let discussion = Discussion {
            id: 1,
            title: "Test Discussion".to_string(),
            description: "A test description".to_string(),
            category: ProposalCategory::Operational,
            proposer: Principal::anonymous(),
            contributors: vec![],
            stage: DiscussionStage::Brainstorm,
            created_at: 1000000,
            stage_changed_at: 1000000,
            comment_count: 0,
            participant_count: 0,
            is_archived: false,
        };

        let hash_without_comment = generate_discussion_hash(&discussion, &[]);

        let comment = Comment {
            id: 1,
            discussion_id: 1,
            author: Principal::anonymous(),
            content: "A comment".to_string(),
            author_type: AuthorType::Human,
            created_at: 2000000,
            is_retracted: false,
            retracted_at: None,
        };

        let hash_with_comment = generate_discussion_hash(&discussion, &[comment]);

        assert_ne!(hash_without_comment, hash_with_comment);
    }
}
