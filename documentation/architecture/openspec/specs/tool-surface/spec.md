## Purpose
Define the nb MCP tool surface for note-taking workflows.

## Requirements

### Requirement: Tool surface
The system SHALL expose the `nb` tool with the following subcommands:
`status`, `notebooks`, `add`, `show`, `edit`, `delete`, `move`, `list`, `search`,
`todo`, `do`, `undo`, `tasks`, `bookmark`, `folders`, `mkdir`, `import`.

#### Scenario: Tool availability
- **WHEN** a client requests tool schemas
- **THEN** the `nb` tool is listed with the supported subcommands

### Requirement: Content handling
The system SHALL accept raw string content for `add` and `edit`, including
content that contains backticks.

#### Scenario: Backtick content
- **WHEN** a client submits content containing backticks
- **THEN** the stored note preserves the backticks

### Requirement: Tag normalization
The system SHALL accept tags as bare strings and prefix them with `#` when
invoking nb. Tags already prefixed with `#` SHALL be preserved.

#### Scenario: Tag prefixes
- **WHEN** tags are provided without a `#` prefix
- **THEN** the system prefixes them before invoking nb

### Requirement: Delete confirmation
The system SHALL require an explicit confirmation flag to delete a note.

#### Scenario: Delete without confirmation
- **WHEN** a delete request omits confirmation
- **THEN** the system rejects the request

### Requirement: Folder scoping
The system SHALL support folder scoping for listing and creating notes.

#### Scenario: Folder creation
- **WHEN** a note is created with a folder
- **THEN** the note is created within that folder
