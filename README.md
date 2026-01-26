# FounderyOS Core Canister

Core productivity canister for FounderyOS - a user productivity platform on the Internet Computer.

## Overview

This canister provides the data layer for FounderyOS, handling:

- **Captures** - Ideas, Tasks, Projects, Reflections, Outlines, Calendar items
- **Sprints** - Sprint management and backlog organization
- **Workspaces** - Document organization and collaboration spaces
- **Documents** - Markdown content within workspaces
- **Templates** - Reusable capture and document templates

## Features

### Capture System
- 6 primary capture types (Idea, Task, Project, Reflection, Outline, Calendar)
- Dynamic fields support (estimates, due dates, labels, assignees)
- Hierarchical relationships (parent/child captures)
- Status tracking (Draft, Active, InProgress, Blocked, Completed, Archived, Cancelled)
- Priority levels (Low, Medium, High, Critical)

### Sprint Management
- Sprint lifecycle (Planning, Active, Review, Completed, Cancelled)
- Capacity planning
- Capture assignment to sprints
- Goal tracking

### Workspace & Documents
- Hierarchical workspace organization
- Markdown document storage
- Template-based document creation
- Folder structure support

### Templates
- Capture templates with default fields
- Document templates
- Public and private templates

## Building

```bash
# Build WASM
cargo build --release --target wasm32-unknown-unknown

# Or use the workspace build script
../../ops-infra/scripts/build-wasm.sh
```

## Testing

```bash
# Run PocketIC integration tests
cargo test
```

## API Reference

### Capture API

| Method | Type | Description |
|--------|------|-------------|
| `create_capture` | Update | Create a new capture |
| `get_capture` | Query | Get capture by ID |
| `update_capture` | Update | Update a capture |
| `delete_capture` | Update | Delete a capture |
| `get_my_captures` | Query | Get user's captures with filtering |

### Sprint API

| Method | Type | Description |
|--------|------|-------------|
| `create_sprint` | Update | Create a new sprint |
| `get_sprint` | Query | Get sprint by ID |
| `get_my_sprints` | Query | Get user's sprints |
| `add_capture_to_sprint` | Update | Add capture to sprint |
| `remove_capture_from_sprint` | Update | Remove capture from sprint |

### Workspace API

| Method | Type | Description |
|--------|------|-------------|
| `create_workspace` | Update | Create a new workspace |
| `get_workspace` | Query | Get workspace by ID |
| `get_my_workspaces` | Query | Get user's workspaces |

### Document API

| Method | Type | Description |
|--------|------|-------------|
| `create_document` | Update | Create a new document |
| `get_document` | Query | Get document by ID |
| `update_document` | Update | Update document content |
| `get_workspace_documents` | Query | Get documents in workspace |

### Template API

| Method | Type | Description |
|--------|------|-------------|
| `create_template` | Update | Create a new template |
| `get_template` | Query | Get template by ID |
| `get_my_templates` | Query | Get user's templates |
| `get_public_templates` | Query | Get public templates |

### Configuration

| Method | Type | Description |
|--------|------|-------------|
| `set_auth_service` | Update | Configure auth service canister |
| `get_auth_service` | Query | Get auth service canister ID |
| `get_controllers` | Query | Get controller principals |

### Health & Stats

| Method | Type | Description |
|--------|------|-------------|
| `health` | Query | Health check (returns "ok") |
| `get_stats` | Query | Get canister statistics |

## Data Privacy

All user data is isolated by principal. Users can only access their own:
- Captures
- Sprints
- Workspaces
- Documents
- Private templates

Public templates are visible to all authenticated users.

## Integration

This canister integrates with:
- **auth-service** - For session validation (optional)
- **foundery-os-suite** - User-facing React UI
- **foundery-os-agents** - AI agent service for capture assistance

## License

Part of the Hello World Co-Op DAO platform.
