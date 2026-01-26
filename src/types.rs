use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

// =============================================================================
// Common Types
// =============================================================================

pub type CaptureId = u64;
pub type SprintId = u64;
pub type WorkspaceId = u64;
pub type DocumentId = u64;
pub type TemplateId = u64;
pub type Timestamp = u64;

/// User account - owner principal with optional subaccount
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<[u8; 32]>,
}

impl Account {
    pub fn new(owner: Principal) -> Self {
        Self {
            owner,
            subaccount: None,
        }
    }
}

// =============================================================================
// Capture Types (Ideas, Tasks, Projects, Reflections, Outlines, Calendar)
// =============================================================================

/// Primary capture type - the main category of a capture
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq)]
pub enum CaptureType {
    Idea,
    Task,
    Project,
    Reflection,
    Outline,
    Calendar,
}

/// Priority level for captures
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}

/// Status of a capture
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq)]
pub enum CaptureStatus {
    Draft,
    Active,
    InProgress,
    Blocked,
    Completed,
    Archived,
    Cancelled,
}

impl Default for CaptureStatus {
    fn default() -> Self {
        CaptureStatus::Draft
    }
}

/// Idea subtypes from FOS captureTaxonomy
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq)]
pub enum IdeaSubtype {
    FeatureRequest,
    Innovation,
    Improvement,
    Research,
    Experiment,
    Concept,
    Vision,
}

/// Task subtypes from FOS captureTaxonomy
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq)]
pub enum TaskSubtype {
    Development,
    Design,
    Documentation,
    Review,
    Testing,
    Deployment,
    Maintenance,
    BugFix,
    Refactor,
}

/// Project subtypes
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq)]
pub enum ProjectSubtype {
    Feature,
    Initiative,
    Epic,
    Milestone,
    Release,
    Campaign,
}

/// Dynamic fields that vary by capture type/subtype
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, Default)]
pub struct DynamicFields {
    /// Estimated effort (story points)
    pub estimate: Option<u32>,
    /// Due date (timestamp)
    pub due_date: Option<Timestamp>,
    /// Start date (timestamp)
    pub start_date: Option<Timestamp>,
    /// Assigned to (user IDs)
    pub assignees: Vec<String>,
    /// Labels/tags
    pub labels: Vec<String>,
    /// Related capture IDs
    pub related_captures: Vec<CaptureId>,
    /// Parent capture ID (for hierarchy)
    pub parent_id: Option<CaptureId>,
    /// Sprint ID if assigned to sprint
    pub sprint_id: Option<SprintId>,
    /// Workspace ID
    pub workspace_id: Option<WorkspaceId>,
    /// Custom key-value metadata
    pub custom_fields: Vec<(String, String)>,
}

/// Core capture record
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct Capture {
    pub id: CaptureId,
    pub owner: Principal,
    pub capture_type: CaptureType,
    pub title: String,
    pub description: Option<String>,
    pub content: Option<String>,
    pub priority: Priority,
    pub status: CaptureStatus,
    pub fields: DynamicFields,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Request to create a new capture
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct CreateCaptureRequest {
    pub capture_type: CaptureType,
    pub title: String,
    pub description: Option<String>,
    pub content: Option<String>,
    pub priority: Option<Priority>,
    pub fields: Option<DynamicFields>,
}

/// Request to update a capture
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct UpdateCaptureRequest {
    pub id: CaptureId,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub priority: Option<Priority>,
    pub status: Option<CaptureStatus>,
    pub fields: Option<DynamicFields>,
}

// =============================================================================
// Sprint Types
// =============================================================================

/// Sprint status
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq)]
pub enum SprintStatus {
    Planning,
    Active,
    Review,
    Completed,
    Cancelled,
}

impl Default for SprintStatus {
    fn default() -> Self {
        SprintStatus::Planning
    }
}

/// Sprint record
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct Sprint {
    pub id: SprintId,
    pub owner: Principal,
    pub name: String,
    pub goal: Option<String>,
    pub status: SprintStatus,
    pub start_date: Timestamp,
    pub end_date: Timestamp,
    pub capacity: Option<u32>,
    pub capture_ids: Vec<CaptureId>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Request to create a sprint
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct CreateSprintRequest {
    pub name: String,
    pub goal: Option<String>,
    pub start_date: Timestamp,
    pub end_date: Timestamp,
    pub capacity: Option<u32>,
}

/// Request to update a sprint
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct UpdateSprintRequest {
    pub name: Option<String>,
    pub goal: Option<String>,
    pub status: Option<SprintStatus>,
    pub start_date: Option<Timestamp>,
    pub end_date: Option<Timestamp>,
    pub capacity: Option<u32>,
}

// =============================================================================
// Workspace & Document Types
// =============================================================================

/// Workspace record - a container for documents and captures
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub owner: Principal,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub parent_id: Option<WorkspaceId>,
    pub is_archived: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Document record - markdown content within a workspace
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct Document {
    pub id: DocumentId,
    pub workspace_id: WorkspaceId,
    pub owner: Principal,
    pub title: String,
    pub content: String,
    pub is_template: bool,
    pub template_id: Option<TemplateId>,
    pub parent_id: Option<DocumentId>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Request to create a workspace
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub parent_id: Option<WorkspaceId>,
}

/// Request to update a workspace
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub parent_id: Option<WorkspaceId>,
    pub is_archived: Option<bool>,
}

/// Request to create a document
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct CreateDocumentRequest {
    pub workspace_id: WorkspaceId,
    pub title: String,
    pub content: Option<String>,
    pub template_id: Option<TemplateId>,
    pub parent_id: Option<DocumentId>,
}

// =============================================================================
// Template Types
// =============================================================================

/// Template type
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq)]
pub enum TemplateType {
    Capture,
    Document,
}

/// Template record
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct Template {
    pub id: TemplateId,
    pub owner: Principal,
    pub template_type: TemplateType,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub capture_type: Option<CaptureType>,
    pub default_fields: Option<DynamicFields>,
    pub is_public: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Request to create a template
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct CreateTemplateRequest {
    pub template_type: TemplateType,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub capture_type: Option<CaptureType>,
    pub default_fields: Option<DynamicFields>,
    pub is_public: Option<bool>,
}

/// Request to update a template
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub capture_type: Option<CaptureType>,
    pub default_fields: Option<DynamicFields>,
    pub is_public: Option<bool>,
}

// =============================================================================
// Query Types
// =============================================================================

/// Filter for querying captures
#[derive(Clone, Debug, CandidType, Deserialize, Default)]
pub struct CaptureFilter {
    pub capture_type: Option<CaptureType>,
    pub status: Option<CaptureStatus>,
    pub priority: Option<Priority>,
    pub sprint_id: Option<SprintId>,
    pub workspace_id: Option<WorkspaceId>,
    pub labels: Option<Vec<String>>,
}

/// Pagination params
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct PaginationParams {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            offset: Some(0),
            limit: Some(50),
        }
    }
}

/// Paginated response wrapper
#[derive(Clone, Debug, CandidType, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}
