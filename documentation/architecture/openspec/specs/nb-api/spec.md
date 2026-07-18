# nb-api Specification

## Purpose
Define the API boundary between nb CLI operations and the MCP server.
## Requirements
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

### Requirement: Published dependency workflow
After the split, `nb-mcp-server` SHALL depend on `nb-api` via a crates.io published version only, without a path dependency.

#### Scenario: Dependency declaration
- **WHEN** a consumer reads `nb-mcp-server/Cargo.toml`
- **THEN** the `nb-api` dependency SHALL specify a compatible published version without a `path` field

#### Scenario: Build succeeds with published crate
- **WHEN** `cargo build` is run in `nb-mcp-server`
- **THEN** it SHALL resolve `nb-api` from crates.io and compile successfully

#### Scenario: Tests pass with published crate
- **WHEN** `cargo test` is run in `nb-mcp-server`
- **THEN** all tests SHALL pass using the published `nb-api` crate
