mod discussion;
mod state;
mod types;

use candid::Principal;
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update};

pub use state::{State, StableState, STATE};
pub use types::*;

// =============================================================================
// Session Validation Types (for inter-canister auth with auth-service)
// =============================================================================

/// Response type from auth-service.validate_access_token
/// Matches Candid: variant { Ok: text; Err: text }
#[derive(candid::CandidType, candid::Deserialize)]
enum SessionValidationResult {
    Ok(String),
    Err(String),
}

// =============================================================================
// Canister Lifecycle
// =============================================================================

#[init]
fn init(controllers: Option<Vec<Principal>>) {
    let effective_controllers = controllers.unwrap_or_else(|| vec![ic_cdk::caller()]);
    STATE.with(|state| {
        let mut s = state.borrow_mut();
        s.controllers = effective_controllers;
    });

    ic_cdk::println!("===========================================");
    ic_cdk::println!("FounderyOS Core Initialization Complete");
    ic_cdk::println!("===========================================");
}

#[pre_upgrade]
fn pre_upgrade() {
    STATE.with(|state| {
        let s = state.borrow();
        let stable: StableState = (&*s).into();
        ic_cdk::storage::stable_save((stable,)).expect("Failed to save state to stable storage");
    });
}

#[post_upgrade]
fn post_upgrade() {
    let restored_state = match ic_cdk::storage::stable_restore::<(StableState,)>() {
        Ok((saved_state,)) => {
            ic_cdk::println!("Restored state from stable storage");
            State::from(saved_state)
        }
        Err(e) => {
            ic_cdk::println!("No previous state found ({}), using default state", e);
            State::new()
        }
    };

    STATE.with(|state| {
        *state.borrow_mut() = restored_state;
    });

    ic_cdk::println!("===========================================");
    ic_cdk::println!("FounderyOS Core Upgrade Complete");
    ic_cdk::println!("===========================================");
}

// =============================================================================
// Configuration
// =============================================================================

#[update]
async fn set_auth_service(canister_id: Principal) -> Result<(), String> {
    require_controller().await?;

    STATE.with(|state| {
        state.borrow_mut().auth_service = Some(canister_id);
    });

    ic_cdk::println!("Auth service set to: {}", canister_id);
    Ok(())
}

#[query]
fn get_auth_service() -> Option<Principal> {
    STATE.with(|state| state.borrow().auth_service)
}

#[query]
fn get_controllers() -> Vec<Principal> {
    STATE.with(|state| state.borrow().get_controllers())
}

// =============================================================================
// Access Control
// =============================================================================

async fn require_controller() -> Result<(), String> {
    let caller = ic_cdk::caller();

    let is_authorized = STATE.with(|state| state.borrow().is_controller(&caller));

    if !is_authorized {
        use ic_cdk::api::management_canister::main::{canister_status, CanisterIdRecord};

        let status = canister_status(CanisterIdRecord {
            canister_id: ic_cdk::id(),
        })
        .await
        .map_err(|(code, msg)| format!("Failed to query canister status: {:?}: {}", code, msg))?
        .0;

        if !status.settings.controllers.contains(&caller) {
            return Err("Unauthorized: Only controllers can perform this action".to_string());
        }

        STATE.with(|state| {
            state.borrow_mut().controllers = status.settings.controllers;
        });
    }

    Ok(())
}

fn require_authenticated() -> Result<Principal, String> {
    let caller = ic_cdk::caller();
    if caller == Principal::anonymous() {
        return Err("Authentication required".to_string());
    }
    Ok(caller)
}

