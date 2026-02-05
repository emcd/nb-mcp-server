# notebook-resolution Specification

## Purpose
TBD - created by archiving change update-notebook-resolution. Update Purpose after archive.
## Requirements
### Requirement: Notebook resolution order
The system SHALL resolve the notebook name in this order:
1. Per-command notebook argument
2. Server configuration (`--notebook` or `NB_MCP_NOTEBOOK`)
3. Git-derived default from the master worktree path

The system SHALL NOT fall back to nb's default/current notebook.

#### Scenario: Explicit notebook argument
- **WHEN** a tool call includes a notebook argument
- **THEN** the system uses that notebook name
- **AND** no Git-derived fallback is used

#### Scenario: Git-derived default
- **WHEN** no notebook argument or server configuration is set
- **AND** the current working directory is within a Git repository
- **THEN** the system uses the basename of the master worktree path

### Requirement: Missing notebook
If the system cannot resolve a notebook name, it SHALL return an error that
instructs the user to configure `--notebook` or `NB_MCP_NOTEBOOK`.

#### Scenario: No notebook available
- **WHEN** no notebook argument or server configuration is set
- **AND** the current working directory is not a Git repository
- **THEN** the command fails with a configuration error

### Requirement: Commit-signing updates
When commit-signing disablement is requested, the system SHALL apply local
Git config updates to the resolved notebook repository only.

#### Scenario: Commit-signing without notebook
- **WHEN** commit-signing disablement is requested
- **AND** no notebook can be resolved
- **THEN** the system reports an error and performs no update

