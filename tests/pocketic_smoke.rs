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