/// Validate an access token via inter-canister call to auth-service.
/// Returns the user_id on success, or an error message on failure.
/// This is an async function that makes a cross-canister call.
async fn validate_session_token(access_token: &str) -> Result<String, String> {
    // Get auth-service canister ID from state
    let auth_service = STATE.with(|s| s.borrow().auth_service);

    let auth_service = match auth_service {
        Some(id) => id,
        None => return Err("Auth service not configured. Call set_auth_service first.".to_string()),
    };

    // Call auth-service to validate the access token
    let result: Result<(SessionValidationResult,), _> =
        ic_cdk::call(auth_service, "validate_access_token", (access_token.to_string(),)).await;

    match result {
        Ok((SessionValidationResult::Ok(user_id),)) => Ok(user_id),
        Ok((SessionValidationResult::Err(err),)) => Err(format!("Session validation failed: {}", err)),
        Err((code, msg)) => Err(format!("Failed to call auth service: {:?} - {}", code, msg)),
    }
}

/// Authenticate using an access token and return the user_id for ownership checks.
/// This enables session-based authentication where the frontend passes the access_token.
async fn require_authenticated_with_token(access_token: &str) -> Result<String, String> {
    if access_token.is_empty() {
        return Err("Access token is required".to_string());
    }
    validate_session_token(access_token).await
}

// =============================================================================
// Capture API
// =============================================================================

#[update]
fn create_capture(request: CreateCaptureRequest) -> Result<Capture, String> {
    let owner = require_authenticated()?;

    let capture = STATE.with(|state| {
        state.borrow_mut().create_capture(owner, request)
    });

    ic_cdk::println!("Created capture {} for {}", capture.id, owner);
    Ok(capture)
}

#[query]
fn get_capture(id: CaptureId) -> Option<Capture> {
    STATE.with(|state| state.borrow().get_capture(id).cloned())
}

#[update]
fn update_capture(request: UpdateCaptureRequest) -> Result<Capture, String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        let s = state.borrow();
        let capture = s.get_capture(request.id)
            .ok_or_else(|| "Capture not found".to_string())?;

        if capture.owner != caller {
            return Err("Not authorized to update this capture".to_string());
        }
        drop(s);

        state.borrow_mut().update_capture(request)
            .ok_or_else(|| "Failed to update capture".to_string())
    })
}

#[update]
fn delete_capture(id: CaptureId) -> Result<Capture, String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        {
            let s = state.borrow();
            let capture = s.get_capture(id)
                .ok_or_else(|| "Capture not found".to_string())?;

            if capture.owner != caller {
                return Err("Not authorized to delete this capture".to_string());
            }
        }

        state.borrow_mut().delete_capture(id)
            .ok_or_else(|| "Failed to delete capture".to_string())
    })
}

#[query]
fn get_my_captures(
    filter: Option<CaptureFilter>,
    pagination: Option<PaginationParams>,
) -> PaginatedResponse<Capture> {
    let caller = ic_cdk::caller();
    if caller == Principal::anonymous() {
        return PaginatedResponse {
            items: vec![],
            total: 0,
            offset: 0,
            limit: 50,
        };
    }

    STATE.with(|state| {
        state.borrow().get_user_captures(
            caller,
            filter,
            pagination.unwrap_or_default(),
        )
    })
}

// =============================================================================
// Sprint API
// =============================================================================

#[update]
fn create_sprint(request: CreateSprintRequest) -> Result<Sprint, String> {
    let owner = require_authenticated()?;

    let sprint = STATE.with(|state| {
        state.borrow_mut().create_sprint(owner, request)
    });

    ic_cdk::println!("Created sprint {} for {}", sprint.id, owner);
    Ok(sprint)
}

#[query]
fn get_sprint(id: SprintId) -> Option<Sprint> {
    STATE.with(|state| state.borrow().get_sprint(id).cloned())
}

#[query]
fn get_my_sprints() -> Vec<Sprint> {
    let caller = ic_cdk::caller();
    if caller == Principal::anonymous() {
        return vec![];
    }

    STATE.with(|state| state.borrow().get_user_sprints(caller))
}

