## Purpose
Define the nb MCP tool surface for note-taking workflows.
## Requirements
### Requirement: Tool surface
The system SHALL expose the `nb` multiplexed tool with the following subcommands:
`status`, `notebooks`, `add`, `show`, `edit`, `delete`, `move`, `list`, `search`,
`todo`, `do`, `undo`, `tasks`, `bookmark`, `folders`, `mkdir`, `import`.

The system SHALL also expose each subcommand as a direct first-class MCP tool
with unprefixed server-local names: `status`, `notebooks`, `add`, `show`, `edit`,
`delete`, `move`, `list`, `search`, `todo`, `do`, `undo`, `tasks`, `bookmark`,
`folders`, `mkdir`, `import`.

First-class tools SHALL use typed parameter schemas with direct dispatch,
bypassing the multiplexed command parsing path.

The multiplexed `nb` tool SHALL remain as the compact/backcompat compatibility
surface. Both surfaces SHALL produce identical results for equivalent operations.

#### Scenario: First-class tool availability
- **WHEN** a client requests tool schemas via `tools/list`
- **THEN** all 17 first-class tools are listed with typed parameter schemas

#### Scenario: Multiplexed tool preservation
- **WHEN** a client invokes `nb` with `command` and `args`
- **THEN** the operation succeeds identically to the equivalent first-class tool

#### Scenario: Cross-surface equivalence (read-only)
- **WHEN** a client invokes `list` directly and also invokes `nb` with `command: "list"` using identical parameters
- **THEN** both produce identical output

#### Scenario: Cross-surface equivalence (mutations)
- **WHEN** a client invokes a mutation tool (e.g., `add`) directly and also via multiplexed `nb` with identical parameters
- **THEN** both apply the same validation, routing, and nb CLI invocation

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

### Requirement: MiMo/Xiaomi schema compatibility
All first-class tool parameter schemas SHALL render `Option<T>` fields as plain
single types (e.g., `"type": "string"`) rather than nullable unions
(e.g., `["string", "null"]`). This is required for compatibility with
Xiaomi/MiMo tool-call serializers that truncate JSON at nullable-union properties.

#### Scenario: Optional scalar fields are plain types
- **WHEN** a first-class tool has an `Option<String>` parameter
- **THEN** the schema renders as `"type": "string"` without `anyOf`/`oneOf`/nullable unions

#### Scenario: Optional fields remain optional
- **WHEN** a first-class tool has an `Option<String>` parameter with `#[serde(default)]`
- **THEN** the field is absent from the schema's `required` array

### Requirement: Error string consistency
Error messages for equivalent validation failures SHALL use consistent wording
across multiplexed and first-class tool surfaces.

#### Scenario: Empty-queries error alignment
- **WHEN** a client invokes `search` with an empty `queries` array
- **THEN** the error message matches the wording used by multiplexed `nb.search` with empty queries
