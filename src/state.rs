use crate::discussion::state::{StableDiscussionState, DISCUSSION_STATE};
use crate::types::*;
use candid::Principal;
use std::cell::RefCell;
use std::collections::BTreeMap;

/// State structure for the FounderyOS Core canister
#[derive(Default)]
pub struct State {
    // Access control
    pub controllers: Vec<Principal>,
    pub auth_service: Option<Principal>,

    // Captures storage (dual indexing: by Principal and by user_id string)
    pub captures: BTreeMap<CaptureId, Capture>,
    pub user_captures: BTreeMap<Principal, Vec<CaptureId>>,
    pub user_id_captures: BTreeMap<String, Vec<CaptureId>>,  // For session-based auth
    pub next_capture_id: CaptureId,

    // Sprints storage (dual indexing)
    pub sprints: BTreeMap<SprintId, Sprint>,
    pub user_sprints: BTreeMap<Principal, Vec<SprintId>>,
    pub user_id_sprints: BTreeMap<String, Vec<SprintId>>,  // For session-based auth
    pub next_sprint_id: SprintId,

    // Workspaces storage (dual indexing)
    pub workspaces: BTreeMap<WorkspaceId, Workspace>,
    pub user_workspaces: BTreeMap<Principal, Vec<WorkspaceId>>,
    pub user_id_workspaces: BTreeMap<String, Vec<WorkspaceId>>,  // For session-based auth
    pub next_workspace_id: WorkspaceId,

    // Documents storage
    pub documents: BTreeMap<DocumentId, Document>,
    pub workspace_documents: BTreeMap<WorkspaceId, Vec<DocumentId>>,
    pub next_document_id: DocumentId,

    // Templates storage (dual indexing)
    pub templates: BTreeMap<TemplateId, Template>,
    pub user_templates: BTreeMap<Principal, Vec<TemplateId>>,
    pub user_id_templates: BTreeMap<String, Vec<TemplateId>>,  // For session-based auth
    pub public_templates: Vec<TemplateId>,
    pub next_template_id: TemplateId,
}