#[update]
fn add_capture_to_sprint(sprint_id: SprintId, capture_id: CaptureId) -> Result<(), String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        let s = state.borrow();

        let sprint = s.get_sprint(sprint_id)
            .ok_or_else(|| "Sprint not found".to_string())?;
        if sprint.owner != caller {
            return Err("Not authorized to modify this sprint".to_string());
        }

        let capture = s.get_capture(capture_id)
            .ok_or_else(|| "Capture not found".to_string())?;
        if capture.owner != caller {
            return Err("Not authorized to add this capture".to_string());
        }

        drop(s);
        state.borrow_mut().add_capture_to_sprint(sprint_id, capture_id)
    })
}

#[update]
fn remove_capture_from_sprint(sprint_id: SprintId, capture_id: CaptureId) -> Result<(), String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        let s = state.borrow();

        let sprint = s.get_sprint(sprint_id)
            .ok_or_else(|| "Sprint not found".to_string())?;
        if sprint.owner != caller {
            return Err("Not authorized to modify this sprint".to_string());
        }

        drop(s);
        state.borrow_mut().remove_capture_from_sprint(sprint_id, capture_id)
    })
}

#[update]
fn update_sprint(id: SprintId, request: UpdateSprintRequest) -> Result<Sprint, String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        let s = state.borrow();
        let sprint = s.get_sprint(id)
            .ok_or_else(|| "Sprint not found".to_string())?;

        if sprint.owner != caller {
            return Err("Not authorized to update this sprint".to_string());
        }
        drop(s);

        state.borrow_mut().update_sprint(id, request)
            .ok_or_else(|| "Failed to update sprint".to_string())
    })
}

#[update]
fn delete_sprint(id: SprintId) -> Result<Sprint, String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        {
            let s = state.borrow();
            let sprint = s.get_sprint(id)
                .ok_or_else(|| "Sprint not found".to_string())?;

            if sprint.owner != caller {
                return Err("Not authorized to delete this sprint".to_string());
            }
        }

        state.borrow_mut().delete_sprint(id)
            .ok_or_else(|| "Failed to delete sprint".to_string())
    })
}

// =============================================================================
// Workspace API
// =============================================================================

#[update]
fn create_workspace(request: CreateWorkspaceRequest) -> Result<Workspace, String> {
    let owner = require_authenticated()?;

    let workspace = STATE.with(|state| {
        state.borrow_mut().create_workspace(owner, request)
    });

    ic_cdk::println!("Created workspace {} for {}", workspace.id, owner);
    Ok(workspace)
}

#[query]
fn get_workspace(id: WorkspaceId) -> Option<Workspace> {
    STATE.with(|state| state.borrow().get_workspace(id).cloned())
}

#[query]
fn get_my_workspaces() -> Vec<Workspace> {
    let caller = ic_cdk::caller();
    if caller == Principal::anonymous() {
        return vec![];
    }

    STATE.with(|state| state.borrow().get_user_workspaces(caller))
}

#[update]
fn update_workspace(id: WorkspaceId, request: UpdateWorkspaceRequest) -> Result<Workspace, String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        let s = state.borrow();
        let workspace = s.get_workspace(id)
            .ok_or_else(|| "Workspace not found".to_string())?;

        if workspace.owner != caller {
            return Err("Not authorized to update this workspace".to_string());
        }
        drop(s);

        state.borrow_mut().update_workspace(id, request)
            .ok_or_else(|| "Failed to update workspace".to_string())
    })
}

#[update]
fn delete_workspace(id: WorkspaceId) -> Result<Workspace, String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        {
            let s = state.borrow();
            let workspace = s.get_workspace(id)
                .ok_or_else(|| "Workspace not found".to_string())?;

            if workspace.owner != caller {
                return Err("Not authorized to delete this workspace".to_string());
            }
        }

        state.borrow_mut().delete_workspace(id)
            .ok_or_else(|| "Failed to delete workspace".to_string())
    })
}

// =============================================================================
// Document API
// =============================================================================

