use candid::{decode_one, encode_args, encode_one, CandidType, Principal};
use pocket_ic::{PocketIc, WasmResult};
use serde::{Deserialize, Serialize};

// ============================================================================
// Type Definitions (matching canister types)
// ============================================================================

#[derive(CandidType, Clone, Serialize, Deserialize, Debug, PartialEq)]
enum CaptureType {
    Idea,
    Task,
    Project,
    Reflection,
    Outline,
    Calendar,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug, PartialEq)]
enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug, PartialEq)]
enum CaptureStatus {
    Draft,
    Active,
    InProgress,
    Blocked,
    Completed,
    Archived,
    Cancelled,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug, Default)]
struct DynamicFields {
    estimate: Option<u32>,
    due_date: Option<u64>,
    start_date: Option<u64>,
    assignees: Vec<String>,
    labels: Vec<String>,
    related_captures: Vec<u64>,
    parent_id: Option<u64>,
    sprint_id: Option<u64>,
    workspace_id: Option<u64>,
    custom_fields: Vec<(String, String)>,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug)]
struct Capture {
    id: u64,
    owner: Principal,
    capture_type: CaptureType,
    title: String,
    description: Option<String>,
    content: Option<String>,
    priority: Priority,
    status: CaptureStatus,
    fields: DynamicFields,
    created_at: u64,
    updated_at: u64,
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct CreateCaptureRequest {
    capture_type: CaptureType,
    title: String,
    description: Option<String>,
    content: Option<String>,
    priority: Option<Priority>,
    fields: Option<DynamicFields>,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug)]
struct Sprint {
    id: u64,
    owner: Principal,
    name: String,
    goal: Option<String>,
    status: SprintStatus,
    start_date: u64,
    end_date: u64,
    capacity: Option<u32>,
    capture_ids: Vec<u64>,
    created_at: u64,
    updated_at: u64,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug, PartialEq)]
enum SprintStatus {
    Planning,
    Active,
    Review,
    Completed,
    Cancelled,
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct CreateSprintRequest {
    name: String,
    goal: Option<String>,
    start_date: u64,
    end_date: u64,
    capacity: Option<u32>,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug)]
struct Workspace {
    id: u64,
    owner: Principal,
    name: String,
    description: Option<String>,
    icon: Option<String>,
    parent_id: Option<u64>,
    is_archived: bool,
    created_at: u64,
    updated_at: u64,
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct CreateWorkspaceRequest {
    name: String,
    description: Option<String>,
    icon: Option<String>,
    parent_id: Option<u64>,
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct Stats {
    total_captures: u64,
    total_sprints: u64,
    total_workspaces: u64,
    total_documents: u64,
    total_templates: u64,
    total_users: u64,
}

// ============================================================================
// Test Helpers
// ============================================================================

fn unwrap_wasm_result(result: WasmResult) -> Vec<u8> {
    match result {
        WasmResult::Reply(bytes) => bytes,
        WasmResult::Reject(msg) => panic!("Canister rejected call: {}", msg),
    }
}

fn setup() -> (PocketIc, Principal, Principal) {
    let pic = PocketIc::new();

    let wasm_path = std::env::var("CARGO_MANIFEST_DIR")
        .map(|dir| format!("{}/target/wasm32-unknown-unknown/release/foundery_os_core.wasm", dir))
        .unwrap_or_else(|_| "target/wasm32-unknown-unknown/release/foundery_os_core.wasm".to_string());

    let wasm = std::fs::read(&wasm_path)
        .unwrap_or_else(|_| panic!("Could not read WASM file at: {}", wasm_path));

    let controller = Principal::from_text("aaaaa-aa").unwrap();
    let user = Principal::from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

    let canister_id = pic.create_canister();
    pic.add_cycles(canister_id, 2_000_000_000_000);
    pic.install_canister(canister_id, wasm, encode_one(Some(vec![controller])).unwrap(), None);

    (pic, canister_id, user)
}

// ============================================================================
// Smoke Tests
// ============================================================================

#[test]
fn test_health_check() {
    let (pic, canister_id, _) = setup();

    let response = pic.query_call(
        canister_id,
        Principal::anonymous(),
        "health",
        encode_one(()).unwrap(),
    ).unwrap();

    let health: String = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert_eq!(health, "ok");
}

#[test]
fn test_get_stats_empty() {
    let (pic, canister_id, _) = setup();

    let response = pic.query_call(
        canister_id,
        Principal::anonymous(),
        "get_stats",
        encode_one(()).unwrap(),
    ).unwrap();

    let stats: Stats = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert_eq!(stats.total_captures, 0);
    assert_eq!(stats.total_sprints, 0);
    assert_eq!(stats.total_workspaces, 0);
    assert_eq!(stats.total_documents, 0);
    assert_eq!(stats.total_templates, 0);
}

#[test]
fn test_create_capture() {
    let (pic, canister_id, user) = setup();

    let request = CreateCaptureRequest {
        capture_type: CaptureType::Idea,
        title: "Test Idea".to_string(),
        description: Some("A test idea description".to_string()),
        content: None,
        priority: Some(Priority::High),
        fields: None,
    };

    let response = pic.update_call(
        canister_id,
        user,
        "create_capture",
        encode_one(request).unwrap(),
    ).unwrap();

    let result: Result<Capture, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok());

    let capture = result.unwrap();
    assert_eq!(capture.id, 1);
    assert_eq!(capture.title, "Test Idea");
    assert_eq!(capture.capture_type, CaptureType::Idea);
    assert_eq!(capture.priority, Priority::High);
    assert_eq!(capture.status, CaptureStatus::Draft);
    assert_eq!(capture.owner, user);
}

#[test]
fn test_create_capture_requires_auth() {
    let (pic, canister_id, _) = setup();

    let request = CreateCaptureRequest {
        capture_type: CaptureType::Task,
        title: "Anonymous Task".to_string(),
        description: None,
        content: None,
        priority: None,
        fields: None,
    };

    let response = pic.update_call(
        canister_id,
        Principal::anonymous(),
        "create_capture",
        encode_one(request).unwrap(),
    ).unwrap();

    let result: Result<Capture, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Authentication required"));
}

#[test]
fn test_create_sprint() {
    let (pic, canister_id, user) = setup();

    let request = CreateSprintRequest {
        name: "Sprint 1".to_string(),
        goal: Some("Complete MVP".to_string()),
        start_date: 1700000000000000000,
        end_date: 1701000000000000000,
        capacity: Some(20),
    };

    let response = pic.update_call(
        canister_id,
        user,
        "create_sprint",
        encode_one(request).unwrap(),
    ).unwrap();

    let result: Result<Sprint, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok());

    let sprint = result.unwrap();
    assert_eq!(sprint.id, 1);
    assert_eq!(sprint.name, "Sprint 1");
    assert_eq!(sprint.status, SprintStatus::Planning);
    assert_eq!(sprint.capacity, Some(20));
}

#[test]
fn test_create_workspace() {
    let (pic, canister_id, user) = setup();

    let request = CreateWorkspaceRequest {
        name: "My Workspace".to_string(),
        description: Some("A test workspace".to_string()),
        icon: None,
        parent_id: None,
    };

    let response = pic.update_call(
        canister_id,
        user,
        "create_workspace",
        encode_one(request).unwrap(),
    ).unwrap();

    let result: Result<Workspace, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok());

    let workspace = result.unwrap();
    assert_eq!(workspace.id, 1);
    assert_eq!(workspace.name, "My Workspace");
    assert!(!workspace.is_archived);
}

#[test]
fn test_add_capture_to_sprint() {
    let (pic, canister_id, user) = setup();

    // Create a capture
    let capture_request = CreateCaptureRequest {
        capture_type: CaptureType::Task,
        title: "Sprint Task".to_string(),
        description: None,
        content: None,
        priority: None,
        fields: None,
    };

    let capture_response = pic.update_call(
        canister_id,
        user,
        "create_capture",
        encode_one(capture_request).unwrap(),
    ).unwrap();

    let capture: Result<Capture, String> = decode_one(&unwrap_wasm_result(capture_response)).unwrap();
    let capture_id = capture.unwrap().id;

    // Create a sprint
    let sprint_request = CreateSprintRequest {
        name: "Sprint 1".to_string(),
        goal: None,
        start_date: 1700000000000000000,
        end_date: 1701000000000000000,
        capacity: None,
    };

    let sprint_response = pic.update_call(
        canister_id,
        user,
        "create_sprint",
        encode_one(sprint_request).unwrap(),
    ).unwrap();

    let sprint: Result<Sprint, String> = decode_one(&unwrap_wasm_result(sprint_response)).unwrap();
    let sprint_id = sprint.unwrap().id;

    // Add capture to sprint
    let add_response = pic.update_call(
        canister_id,
        user,
        "add_capture_to_sprint",
        encode_args((sprint_id, capture_id)).unwrap(),
    ).unwrap();

    let add_result: Result<(), String> = decode_one(&unwrap_wasm_result(add_response)).unwrap();
    assert!(add_result.is_ok());

    // Verify sprint contains the capture
    let get_sprint_response = pic.query_call(
        canister_id,
        user,
        "get_sprint",
        encode_one(sprint_id).unwrap(),
    ).unwrap();

    let get_sprint: Option<Sprint> = decode_one(&unwrap_wasm_result(get_sprint_response)).unwrap();
    assert!(get_sprint.is_some());
    assert!(get_sprint.unwrap().capture_ids.contains(&capture_id));
}

#[test]
fn test_stats_after_operations() {
    let (pic, canister_id, user) = setup();

    // Create a capture
    let capture_request = CreateCaptureRequest {
        capture_type: CaptureType::Idea,
        title: "Test".to_string(),
        description: None,
        content: None,
        priority: None,
        fields: None,
    };

    pic.update_call(
        canister_id,
        user,
        "create_capture",
        encode_one(capture_request).unwrap(),
    ).unwrap();

    // Create a sprint
    let sprint_request = CreateSprintRequest {
        name: "Sprint".to_string(),
        goal: None,
        start_date: 0,
        end_date: 1,
        capacity: None,
    };

    pic.update_call(
        canister_id,
        user,
        "create_sprint",
        encode_one(sprint_request).unwrap(),
    ).unwrap();

    // Create a workspace
    let workspace_request = CreateWorkspaceRequest {
        name: "Workspace".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    pic.update_call(
        canister_id,
        user,
        "create_workspace",
        encode_one(workspace_request).unwrap(),
    ).unwrap();

    // Check stats
    let response = pic.query_call(
        canister_id,
        Principal::anonymous(),
        "get_stats",
        encode_one(()).unwrap(),
    ).unwrap();

    let stats: Stats = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert_eq!(stats.total_captures, 1);
    assert_eq!(stats.total_sprints, 1);
    assert_eq!(stats.total_workspaces, 1);
    assert_eq!(stats.total_users, 1);
}

// ============================================================================
// Inter-Canister Auth Tests (Story FOS-1.1.6)
// ============================================================================

/// Additional type definitions for token-based auth tests
#[derive(CandidType, Clone, Serialize, Deserialize, Debug)]
struct PaginatedCaptureResponse {
    items: Vec<Capture>,
    total: u64,
    offset: u64,
    limit: u64,
}

// ============================================================================
// Task 6.1: Invalid Token Returns Auth Error (AC-1.1.6.1)
// ============================================================================

#[test]
fn test_invalid_token_rejected() {
    let (pic, canister_id, _user) = setup();

    // Try to create capture with invalid token
    let request = CreateCaptureRequest {
        capture_type: CaptureType::Idea,
        title: "Test with invalid token".to_string(),
        description: None,
        content: None,
        priority: None,
        fields: None,
    };

    let response = pic.update_call(
        canister_id,
        Principal::anonymous(),
        "create_capture_with_token",
        encode_args(("invalid-token-12345".to_string(), request)).unwrap(),
    ).unwrap();

    let result: Result<Capture, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Should reject invalid token");
    let err = result.unwrap_err();
    // Should fail because auth service is not configured
    assert!(
        err.contains("Auth service not configured") || err.contains("auth"),
        "Error should mention auth: {}",
        err
    );
}

#[test]
fn test_empty_token_rejected() {
    let (pic, canister_id, _user) = setup();

    let request = CreateCaptureRequest {
        capture_type: CaptureType::Task,
        title: "Test with empty token".to_string(),
        description: None,
        content: None,
        priority: None,
        fields: None,
    };

    let response = pic.update_call(
        canister_id,
        Principal::anonymous(),
        "create_capture_with_token",
        encode_args(("".to_string(), request)).unwrap(),
    ).unwrap();

    let result: Result<Capture, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Should reject empty token");
    let err = result.unwrap_err();
    assert!(
        err.contains("required") || err.contains("token"),
        "Error should mention token required: {}",
        err
    );
}

// ============================================================================
// Task 6.3: Auth Service Configuration (AC-1.1.6.3)
// ============================================================================

#[test]
fn test_auth_service_not_configured_returns_error() {
    let (pic, canister_id, _user) = setup();

    let request = CreateCaptureRequest {
        capture_type: CaptureType::Project,
        title: "Test without auth service".to_string(),
        description: None,
        content: None,
        priority: None,
        fields: None,
    };

    // Auth service is not configured in basic setup
    let response = pic.update_call(
        canister_id,
        Principal::anonymous(),
        "create_capture_with_token",
        encode_args(("some-token".to_string(), request)).unwrap(),
    ).unwrap();

    let result: Result<Capture, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Auth service not configured"),
        "Error should say auth service not configured: {}",
        err
    );
}