impl State {
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
            auth_service: None,
            captures: BTreeMap::new(),
            user_captures: BTreeMap::new(),
            user_id_captures: BTreeMap::new(),
            next_capture_id: 1,
            sprints: BTreeMap::new(),
            user_sprints: BTreeMap::new(),
            user_id_sprints: BTreeMap::new(),
            next_sprint_id: 1,
            workspaces: BTreeMap::new(),
            user_workspaces: BTreeMap::new(),
            user_id_workspaces: BTreeMap::new(),
            next_workspace_id: 1,
            documents: BTreeMap::new(),
            workspace_documents: BTreeMap::new(),
            next_document_id: 1,
            templates: BTreeMap::new(),
            user_templates: BTreeMap::new(),
            user_id_templates: BTreeMap::new(),
            public_templates: Vec::new(),
            next_template_id: 1,
        }
    }

    /// Check if a principal is a controller
    pub fn is_controller(&self, principal: &Principal) -> bool {
        self.controllers.contains(principal)
    }

    /// Add a controller
    pub fn add_controller(&mut self, principal: Principal) {
        if !self.controllers.contains(&principal) {
            self.controllers.push(principal);
        }
    }

    /// Get list of controllers
    pub fn get_controllers(&self) -> Vec<Principal> {
        self.controllers.clone()
    }

    // =========================================================================
    // Capture Operations
    // =========================================================================

    /// Create a new capture
    pub fn create_capture(&mut self, owner: Principal, request: CreateCaptureRequest) -> Capture {
        let now = ic_cdk::api::time();
        let id = self.next_capture_id;
        self.next_capture_id += 1;

        let capture = Capture {
            id,
            owner,
            capture_type: request.capture_type,
            title: request.title,
            description: request.description,
            content: request.content,
            priority: request.priority.unwrap_or_default(),
            status: CaptureStatus::Draft,
            fields: request.fields.unwrap_or_default(),
            created_at: now,
            updated_at: now,
        };

        self.captures.insert(id, capture.clone());
        self.user_captures
            .entry(owner)
            .or_default()
            .push(id);

        capture
    }

    /// Get a capture by ID
    pub fn get_capture(&self, id: CaptureId) -> Option<&Capture> {
        self.captures.get(&id)
    }

    /// Update a capture
    pub fn update_capture(&mut self, request: UpdateCaptureRequest) -> Option<Capture> {
        let capture = self.captures.get_mut(&request.id)?;
        let now = ic_cdk::api::time();

        if let Some(title) = request.title {
            capture.title = title;
        }
        if let Some(description) = request.description {
            capture.description = Some(description);
        }
        if let Some(content) = request.content {
            capture.content = Some(content);
        }
        if let Some(priority) = request.priority {
            capture.priority = priority;
        }
        if let Some(status) = request.status {
            capture.status = status;
        }
        if let Some(fields) = request.fields {
            capture.fields = fields;
        }

        capture.updated_at = now;

        Some(capture.clone())
    }

    /// Delete a capture
    pub fn delete_capture(&mut self, id: CaptureId) -> Option<Capture> {
        let capture = self.captures.remove(&id)?;

        if let Some(user_captures) = self.user_captures.get_mut(&capture.owner) {
            user_captures.retain(|&cid| cid != id);
        }

        Some(capture)
    }

    /// Get captures for a user with optional filter
    pub fn get_user_captures(
        &self,
        owner: Principal,
        filter: Option<CaptureFilter>,
        pagination: PaginationParams,
    ) -> PaginatedResponse<Capture> {
        let capture_ids = self.user_captures.get(&owner).cloned().unwrap_or_default();

        let mut captures: Vec<Capture> = capture_ids
            .iter()
            .filter_map(|id| self.captures.get(id))
            .cloned()
            .collect();

        // Apply filters
        if let Some(ref f) = filter {
            if let Some(ref ct) = f.capture_type {
                captures.retain(|c| &c.capture_type == ct);
            }
            if let Some(ref s) = f.status {
                captures.retain(|c| &c.status == s);
            }
            if let Some(ref p) = f.priority {
                captures.retain(|c| &c.priority == p);
            }
            if let Some(sid) = f.sprint_id {
                captures.retain(|c| c.fields.sprint_id == Some(sid));
            }
            if let Some(wid) = f.workspace_id {
                captures.retain(|c| c.fields.workspace_id == Some(wid));
            }
        }

        let total = captures.len() as u64;
        let offset = pagination.offset.unwrap_or(0);
        let limit = pagination.limit.unwrap_or(50);

        let items: Vec<Capture> = captures
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        PaginatedResponse {
            items,
            total,
            offset,
            limit,
        }
    }

    // =========================================================================
    // User ID-based Operations (for session-based authentication)
    // =========================================================================

    /// Create a capture owned by a user_id (session-based auth)
    pub fn create_capture_for_user_id(&mut self, user_id: &str, request: CreateCaptureRequest) -> Capture {
        let now = ic_cdk::api::time();
        let id = self.next_capture_id;
        self.next_capture_id += 1;

        // Use anonymous principal as placeholder for user_id-owned captures
        let capture = Capture {
            id,
            owner: Principal::anonymous(),  // Placeholder - actual owner is user_id
            capture_type: request.capture_type,
            title: request.title,
            description: request.description,
            content: request.content,
            priority: request.priority.unwrap_or_default(),
            status: CaptureStatus::Draft,
            fields: request.fields.unwrap_or_default(),
            created_at: now,
            updated_at: now,
        };

        self.captures.insert(id, capture.clone());
        self.user_id_captures
            .entry(user_id.to_string())
            .or_default()
            .push(id);

        capture
    }

    /// Get captures by user_id (for session-based auth)
    pub fn get_user_id_captures(
        &self,
        user_id: &str,
        filter: Option<CaptureFilter>,
        pagination: PaginationParams,
    ) -> PaginatedResponse<Capture> {
        let capture_ids = self.user_id_captures.get(user_id).cloned().unwrap_or_default();

        let mut captures: Vec<Capture> = capture_ids
            .iter()
            .filter_map(|id| self.captures.get(id))
            .cloned()
            .collect();

        // Apply filters (same as get_user_captures)
        if let Some(ref f) = filter {
            if let Some(ref ct) = f.capture_type {
                captures.retain(|c| &c.capture_type == ct);
            }
            if let Some(ref s) = f.status {
                captures.retain(|c| &c.status == s);
            }
            if let Some(ref p) = f.priority {
                captures.retain(|c| &c.priority == p);
            }
            if let Some(sid) = f.sprint_id {
                captures.retain(|c| c.fields.sprint_id == Some(sid));
            }
            if let Some(wid) = f.workspace_id {
                captures.retain(|c| c.fields.workspace_id == Some(wid));
            }
        }

        let total = captures.len() as u64;
        let offset = pagination.offset.unwrap_or(0);
        let limit = pagination.limit.unwrap_or(50);

        let items: Vec<Capture> = captures
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        PaginatedResponse {
            items,
            total,
            offset,
            limit,
        }
    }

    /// Check if a capture is owned by user_id
    pub fn is_capture_owned_by_user_id(&self, capture_id: CaptureId, user_id: &str) -> bool {
        self.user_id_captures
            .get(user_id)
            .map(|ids| ids.contains(&capture_id))
            .unwrap_or(false)
    }

    /// Delete a capture owned by user_id
    pub fn delete_capture_by_user_id(&mut self, id: CaptureId, user_id: &str) -> Option<Capture> {
        // Verify ownership
        if !self.is_capture_owned_by_user_id(id, user_id) {
            return None;
        }

        let capture = self.captures.remove(&id)?;

        // Remove from user_id index
        if let Some(user_captures) = self.user_id_captures.get_mut(user_id) {
            user_captures.retain(|&cid| cid != id);
        }

        Some(capture)
    }

    /// Create a sprint owned by user_id
    pub fn create_sprint_for_user_id(&mut self, user_id: &str, request: CreateSprintRequest) -> Sprint {
        let now = ic_cdk::api::time();
        let id = self.next_sprint_id;
        self.next_sprint_id += 1;

        let sprint = Sprint {
            id,
            owner: Principal::anonymous(),  // Placeholder - actual owner is user_id
            name: request.name,
            goal: request.goal,
            status: SprintStatus::Planning,
            start_date: request.start_date,
            end_date: request.end_date,
            capacity: request.capacity,
            capture_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        self.sprints.insert(id, sprint.clone());
        self.user_id_sprints
            .entry(user_id.to_string())
            .or_default()
            .push(id);

        sprint
    }

    /// Get sprints by user_id
    pub fn get_user_id_sprints(&self, user_id: &str) -> Vec<Sprint> {
        self.user_id_sprints
            .get(user_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.sprints.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if a sprint is owned by user_id
    pub fn is_sprint_owned_by_user_id(&self, sprint_id: SprintId, user_id: &str) -> bool {
        self.user_id_sprints
            .get(user_id)
            .map(|ids| ids.contains(&sprint_id))
            .unwrap_or(false)
    }

    /// Create a workspace owned by user_id
    pub fn create_workspace_for_user_id(&mut self, user_id: &str, request: CreateWorkspaceRequest) -> Workspace {
        let now = ic_cdk::api::time();
        let id = self.next_workspace_id;
        self.next_workspace_id += 1;

        let workspace = Workspace {
            id,
            owner: Principal::anonymous(),  // Placeholder - actual owner is user_id
            name: request.name,
            description: request.description,
            icon: request.icon,
            parent_id: request.parent_id,
            is_archived: false,
            created_at: now,
            updated_at: now,
        };

        self.workspaces.insert(id, workspace.clone());
        self.user_id_workspaces
            .entry(user_id.to_string())
            .or_default()
            .push(id);

        workspace
    }

    /// Get workspaces by user_id
    pub fn get_user_id_workspaces(&self, user_id: &str) -> Vec<Workspace> {
        self.user_id_workspaces
            .get(user_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.workspaces.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if a workspace is owned by user_id
    pub fn is_workspace_owned_by_user_id(&self, workspace_id: WorkspaceId, user_id: &str) -> bool {
        self.user_id_workspaces
            .get(user_id)
            .map(|ids| ids.contains(&workspace_id))
            .unwrap_or(false)
    }

    /// Create a template owned by user_id
    pub fn create_template_for_user_id(&mut self, user_id: &str, request: CreateTemplateRequest) -> Template {
        let now = ic_cdk::api::time();
        let id = self.next_template_id;
        self.next_template_id += 1;

        let is_public = request.is_public.unwrap_or(false);

        let template = Template {
            id,
            owner: Principal::anonymous(),  // Placeholder - actual owner is user_id
            template_type: request.template_type,
            name: request.name,
            description: request.description,
            content: request.content,
            capture_type: request.capture_type,
            default_fields: request.default_fields,
            is_public,
            created_at: now,
            updated_at: now,
        };

        self.templates.insert(id, template.clone());
        self.user_id_templates
            .entry(user_id.to_string())
            .or_default()
            .push(id);

        if is_public {
            self.public_templates.push(id);
        }

        template
    }

    /// Get templates by user_id
    pub fn get_user_id_templates(&self, user_id: &str) -> Vec<Template> {
        self.user_id_templates
            .get(user_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.templates.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if a template is owned by user_id
    pub fn is_template_owned_by_user_id(&self, template_id: TemplateId, user_id: &str) -> bool {
        self.user_id_templates
            .get(user_id)
            .map(|ids| ids.contains(&template_id))
            .unwrap_or(false)
    }

    // =========================================================================
    // Sprint Operations
    // =========================================================================

    /// Create a new sprint
    pub fn create_sprint(&mut self, owner: Principal, request: CreateSprintRequest) -> Sprint {
        let now = ic_cdk::api::time();
        let id = self.next_sprint_id;
        self.next_sprint_id += 1;

        let sprint = Sprint {
            id,
            owner,
            name: request.name,
            goal: request.goal,
            status: SprintStatus::Planning,
            start_date: request.start_date,
            end_date: request.end_date,
            capacity: request.capacity,
            capture_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        self.sprints.insert(id, sprint.clone());
        self.user_sprints
            .entry(owner)
            .or_default()
            .push(id);

        sprint
    }

    /// Get a sprint by ID
    pub fn get_sprint(&self, id: SprintId) -> Option<&Sprint> {
        self.sprints.get(&id)
    }

    /// Add capture to sprint
    pub fn add_capture_to_sprint(&mut self, sprint_id: SprintId, capture_id: CaptureId) -> Result<(), String> {
        let sprint = self.sprints.get_mut(&sprint_id)
            .ok_or_else(|| "Sprint not found".to_string())?;

        let capture = self.captures.get_mut(&capture_id)
            .ok_or_else(|| "Capture not found".to_string())?;

        if !sprint.capture_ids.contains(&capture_id) {
            sprint.capture_ids.push(capture_id);
            capture.fields.sprint_id = Some(sprint_id);
        }

        Ok(())
    }

    /// Remove capture from sprint
    pub fn remove_capture_from_sprint(&mut self, sprint_id: SprintId, capture_id: CaptureId) -> Result<(), String> {
        let sprint = self.sprints.get_mut(&sprint_id)
            .ok_or_else(|| "Sprint not found".to_string())?;

        sprint.capture_ids.retain(|&id| id != capture_id);

        if let Some(capture) = self.captures.get_mut(&capture_id) {
            if capture.fields.sprint_id == Some(sprint_id) {
                capture.fields.sprint_id = None;
            }
        }

        Ok(())
    }

    /// Get user's sprints
    pub fn get_user_sprints(&self, owner: Principal) -> Vec<Sprint> {
        self.user_sprints
            .get(&owner)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.sprints.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Update a sprint
    pub fn update_sprint(&mut self, id: SprintId, request: UpdateSprintRequest) -> Option<Sprint> {
        let sprint = self.sprints.get_mut(&id)?;
        let now = ic_cdk::api::time();

        if let Some(name) = request.name {
            sprint.name = name;
        }
        if let Some(goal) = request.goal {
            sprint.goal = Some(goal);
        }
        if let Some(status) = request.status {
            sprint.status = status;
        }
        if let Some(start_date) = request.start_date {
            sprint.start_date = start_date;
        }
        if let Some(end_date) = request.end_date {
            sprint.end_date = end_date;
        }
        if let Some(capacity) = request.capacity {
            sprint.capacity = Some(capacity);
        }

        sprint.updated_at = now;

        Some(sprint.clone())
    }

    /// Delete a sprint
    pub fn delete_sprint(&mut self, id: SprintId) -> Option<Sprint> {
        let sprint = self.sprints.remove(&id)?;

        // Remove from user_sprints index
        if let Some(user_sprints) = self.user_sprints.get_mut(&sprint.owner) {
            user_sprints.retain(|&sid| sid != id);
        }

        Some(sprint)
    }

    // =========================================================================
    // Workspace Operations
    // =========================================================================

    /// Create a new workspace
    pub fn create_workspace(&mut self, owner: Principal, request: CreateWorkspaceRequest) -> Workspace {
        let now = ic_cdk::api::time();
        let id = self.next_workspace_id;
        self.next_workspace_id += 1;

        let workspace = Workspace {
            id,
            owner,
            name: request.name,
            description: request.description,
            icon: request.icon,
            parent_id: request.parent_id,
            is_archived: false,
            created_at: now,
            updated_at: now,
        };

        self.workspaces.insert(id, workspace.clone());
        self.user_workspaces
            .entry(owner)
            .or_default()
            .push(id);

        workspace
    }

    /// Get a workspace by ID
    pub fn get_workspace(&self, id: WorkspaceId) -> Option<&Workspace> {
        self.workspaces.get(&id)
    }

    /// Get user's workspaces
    pub fn get_user_workspaces(&self, owner: Principal) -> Vec<Workspace> {
        self.user_workspaces
            .get(&owner)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.workspaces.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Update a workspace
    pub fn update_workspace(&mut self, id: WorkspaceId, request: UpdateWorkspaceRequest) -> Option<Workspace> {
        let workspace = self.workspaces.get_mut(&id)?;
        let now = ic_cdk::api::time();

        if let Some(name) = request.name {
            workspace.name = name;
        }
        if let Some(description) = request.description {
            workspace.description = Some(description);
        }
        if let Some(icon) = request.icon {
            workspace.icon = Some(icon);
        }
        if let Some(parent_id) = request.parent_id {
            workspace.parent_id = Some(parent_id);
        }
        if let Some(is_archived) = request.is_archived {
            workspace.is_archived = is_archived;
        }

        workspace.updated_at = now;

        Some(workspace.clone())
    }

    /// Delete a workspace
    pub fn delete_workspace(&mut self, id: WorkspaceId) -> Option<Workspace> {
        let workspace = self.workspaces.remove(&id)?;

        // Remove from user_workspaces index
        if let Some(user_workspaces) = self.user_workspaces.get_mut(&workspace.owner) {
            user_workspaces.retain(|&wid| wid != id);
        }

        // Also remove workspace_documents index
        self.workspace_documents.remove(&id);

        Some(workspace)
    }

    // =========================================================================
    // Document Operations
    // =========================================================================

    /// Create a new document
    pub fn create_document(&mut self, owner: Principal, request: CreateDocumentRequest) -> Result<Document, String> {
        // Verify workspace exists
        if !self.workspaces.contains_key(&request.workspace_id) {
            return Err("Workspace not found".to_string());
        }

        let now = ic_cdk::api::time();
        let id = self.next_document_id;
        self.next_document_id += 1;

        let content = if let Some(template_id) = request.template_id {
            // Load content from template
            self.templates
                .get(&template_id)
                .map(|t| t.content.clone())
                .unwrap_or_default()
        } else {
            request.content.unwrap_or_default()
        };

        let document = Document {
            id,
            workspace_id: request.workspace_id,
            owner,
            title: request.title,
            content,
            is_template: false,
            template_id: request.template_id,
            parent_id: request.parent_id,
            created_at: now,
            updated_at: now,
        };

        self.documents.insert(id, document.clone());
        self.workspace_documents
            .entry(request.workspace_id)
            .or_default()
            .push(id);

        Ok(document)
    }

    /// Get a document by ID
    pub fn get_document(&self, id: DocumentId) -> Option<&Document> {
        self.documents.get(&id)
    }

    /// Update document content
    pub fn update_document(&mut self, id: DocumentId, title: Option<String>, content: Option<String>) -> Option<Document> {
        let doc = self.documents.get_mut(&id)?;
        let now = ic_cdk::api::time();

        if let Some(t) = title {
            doc.title = t;
        }
        if let Some(c) = content {
            doc.content = c;
        }
        doc.updated_at = now;

        Some(doc.clone())
    }

    /// Get documents in a workspace
    pub fn get_workspace_documents(&self, workspace_id: WorkspaceId) -> Vec<Document> {
        self.workspace_documents
            .get(&workspace_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.documents.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Delete a document
    pub fn delete_document(&mut self, id: DocumentId) -> Option<Document> {
        let document = self.documents.remove(&id)?;

        // Remove from workspace_documents index
        if let Some(workspace_docs) = self.workspace_documents.get_mut(&document.workspace_id) {
            workspace_docs.retain(|&did| did != id);
        }

        Some(document)
    }

    // =========================================================================
    // Template Operations
    // =========================================================================

    /// Create a new template
    pub fn create_template(&mut self, owner: Principal, request: CreateTemplateRequest) -> Template {
        let now = ic_cdk::api::time();
        let id = self.next_template_id;
        self.next_template_id += 1;

        let is_public = request.is_public.unwrap_or(false);

        let template = Template {
            id,
            owner,
            template_type: request.template_type,
            name: request.name,
            description: request.description,
            content: request.content,
            capture_type: request.capture_type,
            default_fields: request.default_fields,
            is_public,
            created_at: now,
            updated_at: now,
        };

        self.templates.insert(id, template.clone());
        self.user_templates
            .entry(owner)
            .or_default()
            .push(id);

        if is_public {
            self.public_templates.push(id);
        }

        template
    }

    /// Get a template by ID
    pub fn get_template(&self, id: TemplateId) -> Option<&Template> {
        self.templates.get(&id)
    }

    /// Get user's templates
    pub fn get_user_templates(&self, owner: Principal) -> Vec<Template> {
        self.user_templates
            .get(&owner)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.templates.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get public templates
    pub fn get_public_templates(&self) -> Vec<Template> {
        self.public_templates
            .iter()
            .filter_map(|id| self.templates.get(id))
            .cloned()
            .collect()
    }

    /// Update a template
    pub fn update_template(&mut self, id: TemplateId, request: UpdateTemplateRequest) -> Option<Template> {
        let template = self.templates.get_mut(&id)?;
        let now = ic_cdk::api::time();
        let was_public = template.is_public;

        if let Some(name) = request.name {
            template.name = name;
        }
        if let Some(description) = request.description {
            template.description = Some(description);
        }
        if let Some(content) = request.content {
            template.content = content;
        }
        if let Some(capture_type) = request.capture_type {
            template.capture_type = Some(capture_type);
        }
        if let Some(default_fields) = request.default_fields {
            template.default_fields = Some(default_fields);
        }
        if let Some(is_public) = request.is_public {
            template.is_public = is_public;

            // Update public_templates index
            if is_public && !was_public {
                // Add to public templates
                if !self.public_templates.contains(&id) {
                    self.public_templates.push(id);
                }
            } else if !is_public && was_public {
                // Remove from public templates
                self.public_templates.retain(|&tid| tid != id);
            }
        }

        template.updated_at = now;

        Some(template.clone())
    }

    /// Delete a template
    pub fn delete_template(&mut self, id: TemplateId) -> Option<Template> {
        let template = self.templates.remove(&id)?;

        // Remove from user_templates index
        if let Some(user_templates) = self.user_templates.get_mut(&template.owner) {
            user_templates.retain(|&tid| tid != id);
        }

        // Remove from public_templates if applicable
        if template.is_public {
            self.public_templates.retain(|&tid| tid != id);
        }

        Some(template)
    }
}

thread_local! {
    pub static STATE: RefCell<State> = RefCell::new(State::new());
}

/// Serializable state for upgrades
#[derive(candid::CandidType, serde::Deserialize, Clone)]
pub struct StableState {
    pub controllers: Vec<Principal>,
    pub auth_service: Option<Principal>,
    pub captures: Vec<(CaptureId, Capture)>,
    pub user_captures: Vec<(Principal, Vec<CaptureId>)>,
    pub user_id_captures: Vec<(String, Vec<CaptureId>)>,  // For session-based auth
    pub next_capture_id: CaptureId,
    pub sprints: Vec<(SprintId, Sprint)>,
    pub user_sprints: Vec<(Principal, Vec<SprintId>)>,
    pub user_id_sprints: Vec<(String, Vec<SprintId>)>,  // For session-based auth
    pub next_sprint_id: SprintId,
    pub workspaces: Vec<(WorkspaceId, Workspace)>,
    pub user_workspaces: Vec<(Principal, Vec<WorkspaceId>)>,
    pub user_id_workspaces: Vec<(String, Vec<WorkspaceId>)>,  // For session-based auth
    pub next_workspace_id: WorkspaceId,
    pub documents: Vec<(DocumentId, Document)>,
    pub workspace_documents: Vec<(WorkspaceId, Vec<DocumentId>)>,
    pub next_document_id: DocumentId,
    pub templates: Vec<(TemplateId, Template)>,
    pub user_templates: Vec<(Principal, Vec<TemplateId>)>,
    pub user_id_templates: Vec<(String, Vec<TemplateId>)>,  // For session-based auth
    pub public_templates: Vec<TemplateId>,
    pub next_template_id: TemplateId,
    /// Discussion state (Story FOS-4.1.2)
    #[serde(default)]
    pub discussion_state: Option<StableDiscussionState>,
}

impl From<&State> for StableState {
    fn from(state: &State) -> Self {
        // Capture discussion state from thread-local
        let discussion_state = DISCUSSION_STATE.with(|ds| {
            Some(StableDiscussionState::from(&*ds.borrow()))
        });

        StableState {
            controllers: state.controllers.clone(),
            auth_service: state.auth_service,
            captures: state.captures.iter().map(|(k, v)| (*k, v.clone())).collect(),
            user_captures: state.user_captures.iter().map(|(k, v)| (*k, v.clone())).collect(),
            user_id_captures: state.user_id_captures.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            next_capture_id: state.next_capture_id,
            sprints: state.sprints.iter().map(|(k, v)| (*k, v.clone())).collect(),
            user_sprints: state.user_sprints.iter().map(|(k, v)| (*k, v.clone())).collect(),
            user_id_sprints: state.user_id_sprints.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            next_sprint_id: state.next_sprint_id,
            workspaces: state.workspaces.iter().map(|(k, v)| (*k, v.clone())).collect(),
            user_workspaces: state.user_workspaces.iter().map(|(k, v)| (*k, v.clone())).collect(),
            user_id_workspaces: state.user_id_workspaces.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            next_workspace_id: state.next_workspace_id,
            documents: state.documents.iter().map(|(k, v)| (*k, v.clone())).collect(),
            workspace_documents: state.workspace_documents.iter().map(|(k, v)| (*k, v.clone())).collect(),
            next_document_id: state.next_document_id,
            templates: state.templates.iter().map(|(k, v)| (*k, v.clone())).collect(),
            user_templates: state.user_templates.iter().map(|(k, v)| (*k, v.clone())).collect(),
            user_id_templates: state.user_id_templates.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            public_templates: state.public_templates.clone(),
            next_template_id: state.next_template_id,
            discussion_state,
        }
    }
}

impl From<StableState> for State {
    fn from(stable: StableState) -> Self {
        // Restore discussion state to thread-local
        if let Some(ds) = stable.discussion_state {
            DISCUSSION_STATE.with(|state| {
                *state.borrow_mut() = ds.into();
            });
        }

        State {
            controllers: stable.controllers,
            auth_service: stable.auth_service,
            captures: stable.captures.into_iter().collect(),
            user_captures: stable.user_captures.into_iter().collect(),
            user_id_captures: stable.user_id_captures.into_iter().collect(),
            next_capture_id: stable.next_capture_id,
            sprints: stable.sprints.into_iter().collect(),
            user_sprints: stable.user_sprints.into_iter().collect(),
            user_id_sprints: stable.user_id_sprints.into_iter().collect(),
            next_sprint_id: stable.next_sprint_id,
            workspaces: stable.workspaces.into_iter().collect(),
            user_workspaces: stable.user_workspaces.into_iter().collect(),
            user_id_workspaces: stable.user_id_workspaces.into_iter().collect(),
            next_workspace_id: stable.next_workspace_id,
            documents: stable.documents.into_iter().collect(),
            workspace_documents: stable.workspace_documents.into_iter().collect(),
            next_document_id: stable.next_document_id,
            templates: stable.templates.into_iter().collect(),
            user_templates: stable.user_templates.into_iter().collect(),
            user_id_templates: stable.user_id_templates.into_iter().collect(),
            public_templates: stable.public_templates,
            next_template_id: stable.next_template_id,
        }
    }
}
