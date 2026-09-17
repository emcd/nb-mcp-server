## MODIFIED Requirements

### Requirement: nb-api public API
The `nb-api` crate SHALL expose `NbClient` with public methods for all note-taking operations: `status`, `notebooks`, `notebook_path`, `add`, `show_note` (native-`String` text), `show_note_lines` (document-level `eol` + `has_final_eol`, bare-text edits), `search_note_lines` (`&str` pattern), `replace_note_body` (`&str`), `edit_note_substring` (`String` pattern/replacement), `edit_note_lines` (bare-text `LineEdit`), `retitle_note` (`&str`), `edit_note_tags`, `read_note_source_bytes` (raw-bytes hatch), `delete`, `move_note`, `todo`, `do_task`, `undo_task`, `tasks`, `bookmark`, `folders`, `mkdir` (created folders recorded in parent `.index` per the `0.4.1` fix), `import`. `nb-mcp-server` SHALL depend on the published `nb-api 0.4.1` release with no `ByteString` usage and no `base64` dependency for structured text. Methods MAY additionally expose typed accessor methods returning parsed/structured data alongside or in place of raw string output; such additive methods SHALL NOT break existing signatures.

#### Scenario: Full CRUD surface
- **WHEN** a consumer depends on `nb-api`
- **THEN** all note-taking operations are available as public methods on `NbClient`

#### Scenario: Additive typed accessors
- **WHEN** typed accessor methods are added to `NbClient`
- **THEN** existing method signatures remain unchanged

### Requirement: Error types
The `nb-api` crate SHALL expose `NbError` as the error type for all operations and `SearchMode`, `TaskStatus`, `LineEol`, `Occurrence` as operation parameter enums. `NbError` SHALL include `NonUtf8 { selector, path, kind, mime_hint }` for non-UTF-8-but-text files and `IndexLockTimeout { path, timeout_ms }` for `.index` lock contention alongside the existing typed variants. There SHALL be no `ByteString` type and no `EditMode`.

#### Scenario: Error and enum availability
- **WHEN** a consumer uses `nb-api`
- **THEN** all error and parameter types are available from the `nb-api` crate
