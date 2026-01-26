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