#[test]
fn test_set_auth_service_controller_only() {
    let (pic, canister_id, user) = setup();

    // Non-controller should not be able to set auth service
    let fake_auth_service = Principal::from_slice(&[99, 99, 99]);
    let response = pic.update_call(
        canister_id,
        user,  // user is not a controller
        "set_auth_service",
        encode_one(fake_auth_service).unwrap(),
    ).unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Non-controller should not be able to set auth service");
    let err = result.unwrap_err();
    assert!(
        err.contains("controller") || err.contains("Controller"),
        "Error should mention controller: {}",
        err
    );
}

#[test]
fn test_set_auth_service_by_controller() {
    let (pic, canister_id, _user) = setup();

    let controller = Principal::from_text("aaaaa-aa").unwrap();
    let auth_service_principal = Principal::from_slice(&[1, 2, 3, 4]);

    // Controller should be able to set auth service
    let response = pic.update_call(
        canister_id,
        controller,
        "set_auth_service",
        encode_one(auth_service_principal).unwrap(),
    ).unwrap();

    let result: Result<(), String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Controller should be able to set auth service: {:?}", result);

    // Verify auth service is set
    let get_response = pic.query_call(
        canister_id,
        Principal::anonymous(),
        "get_auth_service",
        encode_one(()).unwrap(),
    ).unwrap();

    let stored_auth_service: Option<Principal> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert_eq!(stored_auth_service, Some(auth_service_principal));
}

// ============================================================================
// Task 6.5: User Data Isolation Test (AC-1.1.6.2)
// Tests backwards compatibility with Principal-based auth
// ============================================================================

#[test]
fn test_principal_auth_still_works() {
    let (pic, canister_id, user) = setup();

    // Create capture using Principal-based auth (original method)
    let request = CreateCaptureRequest {
        capture_type: CaptureType::Idea,
        title: "Principal-based capture".to_string(),
        description: Some("Created with II/Principal auth".to_string()),
        content: None,
        priority: Some(Priority::Medium),
        fields: None,
    };

    let response = pic.update_call(
        canister_id,
        user,
        "create_capture",
        encode_one(request).unwrap(),
    ).unwrap();

    let result: Result<Capture, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Principal-based auth should still work");

    let capture = result.unwrap();
    assert_eq!(capture.owner, user, "Capture should be owned by the caller principal");
    assert_eq!(capture.title, "Principal-based capture");
}

#[test]
fn test_user_cannot_access_others_captures_principal_auth() {
    let (pic, canister_id, user_a) = setup();
    let user_b = Principal::from_slice(&[11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);

    // User A creates a capture
    let request = CreateCaptureRequest {
        capture_type: CaptureType::Task,
        title: "User A's private task".to_string(),
        description: None,
        content: None,
        priority: None,
        fields: None,
    };

    let response = pic.update_call(
        canister_id,
        user_a,
        "create_capture",
        encode_one(request).unwrap(),
    ).unwrap();

    let create_result: Result<Capture, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(create_result.is_ok());
    let capture_id = create_result.unwrap().id;

    // User B tries to update User A's capture - should fail
    let update_request = UpdateCaptureRequest {
        id: capture_id,
        title: Some("Hacked!".to_string()),
        description: None,
        content: None,
        priority: None,
        status: None,
        fields: None,
    };

    let update_response = pic.update_call(
        canister_id,
        user_b,
        "update_capture",
        encode_one(update_request).unwrap(),
    ).unwrap();

    let update_result: Result<Capture, String> = decode_one(&unwrap_wasm_result(update_response)).unwrap();
    assert!(update_result.is_err(), "User B should not be able to update User A's capture");

    // User B tries to delete User A's capture - should fail
    let delete_response = pic.update_call(
        canister_id,
        user_b,
        "delete_capture",
        encode_one(capture_id).unwrap(),
    ).unwrap();

    let delete_result: Result<Capture, String> = decode_one(&unwrap_wasm_result(delete_response)).unwrap();
    assert!(delete_result.is_err(), "User B should not be able to delete User A's capture");
}

#[test]
fn test_get_my_captures_only_returns_own_captures() {
    let (pic, canister_id, user_a) = setup();
    let user_b = Principal::from_slice(&[21, 22, 23, 24, 25, 26, 27, 28, 29, 30]);

    // User A creates captures
    for i in 0..3 {
        let request = CreateCaptureRequest {
            capture_type: CaptureType::Idea,
            title: format!("User A Capture {}", i),
            description: None,
            content: None,
            priority: None,
            fields: None,
        };

        pic.update_call(
            canister_id,
            user_a,
            "create_capture",
            encode_one(request).unwrap(),
        ).unwrap();
    }

    // User B creates captures
    for i in 0..2 {
        let request = CreateCaptureRequest {
            capture_type: CaptureType::Task,
            title: format!("User B Capture {}", i),
            description: None,
            content: None,
            priority: None,
            fields: None,
        };

        pic.update_call(
            canister_id,
            user_b,
            "create_capture",
            encode_one(request).unwrap(),
        ).unwrap();
    }

    // User A should only see their 3 captures
    let response_a = pic.query_call(
        canister_id,
        user_a,
        "get_my_captures",
        encode_args((None::<CaptureFilter>, None::<PaginationParams>)).unwrap(),
    ).unwrap();

    let captures_a: PaginatedCaptureResponse = decode_one(&unwrap_wasm_result(response_a)).unwrap();
    assert_eq!(captures_a.total, 3, "User A should have 3 captures");
    for capture in &captures_a.items {
        assert_eq!(capture.owner, user_a, "All captures should be owned by User A");
    }

    // User B should only see their 2 captures
    let response_b = pic.query_call(
        canister_id,
        user_b,
        "get_my_captures",
        encode_args((None::<CaptureFilter>, None::<PaginationParams>)).unwrap(),
    ).unwrap();

    let captures_b: PaginatedCaptureResponse = decode_one(&unwrap_wasm_result(response_b)).unwrap();
    assert_eq!(captures_b.total, 2, "User B should have 2 captures");
    for capture in &captures_b.items {
        assert_eq!(capture.owner, user_b, "All captures should be owned by User B");
    }
}

// ============================================================================
// Additional Type for UpdateCaptureRequest
// ============================================================================

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct UpdateCaptureRequest {
    id: u64,
    title: Option<String>,
    description: Option<String>,
    content: Option<String>,
    priority: Option<Priority>,
    status: Option<CaptureStatus>,
    fields: Option<DynamicFields>,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Default)]
struct CaptureFilter {
    capture_type: Option<CaptureType>,
    status: Option<CaptureStatus>,
    priority: Option<Priority>,
    sprint_id: Option<u64>,
    workspace_id: Option<u64>,
    labels: Option<Vec<String>>,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Default)]
struct PaginationParams {
    offset: Option<u64>,
    limit: Option<u64>,
}

// ============================================================================
// Story FOS-1.1.7: PocketIC Integration Tests
// ============================================================================

// ============================================================================
// Task 1: Document CRUD Tests (AC: 1.1.7.3)
// ============================================================================