#[update]
fn create_document(request: CreateDocumentRequest) -> Result<Document, String> {
    let owner = require_authenticated()?;

    // Verify owner owns the workspace
    STATE.with(|state| {
        let s = state.borrow();
        let workspace = s.get_workspace(request.workspace_id)
            .ok_or_else(|| "Workspace not found".to_string())?;

        if workspace.owner != owner {
            return Err("Not authorized to create documents in this workspace".to_string());
        }

        drop(s);
        state.borrow_mut().create_document(owner, request)
    })
}

#[query]
fn get_document(id: DocumentId) -> Option<Document> {
    STATE.with(|state| state.borrow().get_document(id).cloned())
}

#[update]
fn update_document(
    id: DocumentId,
    title: Option<String>,
    content: Option<String>,
) -> Result<Document, String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        {
            let s = state.borrow();
            let doc = s.get_document(id)
                .ok_or_else(|| "Document not found".to_string())?;

            if doc.owner != caller {
                return Err("Not authorized to update this document".to_string());
            }
        }

        state.borrow_mut().update_document(id, title, content)
            .ok_or_else(|| "Failed to update document".to_string())
    })
}

#[query]
fn get_workspace_documents(workspace_id: WorkspaceId) -> Vec<Document> {
    let caller = ic_cdk::caller();

    STATE.with(|state| {
        let s = state.borrow();

        // Check if caller owns the workspace
        if let Some(workspace) = s.get_workspace(workspace_id) {
            if workspace.owner == caller {
                return s.get_workspace_documents(workspace_id);
            }
        }

        vec![]
    })
}

#[update]
fn delete_document(id: DocumentId) -> Result<Document, String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        {
            let s = state.borrow();
            let doc = s.get_document(id)
                .ok_or_else(|| "Document not found".to_string())?;

            if doc.owner != caller {
                return Err("Not authorized to delete this document".to_string());
            }
        }

        state.borrow_mut().delete_document(id)
            .ok_or_else(|| "Failed to delete document".to_string())
    })
}

// =============================================================================
// Template API
// =============================================================================

#[update]
fn create_template(request: CreateTemplateRequest) -> Result<Template, String> {
    let owner = require_authenticated()?;

    let template = STATE.with(|state| {
        state.borrow_mut().create_template(owner, request)
    });

    ic_cdk::println!("Created template {} for {}", template.id, owner);
    Ok(template)
}

#[query]
fn get_template(id: TemplateId) -> Option<Template> {
    STATE.with(|state| state.borrow().get_template(id).cloned())
}

#[query]
fn get_my_templates() -> Vec<Template> {
    let caller = ic_cdk::caller();
    if caller == Principal::anonymous() {
        return vec![];
    }

    STATE.with(|state| state.borrow().get_user_templates(caller))
}

#[query]
fn get_public_templates() -> Vec<Template> {
    STATE.with(|state| state.borrow().get_public_templates())
}

#[update]
fn update_template(id: TemplateId, request: UpdateTemplateRequest) -> Result<Template, String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        let s = state.borrow();
        let template = s.get_template(id)
            .ok_or_else(|| "Template not found".to_string())?;

        if template.owner != caller {
            return Err("Not authorized to update this template".to_string());
        }
        drop(s);

        state.borrow_mut().update_template(id, request)
            .ok_or_else(|| "Failed to update template".to_string())
    })
}

#[update]
fn delete_template(id: TemplateId) -> Result<Template, String> {
    let caller = require_authenticated()?;

    STATE.with(|state| {
        {
            let s = state.borrow();
            let template = s.get_template(id)
                .ok_or_else(|| "Template not found".to_string())?;

            if template.owner != caller {
                return Err("Not authorized to delete this template".to_string());
            }
        }

        state.borrow_mut().delete_template(id)
            .ok_or_else(|| "Failed to delete template".to_string())
    })
}

// =============================================================================
// Token-Based API (Session Authentication via auth-service)
// These endpoints accept an access_token for session-based authentication.
// Use these from the frontend when authenticating via email/password or OAuth.
// =============================================================================

