## ADDED Requirements

### Requirement: nb-api public API
The `nb-api` crate SHALL expose `NbClient` with public methods for all note-taking operations: `status`, `notebooks`, `notebook_path`, `add`, `show`, `list`, `search`, `edit`, `delete`, `move_note`, `todo`, `do_task`, `undo_task`, `tasks`, `bookmark`, `folders`, `mkdir`, `import`. Methods MAY additionally expose typed accessor methods returning parsed/structured data alongside or in place of raw string output; such additive methods SHALL NOT break existing `Result<String, NbError>` signatures.

#### Scenario: Full CRUD surface
- **WHEN** a consumer depends on `nb-api`
- **THEN** all note-taking operations are available as public methods on `NbClient`

#### Scenario: Additive typed accessors
- **WHEN** typed accessor methods are added to `NbClient`
- **THEN** existing `Result<String, NbError>` method signatures remain unchanged

### Requirement: Standalone dependency boundary
The `nb-api` crate SHALL NOT depend on `rmcp`. `schemars` SHALL be an optional feature, disabled by default.

#### Scenario: No MCP dependencies
- **WHEN** `nb-api` is compiled without the `schemars` feature
- **THEN** no MCP-specific crate is required as a dependency

### Requirement: Configuration
The `nb-api` crate SHALL expose a `Config` type containing only nb-relevant fields: notebook name, create_notebook flag, allow_top_level_notes flag, disable_git_signing flag.

#### Scenario: Config isolation
- **WHEN** a consumer constructs `NbClient`
- **THEN** only nb-relevant configuration is required; MCP-specific fields are not part of `nb-api::Config`

### Requirement: Error types
The `nb-api` crate SHALL expose `NbError` as the error type for all operations and `EditMode`, `SearchMode`, `TaskStatus` as operation parameter enums.

#### Scenario: Error and enum availability
- **WHEN** a consumer uses `nb-api`
- **THEN** all error and parameter types are available from the `nb-api` crate

### Requirement: Path dependency development workflow
During development, `nb-mcp-server` SHALL depend on `nb-api` via path dependency, enabling immediate validation of changes to `nb-api` through the MCP server's test suite.

#### Scenario: Same-workspace validation
- **WHEN** a change is made to `nb-api`
- **THEN** `nb-mcp-server` tests exercise the updated code without a version bump