#[derive(CandidType, Clone, Serialize, Deserialize, Debug)]
struct Document {
    id: u64,
    workspace_id: u64,
    owner: Principal,
    title: String,
    content: String,
    is_template: bool,
    template_id: Option<u64>,
    parent_id: Option<u64>,
    created_at: u64,
    updated_at: u64,
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct CreateDocumentRequest {
    workspace_id: u64,
    title: String,
    content: Option<String>,
    template_id: Option<u64>,
    parent_id: Option<u64>,
}

#[test]
fn test_create_document() {
    let (pic, canister_id, user) = setup();

    // First create a workspace
    let workspace_request = CreateWorkspaceRequest {
        name: "Test Workspace".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    let workspace_response = pic.update_call(
        canister_id,
        user,
        "create_workspace",
        encode_one(workspace_request).unwrap(),
    ).unwrap();

    let workspace: Result<Workspace, String> = decode_one(&unwrap_wasm_result(workspace_response)).unwrap();
    let workspace_id = workspace.unwrap().id;

    // Create a document in the workspace
    let doc_request = CreateDocumentRequest {
        workspace_id,
        title: "Test Document".to_string(),
        content: Some("Initial content".to_string()),
        template_id: None,
        parent_id: None,
    };

    let doc_response = pic.update_call(
        canister_id,
        user,
        "create_document",
        encode_one(doc_request).unwrap(),
    ).unwrap();

    let result: Result<Document, String> = decode_one(&unwrap_wasm_result(doc_response)).unwrap();
    assert!(result.is_ok(), "Should create document: {:?}", result);

    let doc = result.unwrap();
    assert_eq!(doc.id, 1);
    assert_eq!(doc.workspace_id, workspace_id);
    assert_eq!(doc.title, "Test Document");
    assert_eq!(doc.content, "Initial content");
    assert_eq!(doc.owner, user);
}

#[test]
fn test_get_document() {
    let (pic, canister_id, user) = setup();

    // Create workspace and document
    let workspace_request = CreateWorkspaceRequest {
        name: "Test Workspace".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    let workspace_response = pic.update_call(
        canister_id,
        user,
        "create_workspace",
        encode_one(workspace_request).unwrap(),
    ).unwrap();

    let workspace: Result<Workspace, String> = decode_one(&unwrap_wasm_result(workspace_response)).unwrap();
    let workspace_id = workspace.unwrap().id;

    let doc_request = CreateDocumentRequest {
        workspace_id,
        title: "Retrievable Doc".to_string(),
        content: Some("Content to retrieve".to_string()),
        template_id: None,
        parent_id: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_document",
        encode_one(doc_request).unwrap(),
    ).unwrap();

    let created: Result<Document, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let doc_id = created.unwrap().id;

    // Get the document
    let get_response = pic.query_call(
        canister_id,
        user,
        "get_document",
        encode_one(doc_id).unwrap(),
    ).unwrap();

    let fetched: Option<Document> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert!(fetched.is_some(), "Should retrieve the document");

    let doc = fetched.unwrap();
    assert_eq!(doc.id, doc_id);
    assert_eq!(doc.title, "Retrievable Doc");
}

#[test]
fn test_update_document() {
    let (pic, canister_id, user) = setup();

    // Create workspace and document
    let workspace_request = CreateWorkspaceRequest {
        name: "Test Workspace".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    let workspace_response = pic.update_call(
        canister_id,
        user,
        "create_workspace",
        encode_one(workspace_request).unwrap(),
    ).unwrap();

    let workspace: Result<Workspace, String> = decode_one(&unwrap_wasm_result(workspace_response)).unwrap();
    let workspace_id = workspace.unwrap().id;

    let doc_request = CreateDocumentRequest {
        workspace_id,
        title: "Original Title".to_string(),
        content: Some("Original content".to_string()),
        template_id: None,
        parent_id: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_document",
        encode_one(doc_request).unwrap(),
    ).unwrap();

    let created: Result<Document, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let doc_id = created.unwrap().id;

    // Update the document
    let update_response = pic.update_call(
        canister_id,
        user,
        "update_document",
        encode_args((doc_id, Some("Updated Title".to_string()), Some("Updated content".to_string()))).unwrap(),
    ).unwrap();

    let update_result: Result<Document, String> = decode_one(&unwrap_wasm_result(update_response)).unwrap();
    assert!(update_result.is_ok(), "Should update document: {:?}", update_result);

    let updated = update_result.unwrap();
    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.content, "Updated content");
}

#[test]
fn test_document_belongs_to_workspace() {
    let (pic, canister_id, user) = setup();

    // Create two workspaces
    let ws1_request = CreateWorkspaceRequest {
        name: "Workspace 1".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    let ws1_response = pic.update_call(
        canister_id,
        user,
        "create_workspace",
        encode_one(ws1_request).unwrap(),
    ).unwrap();

    let ws1: Result<Workspace, String> = decode_one(&unwrap_wasm_result(ws1_response)).unwrap();
    let ws1_id = ws1.unwrap().id;

    let ws2_request = CreateWorkspaceRequest {
        name: "Workspace 2".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    let ws2_response = pic.update_call(
        canister_id,
        user,
        "create_workspace",
        encode_one(ws2_request).unwrap(),
    ).unwrap();

    let ws2: Result<Workspace, String> = decode_one(&unwrap_wasm_result(ws2_response)).unwrap();
    let ws2_id = ws2.unwrap().id;

    // Create documents in each workspace
    for i in 0..3 {
        let doc_request = CreateDocumentRequest {
            workspace_id: ws1_id,
            title: format!("WS1 Doc {}", i),
            content: None,
            template_id: None,
            parent_id: None,
        };

        pic.update_call(
            canister_id,
            user,
            "create_document",
            encode_one(doc_request).unwrap(),
        ).unwrap();
    }

    for i in 0..2 {
        let doc_request = CreateDocumentRequest {
            workspace_id: ws2_id,
            title: format!("WS2 Doc {}", i),
            content: None,
            template_id: None,
            parent_id: None,
        };

        pic.update_call(
            canister_id,
            user,
            "create_document",
            encode_one(doc_request).unwrap(),
        ).unwrap();
    }

    // Get documents for workspace 1
    let ws1_docs_response = pic.query_call(
        canister_id,
        user,
        "get_workspace_documents",
        encode_one(ws1_id).unwrap(),
    ).unwrap();

    let ws1_docs: Vec<Document> = decode_one(&unwrap_wasm_result(ws1_docs_response)).unwrap();
    assert_eq!(ws1_docs.len(), 3, "Workspace 1 should have 3 documents");
    for doc in &ws1_docs {
        assert_eq!(doc.workspace_id, ws1_id, "All docs should belong to workspace 1");
    }

    // Get documents for workspace 2
    let ws2_docs_response = pic.query_call(
        canister_id,
        user,
        "get_workspace_documents",
        encode_one(ws2_id).unwrap(),
    ).unwrap();

    let ws2_docs: Vec<Document> = decode_one(&unwrap_wasm_result(ws2_docs_response)).unwrap();
    assert_eq!(ws2_docs.len(), 2, "Workspace 2 should have 2 documents");
    for doc in &ws2_docs {
        assert_eq!(doc.workspace_id, ws2_id, "All docs should belong to workspace 2");
    }
}

// ============================================================================
// Task 2: Template CRUD Tests (AC: 1.1.7.4)
// ============================================================================

#[derive(CandidType, Clone, Serialize, Deserialize, Debug, PartialEq)]
enum TemplateType {
    Capture,
    Document,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug)]
struct Template {
    id: u64,
    owner: Principal,
    template_type: TemplateType,
    name: String,
    description: Option<String>,
    content: String,
    capture_type: Option<CaptureType>,
    default_fields: Option<DynamicFields>,
    is_public: bool,
    created_at: u64,
    updated_at: u64,
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct CreateTemplateRequest {
    template_type: TemplateType,
    name: String,
    description: Option<String>,
    content: String,
    capture_type: Option<CaptureType>,
    default_fields: Option<DynamicFields>,
    is_public: Option<bool>,
}

#[test]
fn test_create_capture_template() {
    let (pic, canister_id, user) = setup();

    let request = CreateTemplateRequest {
        template_type: TemplateType::Capture,
        name: "Task Template".to_string(),
        description: Some("A template for tasks".to_string()),
        content: "Task content template".to_string(),
        capture_type: Some(CaptureType::Task),
        default_fields: Some(DynamicFields {
            estimate: Some(8),
            due_date: None,
            start_date: None,
            assignees: vec![],
            labels: vec!["template".to_string()],
            related_captures: vec![],
            parent_id: None,
            sprint_id: None,
            workspace_id: None,
            custom_fields: vec![],
        }),
        is_public: Some(false),
    };

    let response = pic.update_call(
        canister_id,
        user,
        "create_template",
        encode_one(request).unwrap(),
    ).unwrap();

    let result: Result<Template, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Should create capture template: {:?}", result);

    let template = result.unwrap();
    assert_eq!(template.id, 1);
    assert_eq!(template.name, "Task Template");
    assert_eq!(template.template_type, TemplateType::Capture);
    assert_eq!(template.capture_type, Some(CaptureType::Task));
    assert!(!template.is_public);
}

#[test]
fn test_create_document_template() {
    let (pic, canister_id, user) = setup();

    let request = CreateTemplateRequest {
        template_type: TemplateType::Document,
        name: "Meeting Notes Template".to_string(),
        description: Some("Template for meeting notes".to_string()),
        content: "# Meeting Notes\n\n## Attendees\n\n## Agenda\n\n## Action Items\n".to_string(),
        capture_type: None,
        default_fields: None,
        is_public: Some(true),
    };

    let response = pic.update_call(
        canister_id,
        user,
        "create_template",
        encode_one(request).unwrap(),
    ).unwrap();

    let result: Result<Template, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Should create document template: {:?}", result);

    let template = result.unwrap();
    assert_eq!(template.template_type, TemplateType::Document);
    assert!(template.is_public);
    assert!(template.content.contains("Meeting Notes"));
}

#[test]
fn test_get_template() {
    let (pic, canister_id, user) = setup();

    let request = CreateTemplateRequest {
        template_type: TemplateType::Capture,
        name: "Retrievable Template".to_string(),
        description: None,
        content: "Template content".to_string(),
        capture_type: Some(CaptureType::Idea),
        default_fields: None,
        is_public: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_template",
        encode_one(request).unwrap(),
    ).unwrap();

    let created: Result<Template, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let template_id = created.unwrap().id;

    let get_response = pic.query_call(
        canister_id,
        user,
        "get_template",
        encode_one(template_id).unwrap(),
    ).unwrap();

    let fetched: Option<Template> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert!(fetched.is_some(), "Should retrieve the template");

    let template = fetched.unwrap();
    assert_eq!(template.name, "Retrievable Template");
}

#[test]
fn test_get_my_templates() {
    let (pic, canister_id, user) = setup();
    let user_b = Principal::from_slice(&[31, 32, 33, 34, 35, 36, 37, 38, 39, 40]);

    // User A creates templates
    for i in 0..3 {
        let request = CreateTemplateRequest {
            template_type: TemplateType::Capture,
            name: format!("User A Template {}", i),
            description: None,
            content: "Content".to_string(),
            capture_type: None,
            default_fields: None,
            is_public: None,
        };

        pic.update_call(
            canister_id,
            user,
            "create_template",
            encode_one(request).unwrap(),
        ).unwrap();
    }

    // User B creates templates
    for i in 0..2 {
        let request = CreateTemplateRequest {
            template_type: TemplateType::Document,
            name: format!("User B Template {}", i),
            description: None,
            content: "Content".to_string(),
            capture_type: None,
            default_fields: None,
            is_public: None,
        };

        pic.update_call(
            canister_id,
            user_b,
            "create_template",
            encode_one(request).unwrap(),
        ).unwrap();
    }

    // User A should only see their 3 templates
    let response_a = pic.query_call(
        canister_id,
        user,
        "get_my_templates",
        encode_one(()).unwrap(),
    ).unwrap();

    let templates_a: Vec<Template> = decode_one(&unwrap_wasm_result(response_a)).unwrap();
    assert_eq!(templates_a.len(), 3, "User A should have 3 templates");

    // User B should only see their 2 templates
    let response_b = pic.query_call(
        canister_id,
        user_b,
        "get_my_templates",
        encode_one(()).unwrap(),
    ).unwrap();

    let templates_b: Vec<Template> = decode_one(&unwrap_wasm_result(response_b)).unwrap();
    assert_eq!(templates_b.len(), 2, "User B should have 2 templates");
}

#[test]
fn test_get_public_templates() {
    let (pic, canister_id, user) = setup();
    let user_b = Principal::from_slice(&[41, 42, 43, 44, 45, 46, 47, 48, 49, 50]);

    // Create some public templates
    let public_request = CreateTemplateRequest {
        template_type: TemplateType::Document,
        name: "Public Template 1".to_string(),
        description: None,
        content: "Public content".to_string(),
        capture_type: None,
        default_fields: None,
        is_public: Some(true),
    };

    pic.update_call(
        canister_id,
        user,
        "create_template",
        encode_one(public_request).unwrap(),
    ).unwrap();

    // Create a private template
    let private_request = CreateTemplateRequest {
        template_type: TemplateType::Capture,
        name: "Private Template".to_string(),
        description: None,
        content: "Private content".to_string(),
        capture_type: None,
        default_fields: None,
        is_public: Some(false),
    };

    pic.update_call(
        canister_id,
        user,
        "create_template",
        encode_one(private_request).unwrap(),
    ).unwrap();

    // User B creates a public template
    let public_request_b = CreateTemplateRequest {
        template_type: TemplateType::Document,
        name: "Public Template 2".to_string(),
        description: None,
        content: "Another public".to_string(),
        capture_type: None,
        default_fields: None,
        is_public: Some(true),
    };

    pic.update_call(
        canister_id,
        user_b,
        "create_template",
        encode_one(public_request_b).unwrap(),
    ).unwrap();

    // Get public templates (should be 2)
    let response = pic.query_call(
        canister_id,
        Principal::anonymous(),
        "get_public_templates",
        encode_one(()).unwrap(),
    ).unwrap();

    let public_templates: Vec<Template> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert_eq!(public_templates.len(), 2, "Should have 2 public templates");

    for template in &public_templates {
        assert!(template.is_public, "All returned templates should be public");
    }
}

// ============================================================================
// Story FOS-1.1.8: Complete CRUD API Surface
// ============================================================================

// ============================================================================
// Task 6.1: Sprint Update/Delete Tests (AC: 1.1.8.1)
// ============================================================================

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct UpdateSprintRequest {
    name: Option<String>,
    goal: Option<String>,
    status: Option<SprintStatus>,
    start_date: Option<u64>,
    end_date: Option<u64>,
    capacity: Option<u32>,
}

#[test]
fn test_update_sprint() {
    let (pic, canister_id, user) = setup();

    // Create a sprint
    let create_request = CreateSprintRequest {
        name: "Original Sprint".to_string(),
        goal: Some("Original goal".to_string()),
        start_date: 1700000000000000000,
        end_date: 1701000000000000000,
        capacity: Some(20),
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_sprint",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Sprint, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let sprint_id = created.unwrap().id;

    // Update the sprint
    let update_request = UpdateSprintRequest {
        name: Some("Updated Sprint".to_string()),
        goal: Some("Updated goal".to_string()),
        status: Some(SprintStatus::Active),
        start_date: None,
        end_date: None,
        capacity: Some(30),
    };

    let update_response = pic.update_call(
        canister_id,
        user,
        "update_sprint",
        encode_args((sprint_id, update_request)).unwrap(),
    ).unwrap();

    let result: Result<Sprint, String> = decode_one(&unwrap_wasm_result(update_response)).unwrap();
    assert!(result.is_ok(), "Should update sprint: {:?}", result);

    let updated = result.unwrap();
    assert_eq!(updated.name, "Updated Sprint");
    assert_eq!(updated.goal, Some("Updated goal".to_string()));
    assert_eq!(updated.status, SprintStatus::Active);
    assert_eq!(updated.capacity, Some(30));
}

#[test]
fn test_delete_sprint() {
    let (pic, canister_id, user) = setup();

    // Create a sprint
    let create_request = CreateSprintRequest {
        name: "Sprint to Delete".to_string(),
        goal: None,
        start_date: 0,
        end_date: 100,
        capacity: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_sprint",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Sprint, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let sprint_id = created.unwrap().id;

    // Verify it exists
    let get_response = pic.query_call(
        canister_id,
        user,
        "get_sprint",
        encode_one(sprint_id).unwrap(),
    ).unwrap();

    let exists: Option<Sprint> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert!(exists.is_some(), "Sprint should exist before deletion");

    // Delete the sprint
    let delete_response = pic.update_call(
        canister_id,
        user,
        "delete_sprint",
        encode_one(sprint_id).unwrap(),
    ).unwrap();

    let delete_result: Result<Sprint, String> = decode_one(&unwrap_wasm_result(delete_response)).unwrap();
    assert!(delete_result.is_ok(), "Should delete sprint: {:?}", delete_result);

    // Verify it no longer exists
    let get_after_delete = pic.query_call(
        canister_id,
        user,
        "get_sprint",
        encode_one(sprint_id).unwrap(),
    ).unwrap();

    let not_exists: Option<Sprint> = decode_one(&unwrap_wasm_result(get_after_delete)).unwrap();
    assert!(not_exists.is_none(), "Sprint should not exist after deletion");
}

#[test]
fn test_update_sprint_unauthorized() {
    let (pic, canister_id, user_a) = setup();
    let user_b = Principal::from_slice(&[111, 112, 113, 114, 115, 116, 117, 118, 119, 120]);

    // User A creates a sprint
    let create_request = CreateSprintRequest {
        name: "User A's Sprint".to_string(),
        goal: None,
        start_date: 0,
        end_date: 100,
        capacity: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user_a,
        "create_sprint",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Sprint, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let sprint_id = created.unwrap().id;

    // User B tries to update User A's sprint
    let update_request = UpdateSprintRequest {
        name: Some("Hacked!".to_string()),
        goal: None,
        status: None,
        start_date: None,
        end_date: None,
        capacity: None,
    };

    let update_response = pic.update_call(
        canister_id,
        user_b,
        "update_sprint",
        encode_args((sprint_id, update_request)).unwrap(),
    ).unwrap();

    let result: Result<Sprint, String> = decode_one(&unwrap_wasm_result(update_response)).unwrap();
    assert!(result.is_err(), "User B should not be able to update User A's sprint");
    assert!(result.unwrap_err().contains("Not authorized"), "Error should mention authorization");
}

// ============================================================================
// Task 6.2: Workspace Update/Delete Tests (AC: 1.1.8.2)
// ============================================================================

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct UpdateWorkspaceRequest {
    name: Option<String>,
    description: Option<String>,
    icon: Option<String>,
    parent_id: Option<u64>,
    is_archived: Option<bool>,
}

#[test]
fn test_update_workspace() {
    let (pic, canister_id, user) = setup();

    // Create a workspace
    let create_request = CreateWorkspaceRequest {
        name: "Original Workspace".to_string(),
        description: Some("Original description".to_string()),
        icon: None,
        parent_id: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_workspace",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Workspace, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let workspace_id = created.unwrap().id;

    // Update the workspace
    let update_request = UpdateWorkspaceRequest {
        name: Some("Updated Workspace".to_string()),
        description: Some("Updated description".to_string()),
        icon: Some("🚀".to_string()),
        parent_id: None,
        is_archived: Some(true),
    };

    let update_response = pic.update_call(
        canister_id,
        user,
        "update_workspace",
        encode_args((workspace_id, update_request)).unwrap(),
    ).unwrap();

    let result: Result<Workspace, String> = decode_one(&unwrap_wasm_result(update_response)).unwrap();
    assert!(result.is_ok(), "Should update workspace: {:?}", result);

    let updated = result.unwrap();
    assert_eq!(updated.name, "Updated Workspace");
    assert_eq!(updated.description, Some("Updated description".to_string()));
    assert_eq!(updated.icon, Some("🚀".to_string()));
    assert!(updated.is_archived);
}

#[test]
fn test_delete_workspace() {
    let (pic, canister_id, user) = setup();

    // Create a workspace
    let create_request = CreateWorkspaceRequest {
        name: "Workspace to Delete".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_workspace",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Workspace, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let workspace_id = created.unwrap().id;

    // Verify it exists
    let get_response = pic.query_call(
        canister_id,
        user,
        "get_workspace",
        encode_one(workspace_id).unwrap(),
    ).unwrap();

    let exists: Option<Workspace> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert!(exists.is_some(), "Workspace should exist before deletion");

    // Delete the workspace
    let delete_response = pic.update_call(
        canister_id,
        user,
        "delete_workspace",
        encode_one(workspace_id).unwrap(),
    ).unwrap();

    let delete_result: Result<Workspace, String> = decode_one(&unwrap_wasm_result(delete_response)).unwrap();
    assert!(delete_result.is_ok(), "Should delete workspace: {:?}", delete_result);

    // Verify it no longer exists
    let get_after_delete = pic.query_call(
        canister_id,
        user,
        "get_workspace",
        encode_one(workspace_id).unwrap(),
    ).unwrap();

    let not_exists: Option<Workspace> = decode_one(&unwrap_wasm_result(get_after_delete)).unwrap();
    assert!(not_exists.is_none(), "Workspace should not exist after deletion");
}

#[test]
fn test_delete_workspace_unauthorized() {
    let (pic, canister_id, user_a) = setup();
    let user_b = Principal::from_slice(&[121, 122, 123, 124, 125, 126, 127, 128, 129, 130]);

    // User A creates a workspace
    let create_request = CreateWorkspaceRequest {
        name: "User A's Workspace".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user_a,
        "create_workspace",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Workspace, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let workspace_id = created.unwrap().id;

    // User B tries to delete User A's workspace
    let delete_response = pic.update_call(
        canister_id,
        user_b,
        "delete_workspace",
        encode_one(workspace_id).unwrap(),
    ).unwrap();

    let result: Result<Workspace, String> = decode_one(&unwrap_wasm_result(delete_response)).unwrap();
    assert!(result.is_err(), "User B should not be able to delete User A's workspace");
    assert!(result.unwrap_err().contains("Not authorized"), "Error should mention authorization");
}

// ============================================================================
// Task 6.3: Document Delete Tests (AC: 1.1.8.3)
// ============================================================================

#[test]
fn test_delete_document() {
    let (pic, canister_id, user) = setup();

    // Create a workspace first
    let workspace_request = CreateWorkspaceRequest {
        name: "Test Workspace".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    let workspace_response = pic.update_call(
        canister_id,
        user,
        "create_workspace",
        encode_one(workspace_request).unwrap(),
    ).unwrap();

    let workspace: Result<Workspace, String> = decode_one(&unwrap_wasm_result(workspace_response)).unwrap();
    let workspace_id = workspace.unwrap().id;

    // Create a document
    let doc_request = CreateDocumentRequest {
        workspace_id,
        title: "Document to Delete".to_string(),
        content: Some("Content".to_string()),
        template_id: None,
        parent_id: None,
    };

    let doc_response = pic.update_call(
        canister_id,
        user,
        "create_document",
        encode_one(doc_request).unwrap(),
    ).unwrap();

    let created: Result<Document, String> = decode_one(&unwrap_wasm_result(doc_response)).unwrap();
    let doc_id = created.unwrap().id;

    // Verify it exists
    let get_response = pic.query_call(
        canister_id,
        user,
        "get_document",
        encode_one(doc_id).unwrap(),
    ).unwrap();

    let exists: Option<Document> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert!(exists.is_some(), "Document should exist before deletion");

    // Delete the document
    let delete_response = pic.update_call(
        canister_id,
        user,
        "delete_document",
        encode_one(doc_id).unwrap(),
    ).unwrap();

    let delete_result: Result<Document, String> = decode_one(&unwrap_wasm_result(delete_response)).unwrap();
    assert!(delete_result.is_ok(), "Should delete document: {:?}", delete_result);

    // Verify it no longer exists
    let get_after_delete = pic.query_call(
        canister_id,
        user,
        "get_document",
        encode_one(doc_id).unwrap(),
    ).unwrap();

    let not_exists: Option<Document> = decode_one(&unwrap_wasm_result(get_after_delete)).unwrap();
    assert!(not_exists.is_none(), "Document should not exist after deletion");

    // Verify workspace documents list is updated
    let ws_docs_response = pic.query_call(
        canister_id,
        user,
        "get_workspace_documents",
        encode_one(workspace_id).unwrap(),
    ).unwrap();

    let ws_docs: Vec<Document> = decode_one(&unwrap_wasm_result(ws_docs_response)).unwrap();
    assert!(ws_docs.is_empty(), "Workspace should have no documents after deletion");
}

#[test]
fn test_delete_document_unauthorized() {
    let (pic, canister_id, user_a) = setup();
    let user_b = Principal::from_slice(&[131, 132, 133, 134, 135, 136, 137, 138, 139, 140]);

    // User A creates a workspace and document
    let workspace_request = CreateWorkspaceRequest {
        name: "User A's Workspace".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    let workspace_response = pic.update_call(
        canister_id,
        user_a,
        "create_workspace",
        encode_one(workspace_request).unwrap(),
    ).unwrap();

    let workspace: Result<Workspace, String> = decode_one(&unwrap_wasm_result(workspace_response)).unwrap();
    let workspace_id = workspace.unwrap().id;

    let doc_request = CreateDocumentRequest {
        workspace_id,
        title: "User A's Document".to_string(),
        content: None,
        template_id: None,
        parent_id: None,
    };

    let doc_response = pic.update_call(
        canister_id,
        user_a,
        "create_document",
        encode_one(doc_request).unwrap(),
    ).unwrap();

    let created: Result<Document, String> = decode_one(&unwrap_wasm_result(doc_response)).unwrap();
    let doc_id = created.unwrap().id;

    // User B tries to delete User A's document
    let delete_response = pic.update_call(
        canister_id,
        user_b,
        "delete_document",
        encode_one(doc_id).unwrap(),
    ).unwrap();

    let result: Result<Document, String> = decode_one(&unwrap_wasm_result(delete_response)).unwrap();
    assert!(result.is_err(), "User B should not be able to delete User A's document");
    assert!(result.unwrap_err().contains("Not authorized"), "Error should mention authorization");
}

// ============================================================================
// Task 6.4: Template Update/Delete Tests (AC: 1.1.8.4)
// ============================================================================

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct UpdateTemplateRequest {
    name: Option<String>,
    description: Option<String>,
    content: Option<String>,
    capture_type: Option<CaptureType>,
    default_fields: Option<DynamicFields>,
    is_public: Option<bool>,
}

#[test]
fn test_update_template() {
    let (pic, canister_id, user) = setup();

    // Create a template
    let create_request = CreateTemplateRequest {
        template_type: TemplateType::Capture,
        name: "Original Template".to_string(),
        description: Some("Original description".to_string()),
        content: "Original content".to_string(),
        capture_type: Some(CaptureType::Task),
        default_fields: None,
        is_public: Some(false),
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_template",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Template, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let template_id = created.unwrap().id;

    // Update the template
    let update_request = UpdateTemplateRequest {
        name: Some("Updated Template".to_string()),
        description: Some("Updated description".to_string()),
        content: Some("Updated content".to_string()),
        capture_type: Some(CaptureType::Idea),
        default_fields: None,
        is_public: Some(true),
    };

    let update_response = pic.update_call(
        canister_id,
        user,
        "update_template",
        encode_args((template_id, update_request)).unwrap(),
    ).unwrap();

    let result: Result<Template, String> = decode_one(&unwrap_wasm_result(update_response)).unwrap();
    assert!(result.is_ok(), "Should update template: {:?}", result);

    let updated = result.unwrap();
    assert_eq!(updated.name, "Updated Template");
    assert_eq!(updated.description, Some("Updated description".to_string()));
    assert_eq!(updated.content, "Updated content");
    assert_eq!(updated.capture_type, Some(CaptureType::Idea));
    assert!(updated.is_public);
}

#[test]
fn test_delete_template() {
    let (pic, canister_id, user) = setup();

    // Create a template
    let create_request = CreateTemplateRequest {
        template_type: TemplateType::Document,
        name: "Template to Delete".to_string(),
        description: None,
        content: "Content".to_string(),
        capture_type: None,
        default_fields: None,
        is_public: Some(true),
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_template",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Template, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let template_id = created.unwrap().id;

    // Verify it exists
    let get_response = pic.query_call(
        canister_id,
        user,
        "get_template",
        encode_one(template_id).unwrap(),
    ).unwrap();

    let exists: Option<Template> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert!(exists.is_some(), "Template should exist before deletion");

    // Verify it's in public templates
    let public_before = pic.query_call(
        canister_id,
        Principal::anonymous(),
        "get_public_templates",
        encode_one(()).unwrap(),
    ).unwrap();

    let public_templates_before: Vec<Template> = decode_one(&unwrap_wasm_result(public_before)).unwrap();
    assert!(public_templates_before.iter().any(|t| t.id == template_id), "Template should be in public list");

    // Delete the template
    let delete_response = pic.update_call(
        canister_id,
        user,
        "delete_template",
        encode_one(template_id).unwrap(),
    ).unwrap();

    let delete_result: Result<Template, String> = decode_one(&unwrap_wasm_result(delete_response)).unwrap();
    assert!(delete_result.is_ok(), "Should delete template: {:?}", delete_result);

    // Verify it no longer exists
    let get_after_delete = pic.query_call(
        canister_id,
        user,
        "get_template",
        encode_one(template_id).unwrap(),
    ).unwrap();

    let not_exists: Option<Template> = decode_one(&unwrap_wasm_result(get_after_delete)).unwrap();
    assert!(not_exists.is_none(), "Template should not exist after deletion");

    // Verify it's removed from public templates
    let public_after = pic.query_call(
        canister_id,
        Principal::anonymous(),
        "get_public_templates",
        encode_one(()).unwrap(),
    ).unwrap();

    let public_templates_after: Vec<Template> = decode_one(&unwrap_wasm_result(public_after)).unwrap();
    assert!(!public_templates_after.iter().any(|t| t.id == template_id), "Template should be removed from public list");
}

#[test]
fn test_delete_template_unauthorized() {
    let (pic, canister_id, user_a) = setup();
    let user_b = Principal::from_slice(&[141, 142, 143, 144, 145, 146, 147, 148, 149, 150]);

    // User A creates a template
    let create_request = CreateTemplateRequest {
        template_type: TemplateType::Capture,
        name: "User A's Template".to_string(),
        description: None,
        content: "Content".to_string(),
        capture_type: None,
        default_fields: None,
        is_public: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user_a,
        "create_template",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Template, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let template_id = created.unwrap().id;

    // User B tries to delete User A's template
    let delete_response = pic.update_call(
        canister_id,
        user_b,
        "delete_template",
        encode_one(template_id).unwrap(),
    ).unwrap();

    let result: Result<Template, String> = decode_one(&unwrap_wasm_result(delete_response)).unwrap();
    assert!(result.is_err(), "User B should not be able to delete User A's template");
    assert!(result.unwrap_err().contains("Not authorized"), "Error should mention authorization");
}

#[test]
fn test_template_public_toggle() {
    let (pic, canister_id, user) = setup();

    // Create a private template
    let create_request = CreateTemplateRequest {
        template_type: TemplateType::Document,
        name: "Toggle Template".to_string(),
        description: None,
        content: "Content".to_string(),
        capture_type: None,
        default_fields: None,
        is_public: Some(false),
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_template",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Template, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let template = created.unwrap();
    assert!(!template.is_public);

    // Verify not in public list
    let public_before = pic.query_call(
        canister_id,
        Principal::anonymous(),
        "get_public_templates",
        encode_one(()).unwrap(),
    ).unwrap();

    let public_templates_before: Vec<Template> = decode_one(&unwrap_wasm_result(public_before)).unwrap();
    assert!(!public_templates_before.iter().any(|t| t.id == template.id));

    // Make it public
    let update_request = UpdateTemplateRequest {
        name: None,
        description: None,
        content: None,
        capture_type: None,
        default_fields: None,
        is_public: Some(true),
    };

    let update_response = pic.update_call(
        canister_id,
        user,
        "update_template",
        encode_args((template.id, update_request)).unwrap(),
    ).unwrap();

    let updated: Result<Template, String> = decode_one(&unwrap_wasm_result(update_response)).unwrap();
    assert!(updated.unwrap().is_public);

    // Verify now in public list
    let public_after = pic.query_call(
        canister_id,
        Principal::anonymous(),
        "get_public_templates",
        encode_one(()).unwrap(),
    ).unwrap();

    let public_templates_after: Vec<Template> = decode_one(&unwrap_wasm_result(public_after)).unwrap();
    assert!(public_templates_after.iter().any(|t| t.id == template.id));
}

// ============================================================================
// Task 3: Capture Update/Delete Tests (AC: 1.1.7.1)
// ============================================================================

#[test]
fn test_update_capture() {
    let (pic, canister_id, user) = setup();

    // Create a capture
    let create_request = CreateCaptureRequest {
        capture_type: CaptureType::Task,
        title: "Original Task".to_string(),
        description: Some("Original description".to_string()),
        content: None,
        priority: Some(Priority::Low),
        fields: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_capture",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Capture, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let capture_id = created.unwrap().id;

    // Update the capture
    let update_request = UpdateCaptureRequest {
        id: capture_id,
        title: Some("Updated Task".to_string()),
        description: Some("Updated description".to_string()),
        content: Some("New content".to_string()),
        priority: Some(Priority::High),
        status: Some(CaptureStatus::InProgress),
        fields: None,
    };

    let update_response = pic.update_call(
        canister_id,
        user,
        "update_capture",
        encode_one(update_request).unwrap(),
    ).unwrap();

    let result: Result<Capture, String> = decode_one(&unwrap_wasm_result(update_response)).unwrap();
    assert!(result.is_ok(), "Should update capture: {:?}", result);

    let updated = result.unwrap();
    assert_eq!(updated.title, "Updated Task");
    assert_eq!(updated.description, Some("Updated description".to_string()));
    assert_eq!(updated.content, Some("New content".to_string()));
    assert_eq!(updated.priority, Priority::High);
    assert_eq!(updated.status, CaptureStatus::InProgress);
}

#[test]
fn test_delete_capture() {
    let (pic, canister_id, user) = setup();

    // Create a capture
    let create_request = CreateCaptureRequest {
        capture_type: CaptureType::Idea,
        title: "To Be Deleted".to_string(),
        description: None,
        content: None,
        priority: None,
        fields: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_capture",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let created: Result<Capture, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let capture_id = created.unwrap().id;

    // Verify it exists
    let get_response = pic.query_call(
        canister_id,
        user,
        "get_capture",
        encode_one(capture_id).unwrap(),
    ).unwrap();

    let exists: Option<Capture> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert!(exists.is_some(), "Capture should exist before deletion");

    // Delete the capture
    let delete_response = pic.update_call(
        canister_id,
        user,
        "delete_capture",
        encode_one(capture_id).unwrap(),
    ).unwrap();

    let delete_result: Result<Capture, String> = decode_one(&unwrap_wasm_result(delete_response)).unwrap();
    assert!(delete_result.is_ok(), "Should delete capture: {:?}", delete_result);

    // Verify it no longer exists
    let get_after_delete = pic.query_call(
        canister_id,
        user,
        "get_capture",
        encode_one(capture_id).unwrap(),
    ).unwrap();

    let not_exists: Option<Capture> = decode_one(&unwrap_wasm_result(get_after_delete)).unwrap();
    assert!(not_exists.is_none(), "Capture should not exist after deletion");
}

#[test]
fn test_capture_all_types() {
    let (pic, canister_id, user) = setup();

    let capture_types = vec![
        CaptureType::Idea,
        CaptureType::Task,
        CaptureType::Project,
        CaptureType::Reflection,
        CaptureType::Outline,
        CaptureType::Calendar,
    ];

    for capture_type in capture_types {
        let request = CreateCaptureRequest {
            capture_type: capture_type.clone(),
            title: format!("{:?} capture", capture_type),
            description: None,
            content: None,
            priority: None,
            fields: None,
        };

        let response = pic.update_call(
            canister_id,
            user,
            "create_capture",
            encode_one(request).unwrap(),
        ).unwrap();

        let result: Result<Capture, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
        assert!(result.is_ok(), "Should create {:?} capture: {:?}", capture_type, result);

        let capture = result.unwrap();
        assert_eq!(capture.capture_type, capture_type, "Capture type should match");
    }

    // Verify all 6 captures were created
    let stats_response = pic.query_call(
        canister_id,
        Principal::anonymous(),
        "get_stats",
        encode_one(()).unwrap(),
    ).unwrap();

    let stats: Stats = decode_one(&unwrap_wasm_result(stats_response)).unwrap();
    assert_eq!(stats.total_captures, 6, "Should have created 6 captures (one of each type)");
}

// ============================================================================
// Task 4: Sprint Update/Delete Tests (AC: 1.1.7.2)
// Note: update_sprint and delete_sprint are not implemented in lib.rs yet
// These tests verify the existing functionality
// ============================================================================

#[test]
fn test_get_my_sprints() {
    let (pic, canister_id, user) = setup();
    let user_b = Principal::from_slice(&[51, 52, 53, 54, 55, 56, 57, 58, 59, 60]);

    // User A creates sprints
    for i in 0..2 {
        let request = CreateSprintRequest {
            name: format!("User A Sprint {}", i),
            goal: None,
            start_date: 0,
            end_date: 100,
            capacity: None,
        };

        pic.update_call(
            canister_id,
            user,
            "create_sprint",
            encode_one(request).unwrap(),
        ).unwrap();
    }

    // User B creates a sprint
    let request = CreateSprintRequest {
        name: "User B Sprint".to_string(),
        goal: None,
        start_date: 0,
        end_date: 100,
        capacity: None,
    };

    pic.update_call(
        canister_id,
        user_b,
        "create_sprint",
        encode_one(request).unwrap(),
    ).unwrap();

    // User A should only see their 2 sprints
    let response_a = pic.query_call(
        canister_id,
        user,
        "get_my_sprints",
        encode_one(()).unwrap(),
    ).unwrap();

    let sprints_a: Vec<Sprint> = decode_one(&unwrap_wasm_result(response_a)).unwrap();
    assert_eq!(sprints_a.len(), 2, "User A should have 2 sprints");

    // User B should only see their 1 sprint
    let response_b = pic.query_call(
        canister_id,
        user_b,
        "get_my_sprints",
        encode_one(()).unwrap(),
    ).unwrap();

    let sprints_b: Vec<Sprint> = decode_one(&unwrap_wasm_result(response_b)).unwrap();
    assert_eq!(sprints_b.len(), 1, "User B should have 1 sprint");
}

#[test]
fn test_remove_capture_from_sprint() {
    let (pic, canister_id, user) = setup();

    // Create a capture
    let capture_request = CreateCaptureRequest {
        capture_type: CaptureType::Task,
        title: "Sprint Task".to_string(),
        description: None,
        content: None,
        priority: None,
        fields: None,
    };

    let capture_response = pic.update_call(
        canister_id,
        user,
        "create_capture",
        encode_one(capture_request).unwrap(),
    ).unwrap();

    let capture: Result<Capture, String> = decode_one(&unwrap_wasm_result(capture_response)).unwrap();
    let capture_id = capture.unwrap().id;

    // Create a sprint
    let sprint_request = CreateSprintRequest {
        name: "Test Sprint".to_string(),
        goal: None,
        start_date: 0,
        end_date: 100,
        capacity: None,
    };

    let sprint_response = pic.update_call(
        canister_id,
        user,
        "create_sprint",
        encode_one(sprint_request).unwrap(),
    ).unwrap();

    let sprint: Result<Sprint, String> = decode_one(&unwrap_wasm_result(sprint_response)).unwrap();
    let sprint_id = sprint.unwrap().id;

    // Add capture to sprint
    let add_response = pic.update_call(
        canister_id,
        user,
        "add_capture_to_sprint",
        encode_args((sprint_id, capture_id)).unwrap(),
    ).unwrap();

    let add_result: Result<(), String> = decode_one(&unwrap_wasm_result(add_response)).unwrap();
    assert!(add_result.is_ok());

    // Verify capture is in sprint
    let get_sprint_response = pic.query_call(
        canister_id,
        user,
        "get_sprint",
        encode_one(sprint_id).unwrap(),
    ).unwrap();

    let sprint_with_capture: Option<Sprint> = decode_one(&unwrap_wasm_result(get_sprint_response)).unwrap();
    assert!(sprint_with_capture.unwrap().capture_ids.contains(&capture_id));

    // Remove capture from sprint
    let remove_response = pic.update_call(
        canister_id,
        user,
        "remove_capture_from_sprint",
        encode_args((sprint_id, capture_id)).unwrap(),
    ).unwrap();

    let remove_result: Result<(), String> = decode_one(&unwrap_wasm_result(remove_response)).unwrap();
    assert!(remove_result.is_ok(), "Should remove capture from sprint: {:?}", remove_result);

    // Verify capture is no longer in sprint
    let get_sprint_after = pic.query_call(
        canister_id,
        user,
        "get_sprint",
        encode_one(sprint_id).unwrap(),
    ).unwrap();

    let sprint_after: Option<Sprint> = decode_one(&unwrap_wasm_result(get_sprint_after)).unwrap();
    assert!(!sprint_after.unwrap().capture_ids.contains(&capture_id), "Capture should be removed from sprint");
}

// ============================================================================
// Task 5: Workspace Update/Delete Tests (AC: 1.1.7.3)
// Note: update_workspace and delete_workspace are not implemented in lib.rs yet
// ============================================================================

#[test]
fn test_get_my_workspaces() {
    let (pic, canister_id, user) = setup();
    let user_b = Principal::from_slice(&[61, 62, 63, 64, 65, 66, 67, 68, 69, 70]);

    // User A creates workspaces
    for i in 0..3 {
        let request = CreateWorkspaceRequest {
            name: format!("User A Workspace {}", i),
            description: None,
            icon: None,
            parent_id: None,
        };

        pic.update_call(
            canister_id,
            user,
            "create_workspace",
            encode_one(request).unwrap(),
        ).unwrap();
    }

    // User B creates a workspace
    let request = CreateWorkspaceRequest {
        name: "User B Workspace".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    pic.update_call(
        canister_id,
        user_b,
        "create_workspace",
        encode_one(request).unwrap(),
    ).unwrap();

    // User A should only see their 3 workspaces
    let response_a = pic.query_call(
        canister_id,
        user,
        "get_my_workspaces",
        encode_one(()).unwrap(),
    ).unwrap();

    let workspaces_a: Vec<Workspace> = decode_one(&unwrap_wasm_result(response_a)).unwrap();
    assert_eq!(workspaces_a.len(), 3, "User A should have 3 workspaces");

    // User B should only see their 1 workspace
    let response_b = pic.query_call(
        canister_id,
        user_b,
        "get_my_workspaces",
        encode_one(()).unwrap(),
    ).unwrap();

    let workspaces_b: Vec<Workspace> = decode_one(&unwrap_wasm_result(response_b)).unwrap();
    assert_eq!(workspaces_b.len(), 1, "User B should have 1 workspace");
}

// ============================================================================
// Task 6: Comprehensive Data Isolation Tests (AC: 1.1.7.5)
// ============================================================================

#[test]
fn test_sprint_data_isolation() {
    let (pic, canister_id, user_a) = setup();
    let user_b = Principal::from_slice(&[71, 72, 73, 74, 75, 76, 77, 78, 79, 80]);

    // User A creates a sprint
    let sprint_request = CreateSprintRequest {
        name: "User A's Sprint".to_string(),
        goal: Some("Private goal".to_string()),
        start_date: 0,
        end_date: 100,
        capacity: Some(20),
    };

    let create_response = pic.update_call(
        canister_id,
        user_a,
        "create_sprint",
        encode_one(sprint_request).unwrap(),
    ).unwrap();

    let created: Result<Sprint, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let sprint_id = created.unwrap().id;

    // User A creates a capture to add to sprint
    let capture_request = CreateCaptureRequest {
        capture_type: CaptureType::Task,
        title: "User A's Task".to_string(),
        description: None,
        content: None,
        priority: None,
        fields: None,
    };

    let capture_response = pic.update_call(
        canister_id,
        user_a,
        "create_capture",
        encode_one(capture_request).unwrap(),
    ).unwrap();

    let capture: Result<Capture, String> = decode_one(&unwrap_wasm_result(capture_response)).unwrap();
    let capture_id = capture.unwrap().id;

    // Add capture to sprint
    pic.update_call(
        canister_id,
        user_a,
        "add_capture_to_sprint",
        encode_args((sprint_id, capture_id)).unwrap(),
    ).unwrap();

    // User B should not see User A's sprints
    let sprints_b = pic.query_call(
        canister_id,
        user_b,
        "get_my_sprints",
        encode_one(()).unwrap(),
    ).unwrap();

    let sprints: Vec<Sprint> = decode_one(&unwrap_wasm_result(sprints_b)).unwrap();
    assert_eq!(sprints.len(), 0, "User B should not see User A's sprints");

    // User B should not be able to add their capture to User A's sprint
    let capture_b_request = CreateCaptureRequest {
        capture_type: CaptureType::Task,
        title: "User B's Task".to_string(),
        description: None,
        content: None,
        priority: None,
        fields: None,
    };

    let capture_b_response = pic.update_call(
        canister_id,
        user_b,
        "create_capture",
        encode_one(capture_b_request).unwrap(),
    ).unwrap();

    let capture_b: Result<Capture, String> = decode_one(&unwrap_wasm_result(capture_b_response)).unwrap();
    let capture_b_id = capture_b.unwrap().id;

    let add_response = pic.update_call(
        canister_id,
        user_b,
        "add_capture_to_sprint",
        encode_args((sprint_id, capture_b_id)).unwrap(),
    ).unwrap();

    let add_result: Result<(), String> = decode_one(&unwrap_wasm_result(add_response)).unwrap();
    assert!(add_result.is_err(), "User B should not be able to add capture to User A's sprint");
}

#[test]
fn test_workspace_data_isolation() {
    let (pic, canister_id, user_a) = setup();
    let user_b = Principal::from_slice(&[81, 82, 83, 84, 85, 86, 87, 88, 89, 90]);

    // User A creates a workspace
    let workspace_request = CreateWorkspaceRequest {
        name: "User A's Workspace".to_string(),
        description: Some("Private workspace".to_string()),
        icon: None,
        parent_id: None,
    };

    let create_response = pic.update_call(
        canister_id,
        user_a,
        "create_workspace",
        encode_one(workspace_request).unwrap(),
    ).unwrap();

    let created: Result<Workspace, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let workspace_id = created.unwrap().id;

    // User B should not see User A's workspaces
    let workspaces_b = pic.query_call(
        canister_id,
        user_b,
        "get_my_workspaces",
        encode_one(()).unwrap(),
    ).unwrap();

    let workspaces: Vec<Workspace> = decode_one(&unwrap_wasm_result(workspaces_b)).unwrap();
    assert_eq!(workspaces.len(), 0, "User B should not see User A's workspaces");

    // User B should not be able to create documents in User A's workspace
    let doc_request = CreateDocumentRequest {
        workspace_id,
        title: "Unauthorized Document".to_string(),
        content: None,
        template_id: None,
        parent_id: None,
    };

    let doc_response = pic.update_call(
        canister_id,
        user_b,
        "create_document",
        encode_one(doc_request).unwrap(),
    ).unwrap();

    let doc_result: Result<Document, String> = decode_one(&unwrap_wasm_result(doc_response)).unwrap();
    assert!(doc_result.is_err(), "User B should not be able to create document in User A's workspace");
}

#[test]
fn test_document_data_isolation() {
    let (pic, canister_id, user_a) = setup();
    let user_b = Principal::from_slice(&[91, 92, 93, 94, 95, 96, 97, 98, 99, 100]);

    // User A creates a workspace and document
    let workspace_request = CreateWorkspaceRequest {
        name: "User A's Workspace".to_string(),
        description: None,
        icon: None,
        parent_id: None,
    };

    let workspace_response = pic.update_call(
        canister_id,
        user_a,
        "create_workspace",
        encode_one(workspace_request).unwrap(),
    ).unwrap();

    let workspace: Result<Workspace, String> = decode_one(&unwrap_wasm_result(workspace_response)).unwrap();
    let workspace_id = workspace.unwrap().id;

    let doc_request = CreateDocumentRequest {
        workspace_id,
        title: "User A's Document".to_string(),
        content: Some("Private content".to_string()),
        template_id: None,
        parent_id: None,
    };

    let doc_response = pic.update_call(
        canister_id,
        user_a,
        "create_document",
        encode_one(doc_request).unwrap(),
    ).unwrap();

    let created: Result<Document, String> = decode_one(&unwrap_wasm_result(doc_response)).unwrap();
    let doc_id = created.unwrap().id;

    // User B should not be able to get documents from User A's workspace
    let ws_docs_response = pic.query_call(
        canister_id,
        user_b,
        "get_workspace_documents",
        encode_one(workspace_id).unwrap(),
    ).unwrap();

    let ws_docs: Vec<Document> = decode_one(&unwrap_wasm_result(ws_docs_response)).unwrap();
    assert_eq!(ws_docs.len(), 0, "User B should not see User A's workspace documents");

    // User B should not be able to update User A's document
    let update_response = pic.update_call(
        canister_id,
        user_b,
        "update_document",
        encode_args((doc_id, Some("Hacked Title".to_string()), None::<String>)).unwrap(),
    ).unwrap();

    let update_result: Result<Document, String> = decode_one(&unwrap_wasm_result(update_response)).unwrap();
    assert!(update_result.is_err(), "User B should not be able to update User A's document");
}

#[test]
fn test_template_data_isolation() {
    let (pic, canister_id, user_a) = setup();
    let user_b = Principal::from_slice(&[101, 102, 103, 104, 105, 106, 107, 108, 109, 110]);

    // User A creates a private template
    let private_template = CreateTemplateRequest {
        template_type: TemplateType::Capture,
        name: "User A's Private Template".to_string(),
        description: None,
        content: "Private template content".to_string(),
        capture_type: None,
        default_fields: None,
        is_public: Some(false),
    };

    let create_response = pic.update_call(
        canister_id,
        user_a,
        "create_template",
        encode_one(private_template).unwrap(),
    ).unwrap();

    let _created: Result<Template, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();

    // User A creates a public template
    let public_template = CreateTemplateRequest {
        template_type: TemplateType::Document,
        name: "User A's Public Template".to_string(),
        description: None,
        content: "Public template content".to_string(),
        capture_type: None,
        default_fields: None,
        is_public: Some(true),
    };

    pic.update_call(
        canister_id,
        user_a,
        "create_template",
        encode_one(public_template).unwrap(),
    ).unwrap();

    // User B should not see User A's private templates in get_my_templates
    let templates_b = pic.query_call(
        canister_id,
        user_b,
        "get_my_templates",
        encode_one(()).unwrap(),
    ).unwrap();

    let templates: Vec<Template> = decode_one(&unwrap_wasm_result(templates_b)).unwrap();
    assert_eq!(templates.len(), 0, "User B should not see User A's templates in get_my_templates");

    // But User B should see User A's public template in get_public_templates
    let public_templates = pic.query_call(
        canister_id,
        user_b,
        "get_public_templates",
        encode_one(()).unwrap(),
    ).unwrap();

    let public: Vec<Template> = decode_one(&unwrap_wasm_result(public_templates)).unwrap();
    assert_eq!(public.len(), 1, "User B should see 1 public template");
    assert!(public[0].is_public, "Template should be public");
}

// ============================================================================
// Discussion Types (Story FOS-4.1.2)
// ============================================================================

#[derive(CandidType, Clone, Serialize, Deserialize, Debug, PartialEq)]
enum ProposalCategory {
    Constitutional,
    Operational,
    Treasury,
    SoftwareDevelopment,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug, PartialEq)]
enum DiscussionStage {
    Brainstorm,
    Refining,
    Ready,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug, PartialEq)]
enum AuthorType {
    Human,
    Agent { agent_id: String },
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug)]
struct Discussion {
    id: u64,
    title: String,
    description: String,
    category: ProposalCategory,
    proposer: Principal,
    contributors: Vec<Principal>,
    stage: DiscussionStage,
    created_at: u64,
    stage_changed_at: u64,
    comment_count: u64,
    participant_count: u64,
    is_archived: bool,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug)]
struct Comment {
    id: u64,
    discussion_id: u64,
    author: Principal,
    content: String,
    author_type: AuthorType,
    created_at: u64,
    is_retracted: bool,
    retracted_at: Option<u64>,
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct CreateDiscussionArgs {
    title: String,
    description: String,
    category: ProposalCategory,
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
struct AddCommentArgs {
    discussion_id: u64,
    content: String,
    author_type: AuthorType,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug, Default)]
struct DiscussionFilter {
    stage: Option<DiscussionStage>,
    category: Option<ProposalCategory>,
    proposer: Option<Principal>,
    include_archived: Option<bool>,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug, Default)]
struct DiscussionPaginationParams {
    offset: Option<u64>,
    limit: Option<u64>,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug)]
struct PaginatedDiscussionResponse {
    items: Vec<Discussion>,
    total: u64,
    offset: u64,
    limit: u64,
}

#[derive(CandidType, Clone, Serialize, Deserialize, Debug)]
struct QualityGateStatus {
    participants_met: bool,
    participants_count: u64,
    comments_met: bool,
    substantive_comments: u64,
    duration_met: bool,
    hours_in_refining: u64,
    all_met: bool,
}

// ============================================================================
// Discussion Tests - Story FOS-4.1.2
// AC-4.1.2.1: Discussion threads can be created
// ============================================================================

#[test]
fn test_fos_4_1_2_create_discussion() {
    let (pic, canister_id, user) = setup();

    let request = CreateDiscussionArgs {
        title: "Test Governance Proposal".to_string(),
        description: "This is a test description for a governance proposal discussion.".to_string(),
        category: ProposalCategory::Operational,
    };

    let response = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(request).unwrap(),
    ).unwrap();

    let result: Result<u64, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_ok(), "Discussion creation should succeed");
    assert_eq!(result.unwrap(), 1, "First discussion should have ID 1");

    // Verify we can retrieve the discussion
    let get_response = pic.query_call(
        canister_id,
        user,
        "get_discussion",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let discussion: Option<Discussion> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert!(discussion.is_some(), "Discussion should exist");

    let d = discussion.unwrap();
    assert_eq!(d.title, "Test Governance Proposal");
    assert_eq!(d.stage, DiscussionStage::Brainstorm);
    assert_eq!(d.proposer, user);
    assert_eq!(d.participant_count, 1); // Proposer counts as first participant
}

#[test]
fn test_fos_4_1_2_create_discussion_empty_title_rejected() {
    let (pic, canister_id, user) = setup();

    let request = CreateDiscussionArgs {
        title: "".to_string(),
        description: "Description".to_string(),
        category: ProposalCategory::Operational,
    };

    let response = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(request).unwrap(),
    ).unwrap();

    let result: Result<u64, String> = decode_one(&unwrap_wasm_result(response)).unwrap();
    assert!(result.is_err(), "Empty title should be rejected");
    assert!(result.unwrap_err().contains("empty"), "Error should mention empty title");
}

// ============================================================================
// AC-4.1.2.2: Comments are append-only
// ============================================================================

#[test]
fn test_fos_4_1_2_add_comment() {
    let (pic, canister_id, user) = setup();

    // Create discussion first
    let create_request = CreateDiscussionArgs {
        title: "Discussion for Comments".to_string(),
        description: "Testing comments".to_string(),
        category: ProposalCategory::Operational,
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let discussion_id: Result<u64, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    let discussion_id = discussion_id.unwrap();

    // Add a comment
    let comment_request = AddCommentArgs {
        discussion_id,
        content: "This is a test comment with sufficient length for testing purposes.".to_string(),
        author_type: AuthorType::Human,
    };

    let comment_response = pic.update_call(
        canister_id,
        user,
        "add_comment",
        encode_one(comment_request).unwrap(),
    ).unwrap();

    let comment_result: Result<u64, String> = decode_one(&unwrap_wasm_result(comment_response)).unwrap();
    assert!(comment_result.is_ok(), "Comment should be added successfully");
    assert_eq!(comment_result.unwrap(), 1, "First comment should have ID 1");

    // Verify comment is retrievable
    let get_comments_response = pic.query_call(
        canister_id,
        user,
        "get_comments",
        encode_args((discussion_id, 0u64, 10u64)).unwrap(),
    ).unwrap();

    let comments: Vec<Comment> = decode_one(&unwrap_wasm_result(get_comments_response)).unwrap();
    assert_eq!(comments.len(), 1);
    assert!(!comments[0].is_retracted);
}

#[test]
fn test_fos_4_1_2_retract_comment() {
    let (pic, canister_id, user) = setup();

    // Create discussion and add comment
    let create_request = CreateDiscussionArgs {
        title: "Discussion for Retraction".to_string(),
        description: "Testing retraction".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let comment_request = AddCommentArgs {
        discussion_id: 1,
        content: "This comment will be retracted".to_string(),
        author_type: AuthorType::Human,
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "add_comment",
        encode_one(comment_request).unwrap(),
    ).unwrap();

    // Retract the comment
    let retract_response = pic.update_call(
        canister_id,
        user,
        "retract_comment",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let retract_result: Result<(), String> = decode_one(&unwrap_wasm_result(retract_response)).unwrap();
    assert!(retract_result.is_ok(), "Retraction should succeed");

    // Verify comment is marked as retracted but still exists
    let get_comments_response = pic.query_call(
        canister_id,
        user,
        "get_comments",
        encode_args((1u64, 0u64, 10u64)).unwrap(),
    ).unwrap();

    let comments: Vec<Comment> = decode_one(&unwrap_wasm_result(get_comments_response)).unwrap();
    assert_eq!(comments.len(), 1, "Comment should still exist");
    assert!(comments[0].is_retracted, "Comment should be marked retracted");
    assert!(comments[0].retracted_at.is_some(), "Retraction time should be set");
}

#[test]
fn test_fos_4_1_2_retract_someone_elses_comment_rejected() {
    let (pic, canister_id, user_a) = setup();
    let user_b = Principal::from_slice(&[201, 202, 203, 204, 205, 206, 207, 208, 209, 210]);

    // User A creates discussion and adds comment
    let create_request = CreateDiscussionArgs {
        title: "Discussion".to_string(),
        description: "Description".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        user_a,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let comment_request = AddCommentArgs {
        discussion_id: 1,
        content: "User A's comment".to_string(),
        author_type: AuthorType::Human,
    };

    let _ = pic.update_call(
        canister_id,
        user_a,
        "add_comment",
        encode_one(comment_request).unwrap(),
    ).unwrap();

    // User B tries to retract User A's comment
    let retract_response = pic.update_call(
        canister_id,
        user_b,
        "retract_comment",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let retract_result: Result<(), String> = decode_one(&unwrap_wasm_result(retract_response)).unwrap();
    assert!(retract_result.is_err(), "Should not be able to retract someone else's comment");
}

// ============================================================================
// AC-4.1.2.3: Stage transitions
// ============================================================================

#[test]
fn test_fos_4_1_2_stage_transition_brainstorm_to_refining() {
    let (pic, canister_id, user) = setup();

    // Create discussion in Brainstorm stage
    let create_request = CreateDiscussionArgs {
        title: "Stage Transition Test".to_string(),
        description: "Testing stage transitions".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    // Advance to Refining
    let advance_response = pic.update_call(
        canister_id,
        user,
        "advance_stage",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let result: Result<DiscussionStage, String> = decode_one(&unwrap_wasm_result(advance_response)).unwrap();
    assert!(result.is_ok(), "Stage advance should succeed");
    assert_eq!(result.unwrap(), DiscussionStage::Refining);

    // Verify discussion is now in Refining stage
    let get_response = pic.query_call(
        canister_id,
        user,
        "get_discussion",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let discussion: Option<Discussion> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert_eq!(discussion.unwrap().stage, DiscussionStage::Refining);
}

// ============================================================================
// AC-4.1.2.4: Contributor invites
// ============================================================================

#[test]
fn test_fos_4_1_2_invite_contributor() {
    let (pic, canister_id, proposer) = setup();
    let contributor = Principal::from_slice(&[211, 212, 213, 214, 215, 216, 217, 218, 219, 220]);

    // Create discussion
    let create_request = CreateDiscussionArgs {
        title: "Contributor Test".to_string(),
        description: "Testing contributors".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        proposer,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    // Invite contributor
    let invite_response = pic.update_call(
        canister_id,
        proposer,
        "invite_contributor",
        encode_args((1u64, contributor)).unwrap(),
    ).unwrap();

    let invite_result: Result<(), String> = decode_one(&unwrap_wasm_result(invite_response)).unwrap();
    assert!(invite_result.is_ok(), "Invite should succeed");

    // Contributor accepts invite
    let accept_response = pic.update_call(
        canister_id,
        contributor,
        "respond_to_invite",
        encode_args((1u64, true)).unwrap(),
    ).unwrap();

    let accept_result: Result<(), String> = decode_one(&unwrap_wasm_result(accept_response)).unwrap();
    assert!(accept_result.is_ok(), "Accept should succeed");

    // Verify contributor is added to the discussion
    let get_response = pic.query_call(
        canister_id,
        proposer,
        "get_discussion",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let discussion: Option<Discussion> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert!(discussion.unwrap().contributors.contains(&contributor), "Contributor should be in list");
}

// ============================================================================
// AC-4.1.2.5: Quality gates
// ============================================================================

#[test]
fn test_fos_4_1_2_quality_gate_status() {
    let (pic, canister_id, user) = setup();

    // Create discussion
    let create_request = CreateDiscussionArgs {
        title: "Quality Gate Test".to_string(),
        description: "Testing quality gates".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    // Check quality gate status
    let gate_response = pic.query_call(
        canister_id,
        user,
        "get_quality_gate_status",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let gate_status: Option<QualityGateStatus> = decode_one(&unwrap_wasm_result(gate_response)).unwrap();
    assert!(gate_status.is_some(), "Gate status should be returned");

    let status = gate_status.unwrap();
    assert!(!status.all_met, "Gates should not be met initially");
    assert!(!status.participants_met, "Need 3+ participants");
    assert!(!status.comments_met, "Need 5+ substantive comments");
}

#[test]
fn test_fos_4_1_2_quality_gates_prevent_ready_without_meeting_thresholds() {
    let (pic, canister_id, user) = setup();

    // Create discussion and advance to Refining
    let create_request = CreateDiscussionArgs {
        title: "Quality Gate Block Test".to_string(),
        description: "Testing quality gate enforcement".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    // Advance to Refining (no quality gates for this transition)
    let _ = pic.update_call(
        canister_id,
        user,
        "advance_stage",
        encode_one(1u64).unwrap(),
    ).unwrap();

    // Try to advance to Ready (should fail - quality gates not met)
    let advance_response = pic.update_call(
        canister_id,
        user,
        "advance_stage",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let result: Result<DiscussionStage, String> = decode_one(&unwrap_wasm_result(advance_response)).unwrap();
    assert!(result.is_err(), "Should not be able to advance to Ready without meeting quality gates");
    assert!(result.unwrap_err().contains("Quality gates"), "Error should mention quality gates");
}

// ============================================================================
// AC-4.1.2.6: Agent comments tagged distinctly
// ============================================================================

#[test]
fn test_fos_4_1_2_agent_comments_tagged_distinctly() {
    let (pic, canister_id, user) = setup();

    // Create discussion
    let create_request = CreateDiscussionArgs {
        title: "Agent Comment Test".to_string(),
        description: "Testing agent comment tagging".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    // Add agent comment
    let agent_comment = AddCommentArgs {
        discussion_id: 1,
        content: "This is an AI-generated comment providing analysis of the proposal.".to_string(),
        author_type: AuthorType::Agent { agent_id: "gpt-4".to_string() },
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "add_comment",
        encode_one(agent_comment).unwrap(),
    ).unwrap();

    // Verify comment is tagged as Agent
    let get_comments_response = pic.query_call(
        canister_id,
        user,
        "get_comments",
        encode_args((1u64, 0u64, 10u64)).unwrap(),
    ).unwrap();

    let comments: Vec<Comment> = decode_one(&unwrap_wasm_result(get_comments_response)).unwrap();
    assert_eq!(comments.len(), 1);
    match &comments[0].author_type {
        AuthorType::Agent { agent_id } => assert_eq!(agent_id, "gpt-4"),
        AuthorType::Human => panic!("Comment should be tagged as Agent, not Human"),
    }

    // Verify agent comment doesn't count toward participant count
    let get_discussion_response = pic.query_call(
        canister_id,
        user,
        "get_discussion",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let discussion: Option<Discussion> = decode_one(&unwrap_wasm_result(get_discussion_response)).unwrap();
    // Only proposer should count - agent doesn't add to participant count
    assert_eq!(discussion.unwrap().participant_count, 1, "Agent should not increase participant count");
}

// ============================================================================
// AC-4.1.2.7: Discussion hash for verification
// ============================================================================

#[test]
fn test_fos_4_1_2_discussion_hash_generated() {
    let (pic, canister_id, user) = setup();

    // Create discussion with some comments
    let create_request = CreateDiscussionArgs {
        title: "Hash Test Discussion".to_string(),
        description: "Testing hash generation".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let comment = AddCommentArgs {
        discussion_id: 1,
        content: "A comment for hash testing".to_string(),
        author_type: AuthorType::Human,
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "add_comment",
        encode_one(comment).unwrap(),
    ).unwrap();

    // Get hash
    let hash_response = pic.query_call(
        canister_id,
        user,
        "get_discussion_hash",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let hash: Option<String> = decode_one(&unwrap_wasm_result(hash_response)).unwrap();
    assert!(hash.is_some(), "Hash should be returned");

    let hash_value = hash.unwrap();
    assert!(!hash_value.is_empty(), "Hash should not be empty");
    assert_eq!(hash_value.len(), 64, "SHA-256 hex hash should be 64 characters");
}

#[test]
fn test_fos_4_1_2_discussion_hash_is_deterministic() {
    let (pic, canister_id, user) = setup();

    // Create discussion
    let create_request = CreateDiscussionArgs {
        title: "Determinism Test".to_string(),
        description: "Testing hash determinism".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    // Get hash twice
    let hash_response_1 = pic.query_call(
        canister_id,
        user,
        "get_discussion_hash",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let hash_response_2 = pic.query_call(
        canister_id,
        user,
        "get_discussion_hash",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let hash_1: Option<String> = decode_one(&unwrap_wasm_result(hash_response_1)).unwrap();
    let hash_2: Option<String> = decode_one(&unwrap_wasm_result(hash_response_2)).unwrap();

    assert_eq!(hash_1, hash_2, "Hash should be deterministic");
}

#[test]
fn test_fos_4_1_2_list_discussions() {
    let (pic, canister_id, user) = setup();

    // Create multiple discussions
    for i in 1..=3 {
        let create_request = CreateDiscussionArgs {
            title: format!("Discussion {}", i),
            description: format!("Description {}", i),
            category: ProposalCategory::Operational,
        };

        let _ = pic.update_call(
            canister_id,
            user,
            "create_discussion",
            encode_one(create_request).unwrap(),
        ).unwrap();
    }

    // List all discussions
    let list_response = pic.query_call(
        canister_id,
        user,
        "list_discussions",
        encode_args((None::<DiscussionFilter>, None::<DiscussionPaginationParams>)).unwrap(),
    ).unwrap();

    let paginated: PaginatedDiscussionResponse = decode_one(&unwrap_wasm_result(list_response)).unwrap();
    assert_eq!(paginated.items.len(), 3, "Should have 3 discussions");
    assert_eq!(paginated.total, 3, "Total should be 3");
}

#[test]
fn test_fos_4_1_2_archive_discussion() {
    let (pic, canister_id, user) = setup();

    // Create discussion
    let create_request = CreateDiscussionArgs {
        title: "Archive Test".to_string(),
        description: "Testing archival".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    // Archive discussion
    let archive_response = pic.update_call(
        canister_id,
        user,
        "archive_discussion",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let archive_result: Result<(), String> = decode_one(&unwrap_wasm_result(archive_response)).unwrap();
    assert!(archive_result.is_ok(), "Archive should succeed");

    // Verify discussion is archived
    let get_response = pic.query_call(
        canister_id,
        user,
        "get_discussion",
        encode_one(1u64).unwrap(),
    ).unwrap();

    let discussion: Option<Discussion> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert!(discussion.unwrap().is_archived, "Discussion should be archived");

    // Verify archived discussions are excluded from default list
    let list_response = pic.query_call(
        canister_id,
        user,
        "list_discussions",
        encode_args((None::<DiscussionFilter>, None::<DiscussionPaginationParams>)).unwrap(),
    ).unwrap();

    let paginated: PaginatedDiscussionResponse = decode_one(&unwrap_wasm_result(list_response)).unwrap();
    assert_eq!(paginated.items.len(), 0, "Archived discussion should not appear in default list");
}

// ============================================================================
// FOS-4.1.2 Additional Tests (Code Review Fixes)
// ============================================================================

/// H1: Test that anyone can comment in Brainstorm stage (AC-4.1.2.4 related)
#[test]
fn test_fos_4_1_2_anyone_can_comment_in_brainstorm_stage() {
    let (pic, canister_id, proposer) = setup();
    let random_user = Principal::from_slice(&[99, 98, 97, 96, 95, 94, 93, 92, 91, 90]);

    // Create discussion (in Brainstorm stage by default)
    let create_request = CreateDiscussionArgs {
        title: "Open Discussion".to_string(),
        description: "Anyone should be able to comment in Brainstorm".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        proposer,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    // Random user (not proposer, not contributor) comments in Brainstorm stage
    let comment_args = AddCommentArgs {
        discussion_id: 1,
        content: "I'm a random user commenting in Brainstorm stage!".to_string(),
        author_type: AuthorType::Human,
    };

    let comment_response = pic.update_call(
        canister_id,
        random_user,  // Not the proposer!
        "add_comment",
        encode_one(comment_args).unwrap(),
    ).unwrap();

    let comment_result: Result<u64, String> = decode_one(&unwrap_wasm_result(comment_response)).unwrap();
    assert!(comment_result.is_ok(), "Random user should be able to comment in Brainstorm stage");
}

/// D1: Test that discussion state survives canister upgrades (DoD: stable storage)
#[test]
fn test_fos_4_1_2_discussion_state_survives_upgrade() {
    let (pic, canister_id, user) = setup();

    // Create a discussion
    let create_request = CreateDiscussionArgs {
        title: "Upgrade Test Discussion".to_string(),
        description: "Testing that state survives upgrade".to_string(),
        category: ProposalCategory::Treasury,
    };

    let create_response = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    let create_result: Result<u64, String> = decode_one(&unwrap_wasm_result(create_response)).unwrap();
    assert!(create_result.is_ok());
    let discussion_id = create_result.unwrap();

    // Add a comment
    let comment_args = AddCommentArgs {
        discussion_id,
        content: "This comment should survive the upgrade!".to_string(),
        author_type: AuthorType::Human,
    };

    let comment_response = pic.update_call(
        canister_id,
        user,
        "add_comment",
        encode_one(comment_args).unwrap(),
    ).unwrap();

    let comment_result: Result<u64, String> = decode_one(&unwrap_wasm_result(comment_response)).unwrap();
    assert!(comment_result.is_ok());

    // Perform canister upgrade (reinstall with same WASM)
    let wasm_path = std::env::var("CARGO_MANIFEST_DIR")
        .map(|dir| format!("{}/target/wasm32-unknown-unknown/release/foundery_os_core.wasm", dir))
        .unwrap_or_else(|_| "target/wasm32-unknown-unknown/release/foundery_os_core.wasm".to_string());
    let wasm = std::fs::read(&wasm_path).expect("Could not read WASM file for upgrade");

    // Upgrade the canister (this triggers pre_upgrade and post_upgrade)
    pic.upgrade_canister(canister_id, wasm, vec![], None).expect("Upgrade failed");

    // Verify discussion still exists after upgrade
    let get_response = pic.query_call(
        canister_id,
        user,
        "get_discussion",
        encode_one(discussion_id).unwrap(),
    ).unwrap();

    let discussion: Option<Discussion> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert!(discussion.is_some(), "Discussion should exist after upgrade");
    let discussion = discussion.unwrap();
    assert_eq!(discussion.title, "Upgrade Test Discussion", "Discussion title should be preserved");
    assert_eq!(discussion.category, ProposalCategory::Treasury, "Discussion category should be preserved");
    assert_eq!(discussion.comment_count, 1, "Comment count should be preserved");

    // Verify comments still exist after upgrade
    let comments_response = pic.query_call(
        canister_id,
        user,
        "get_comments",
        encode_args((discussion_id, 0u64, 100u64)).unwrap(),
    ).unwrap();

    let comments: Vec<Comment> = decode_one(&unwrap_wasm_result(comments_response)).unwrap();
    assert_eq!(comments.len(), 1, "Comments should be preserved after upgrade");
    assert_eq!(comments[0].content, "This comment should survive the upgrade!", "Comment content should be preserved");
}

/// H2: Test that skipping Refining stage is prevented (AC-4.1.2.3)
#[test]
fn test_fos_4_1_2_cannot_skip_refining_stage() {
    let (pic, canister_id, user) = setup();

    // Create discussion (starts in Brainstorm)
    let create_request = CreateDiscussionArgs {
        title: "Stage Skip Test".to_string(),
        description: "Trying to skip from Brainstorm directly to Ready".to_string(),
        category: ProposalCategory::Operational,
    };

    let _ = pic.update_call(
        canister_id,
        user,
        "create_discussion",
        encode_one(create_request).unwrap(),
    ).unwrap();

    // Verify we're in Brainstorm
    let get_response = pic.query_call(
        canister_id,
        user,
        "get_discussion",
        encode_one(1u64).unwrap(),
    ).unwrap();
    let discussion: Option<Discussion> = decode_one(&unwrap_wasm_result(get_response)).unwrap();
    assert_eq!(discussion.unwrap().stage, DiscussionStage::Brainstorm);

    // First advance should go to Refining (valid)
    let advance_response = pic.update_call(
        canister_id,
        user,
        "advance_stage",
        encode_one(1u64).unwrap(),
    ).unwrap();
    let advance_result: Result<DiscussionStage, String> = decode_one(&unwrap_wasm_result(advance_response)).unwrap();
    assert!(advance_result.is_ok(), "Brainstorm → Refining should succeed");
    assert_eq!(advance_result.unwrap(), DiscussionStage::Refining);

    // Trying to advance to Ready without meeting quality gates should fail
    // (This proves we can't skip to Ready - quality gates enforce Refining duration)
    let advance_response2 = pic.update_call(
        canister_id,
        user,
        "advance_stage",
        encode_one(1u64).unwrap(),
    ).unwrap();
    let advance_result2: Result<DiscussionStage, String> = decode_one(&unwrap_wasm_result(advance_response2)).unwrap();
    assert!(advance_result2.is_err(), "Refining → Ready should fail without quality gates");
    assert!(advance_result2.unwrap_err().contains("Quality gates not met"), "Error should mention quality gates");
}