/// Create a capture using session-based authentication
#[update]
async fn create_capture_with_token(access_token: String, request: CreateCaptureRequest) -> Result<Capture, String> {
    let user_id = require_authenticated_with_token(&access_token).await?;

    let capture = STATE.with(|state| {
        state.borrow_mut().create_capture_for_user_id(&user_id, request)
    });

    ic_cdk::println!("Created capture {} for user_id {}", capture.id, user_id);
    Ok(capture)
}

/// Update a capture using session-based authentication
#[update]
async fn update_capture_with_token(access_token: String, request: UpdateCaptureRequest) -> Result<Capture, String> {
    let user_id = require_authenticated_with_token(&access_token).await?;

    STATE.with(|state| {
        // Verify ownership
        if !state.borrow().is_capture_owned_by_user_id(request.id, &user_id) {
            return Err("Not authorized to update this capture".to_string());
        }

        state.borrow_mut().update_capture(request)
            .ok_or_else(|| "Failed to update capture".to_string())
    })
}

/// Delete a capture using session-based authentication
#[update]
async fn delete_capture_with_token(access_token: String, id: CaptureId) -> Result<Capture, String> {
    let user_id = require_authenticated_with_token(&access_token).await?;

    STATE.with(|state| {
        state.borrow_mut().delete_capture_by_user_id(id, &user_id)
            .ok_or_else(|| "Capture not found or not authorized".to_string())
    })
}

/// Get captures for the authenticated user (session-based)
#[update]
async fn get_my_captures_with_token(
    access_token: String,
    filter: Option<CaptureFilter>,
    pagination: Option<PaginationParams>,
) -> Result<PaginatedResponse<Capture>, String> {
    let user_id = require_authenticated_with_token(&access_token).await?;

    Ok(STATE.with(|state| {
        state.borrow().get_user_id_captures(
            &user_id,
            filter,
            pagination.unwrap_or_default(),
        )
    }))
}

/// Create a sprint using session-based authentication
#[update]
async fn create_sprint_with_token(access_token: String, request: CreateSprintRequest) -> Result<Sprint, String> {
    let user_id = require_authenticated_with_token(&access_token).await?;

    let sprint = STATE.with(|state| {
        state.borrow_mut().create_sprint_for_user_id(&user_id, request)
    });

    ic_cdk::println!("Created sprint {} for user_id {}", sprint.id, user_id);
    Ok(sprint)
}

/// Get sprints for the authenticated user (session-based)
#[update]
async fn get_my_sprints_with_token(access_token: String) -> Result<Vec<Sprint>, String> {
    let user_id = require_authenticated_with_token(&access_token).await?;

    Ok(STATE.with(|state| {
        state.borrow().get_user_id_sprints(&user_id)
    }))
}

/// Create a workspace using session-based authentication
#[update]
async fn create_workspace_with_token(access_token: String, request: CreateWorkspaceRequest) -> Result<Workspace, String> {
    let user_id = require_authenticated_with_token(&access_token).await?;

    let workspace = STATE.with(|state| {
        state.borrow_mut().create_workspace_for_user_id(&user_id, request)
    });

    ic_cdk::println!("Created workspace {} for user_id {}", workspace.id, user_id);
    Ok(workspace)
}

/// Get workspaces for the authenticated user (session-based)
#[update]
async fn get_my_workspaces_with_token(access_token: String) -> Result<Vec<Workspace>, String> {
    let user_id = require_authenticated_with_token(&access_token).await?;

    Ok(STATE.with(|state| {
        state.borrow().get_user_id_workspaces(&user_id)
    }))
}

/// Create a template using session-based authentication
#[update]
async fn create_template_with_token(access_token: String, request: CreateTemplateRequest) -> Result<Template, String> {
    let user_id = require_authenticated_with_token(&access_token).await?;

    let template = STATE.with(|state| {
        state.borrow_mut().create_template_for_user_id(&user_id, request)
    });

    ic_cdk::println!("Created template {} for user_id {}", template.id, user_id);
    Ok(template)
}

