## MODIFIED Requirements

### Requirement: Tool surface

The system SHALL expose the `nb` multiplexed tool with the following subcommands:
`status`, `notebooks`, `add`, `show`, `delete`, `move`, `list`, `search`,
`todo`, `do`, `undo`, `tasks`, `bookmark`, `folders`, `mkdir`, `import`.

The system SHALL also expose each subcommand as a direct first-class MCP tool
with unprefixed server-local names: `status`, `notebooks`, `add`, `show`,
`delete`, `move`, `list`, `search`, `todo`, `do`, `undo`, `tasks`, `bookmark`,
`folders`, `mkdir`, `import`.

The system SHALL additionally expose direct-only first-class tools backed by
`nb-api 0.4.1` with no multiplexed alias: `replace_note_body`,
`edit_note_substring`, `edit_note_lines`, `retitle_note`, `edit_note_tags`,
`show_note_lines`, `search_note_lines`.

First-class tools SHALL use typed parameter schemas with direct dispatch,
bypassing the multiplexed command parsing path. The `show_note_lines` envelope
SHALL carry document-level `eol` + `has_final_eol` (no per-line terminator) and
numeric `.index` `selector` + `numeric_id` alongside `path`; the
`search_note_lines` envelope SHALL carry plain-`String` hit `text` with no EOL
fields and no `numeric_id` (upstream exposes neither on that type), per the
`nb-api-0-4-wire` capability.

The multiplexed `nb` tool SHALL remain as the compact/backcompat compatibility
surface for the retained subcommands only. Both surfaces SHALL produce identical results for equivalent operations.

#### Scenario: First-class tool availability
- **WHEN** a client requests tool schemas via `tools/list`
- **THEN** all 23 first-class tools are listed with typed parameter schemas

#### Scenario: Multiplexed tool preservation
- **WHEN** a client invokes `nb` with `command` and `args`
- **THEN** the operation succeeds identically to the equivalent first-class tool

#### Scenario: Cross-surface equivalence (read-only)
- **WHEN** a client invokes `list` directly and also invokes `nb` with `command: "list"` using identical parameters
- **THEN** both produce identical output

#### Scenario: Cross-surface equivalence (mutations)
- **WHEN** a client invokes a mutation tool (e.g., `add`) directly and also via multiplexed `nb` with identical parameters
- **THEN** both apply the same validation, routing, and nb CLI invocation

## ADDED Requirements

### Requirement: Direct-only exclusion from multiplexed surface
The multiplexed `nb` tool SHALL reject `replace_note_body`, `edit_note_substring`, `edit_note_lines`, `retitle_note`, `edit_note_tags`, `show_note_lines`, and `search_note_lines` as subcommands with guidance naming the direct tool, and SHALL reject the removed `edit` subcommand with guidance naming the replacement direct tools.

#### Scenario: Direct-only rejection
- **WHEN** a client invokes multiplexed `nb` with `command: "edit_note_lines"`
- **THEN** the request is rejected with guidance to invoke the direct `edit_note_lines` tool
