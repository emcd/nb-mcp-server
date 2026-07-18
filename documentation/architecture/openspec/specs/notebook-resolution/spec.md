# notebook-resolution Specification

## Purpose
Define how the server selects, validates, and creates the notebook used by MCP operations.
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

### Requirement: Automatic notebook creation
When the resolved notebook does not exist, the system SHALL create it before
executing the requested command, unless auto-creation is disabled.

#### Scenario: Create missing notebook
- **WHEN** the notebook name is resolved
- **AND** the notebook does not exist
- **THEN** the system creates the notebook and continues the request

#### Scenario: Auto-creation disabled
- **WHEN** the notebook name is resolved
- **AND** the notebook does not exist
- **AND** auto-creation is disabled
- **THEN** the system returns an error instructing how to create or enable it