/// Get templates for the authenticated user (session-based)
#[update]
async fn get_my_templates_with_token(access_token: String) -> Result<Vec<Template>, String> {
    let user_id = require_authenticated_with_token(&access_token).await?;

    Ok(STATE.with(|state| {
        state.borrow().get_user_id_templates(&user_id)
    }))
}

// =============================================================================
// Stats & Health
// =============================================================================

#[derive(candid::CandidType, serde::Serialize)]
pub struct Stats {
    pub total_captures: u64,
    pub total_sprints: u64,
    pub total_workspaces: u64,
    pub total_documents: u64,
    pub total_templates: u64,
    pub total_users: u64,
}

#[query]
fn get_stats() -> Stats {
    STATE.with(|state| {
        let s = state.borrow();
        Stats {
            total_captures: s.captures.len() as u64,
            total_sprints: s.sprints.len() as u64,
            total_workspaces: s.workspaces.len() as u64,
            total_documents: s.documents.len() as u64,
            total_templates: s.templates.len() as u64,
            total_users: s.user_captures.len() as u64,
        }
    })
}

#[query]
fn health() -> String {
    "ok".to_string()
}

// =============================================================================
// Discussion API (Story FOS-4.1.2)
// =============================================================================

#[update]
fn create_discussion(args: discussion::CreateDiscussionArgs) -> Result<u64, String> {
    let caller = require_authenticated()?;
    let now = ic_cdk::api::time();
    discussion::api::create_discussion(caller, args, now)
}

#[query]
fn get_discussion(id: u64) -> Option<discussion::Discussion> {
    discussion::api::get_discussion(id)
}

#[query]
fn list_discussions(
    filter: Option<discussion::DiscussionFilter>,
    pagination: Option<discussion::DiscussionPaginationParams>,
) -> discussion::PaginatedDiscussionResponse {
    discussion::api::list_discussions(filter, pagination)
}

#[update]
fn archive_discussion(discussion_id: u64) -> Result<(), String> {
    let caller = require_authenticated()?;
    let now = ic_cdk::api::time();
    discussion::api::archive_discussion(caller, discussion_id, now)
}

#[update]
fn add_comment(args: discussion::AddCommentArgs) -> Result<u64, String> {
    let caller = require_authenticated()?;
    let now = ic_cdk::api::time();
    discussion::api::add_comment(caller, args, now)
}

#[query]
fn get_comments(discussion_id: u64, offset: u64, limit: u64) -> Vec<discussion::Comment> {
    discussion::api::get_comments(discussion_id, offset, limit)
}

#[update]
fn retract_comment(comment_id: u64) -> Result<(), String> {
    let caller = require_authenticated()?;
    let now = ic_cdk::api::time();
    discussion::api::retract_comment(caller, comment_id, now)
}

#[update]
fn advance_stage(discussion_id: u64) -> Result<discussion::DiscussionStage, String> {
    let caller = require_authenticated()?;
    let now = ic_cdk::api::time();
    discussion::api::advance_stage(caller, discussion_id, now)
}

#[update]
fn invite_contributor(discussion_id: u64, invitee: Principal) -> Result<(), String> {
    let caller = require_authenticated()?;
    let now = ic_cdk::api::time();
    discussion::api::invite_contributor(caller, discussion_id, invitee, now)
}

#[update]
fn respond_to_invite(discussion_id: u64, accept: bool) -> Result<(), String> {
    let caller = require_authenticated()?;
    let now = ic_cdk::api::time();
    discussion::api::respond_to_invite(caller, discussion_id, accept, now)
}

#[query]
fn get_quality_gate_status(discussion_id: u64) -> Option<discussion::QualityGateStatus> {
    let now = ic_cdk::api::time();
    discussion::api::get_quality_gate_status(discussion_id, now)
}

#[query]
fn get_discussion_hash(discussion_id: u64) -> Option<String> {
    discussion::api::get_discussion_hash(discussion_id)
}

// Export candid interface
ic_cdk::export_candid!();
