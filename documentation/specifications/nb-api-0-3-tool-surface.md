<!-- nbspec: change=adapt-nb-api-0-3 notebook=nb-mcp-server note=proposals/adapt-nb-api-0-3/specifications/20260813235539.md hash=sha256:3dbbaced082f9c976635c51bc0ef0085964f8ed781916de35d6251fc926d85a0 -->
# nb-api-0-3-tool-surface

## ADDED Requirements

### Requirement: Published nb-api 0.3 integration

`nb-mcp-server` SHALL depend on the published `nb-api 0.3.0` crates.io release without a path or Git source and SHALL compile exclusively against the public 0.3 method surface. If a compatible `0.3.x` patch (including `0.3.1`) is available before implementation completes, the dependency floor MAY be raised to that patch.

#### Scenario: Published dependency resolution

- **WHEN** Cargo resolves the server dependency graph
- **THEN** `nb-api` resolves to a compatible `0.3.0` or later crates.io package
- **AND THEN** the dependency declaration contains no `path` or Git source

#### Scenario: Removed API compilation

- **WHEN** all server targets compile against `nb-api 0.3.0`
- **THEN** no removed `edit_note` or `EditMode` reference remains

### Requirement: Legacy edit surface removal

The server SHALL remove the legacy `edit` MCP tool (and its `EditMode` overwrite/append/prepend surface) from both the direct and multiplexed tool surfaces. The multiplexed `nb.edit` subcommand SHALL be rejected with a recovery-oriented message naming the replacement tools.

#### Scenario: Direct edit tool absent

- **WHEN** a client requests `tools/list`
- **THEN** the `edit` tool is not present
- **AND THEN** the replacement body-aware tools are present

#### Scenario: Multiplexed edit rejected

- **WHEN** a client invokes multiplexed `nb` with `command: "edit"`
- **THEN** the server rejects the request
- **AND THEN** the diagnostic names the replacement tools (`replace_note_body`, `edit_note_substring`, `edit_note_lines`, `retitle_note`, `edit_note_tags`)

### Requirement: Body-aware replacement tools

The server SHALL expose `replace_note_body`, `edit_note_substring`, `edit_note_lines`, `retitle_note`, and `edit_note_tags` as **direct-only** first-class MCP tools backed by the corresponding `NbClient` 0.3 methods. Their MCP parameter schemas SHALL present flat `id`/`selector` addressing and text-first content (the amended typed-schema contract), while internally preserving the 0.3 operation semantics (`NoteTarget` construction from the flat `id`, content bytes derived from UTF-8 text). The server MUST NOT expose multiplexed aliases for these tools.

#### Scenario: Flat selector addressing

- **WHEN** a client invokes any body-aware or line tool with an `id` (alias `selector`) string
- **THEN** the server addresses the note by that selector (constructing `NoteTarget::selector` internally)
- **AND THEN** the server does not expose or require the `NoteTarget` `path` variant or notebook-relative storage filenames

#### Scenario: Text-first body content

- **WHEN** a client passes `pattern`, `replacement`, `title`, `new_body`, or line-edit `content` as a plain string
- **THEN** the server treats it as UTF-8 bytes

#### Scenario: Non-UTF-8 body content rejected

- **WHEN** a client passes content (e.g., `new_body`, `pattern`, `replacement`, `title`, line-edit `content`) that is not valid UTF-8
- **THEN** the server rejects the request with an actionable validation error naming the field
- **AND THEN** the server does NOT offer a base64 opt-in flag; callers provide plain UTF-8 text

#### Scenario: Fingerprint-required body replacement

- **WHEN** a client invokes `replace_note_body` without a matching body fingerprint
- **THEN** the server rejects the request with fingerprint-mismatch guidance
- **AND THEN** the diagnostic tells the caller to re-read the note and retry with a fresh fingerprint

#### Scenario: Anchored line edits

- **WHEN** a client invokes `edit_note_lines` with a stale or out-of-range `LineRef`
- **THEN** the server rejects the request with anchor-mismatch guidance
- **AND THEN** the diagnostic tells the caller to re-read the note lines and retry

#### Scenario: Contiguous-body refusal

- **WHEN** a body-aware tool targets a multi-fragment document (e.g., a bookmark with multiple body fragments)
- **THEN** the server rejects the request with `FragmentedBody` guidance
- **AND THEN** the diagnostic notes that metadata operations (`retitle_note`, `edit_note_tags`) still apply

#### Scenario: No multiplexed alias

- **WHEN** a client invokes multiplexed `nb` with any of `replace_note_body`, `edit_note_substring`, `edit_note_lines`, `retitle_note`, or `edit_note_tags` as the command
- **THEN** the multiplexed surface does not route the command to a body-aware tool
- **AND THEN** the request is rejected with guidance to use the direct tool

### Requirement: Bounded line read and search

The server SHALL expose `show_note_lines` and `search_note_lines` as **direct-only** first-class MCP tools with bounded windowing and anchored results, consistent with the 0.3 `ShowNoteLines` and `SearchNoteLines` wire types. The server MUST NOT expose multiplexed aliases for these tools. Line and hit `text` fields SHALL be plain UTF-8 strings; `title` SHALL be an optional decoded string when present (`Option<String>`); the server SHALL decode nb-api's internal `ByteString` fields before serializing MCP results.

#### Scenario: Windowed line listing

- **WHEN** a client invokes `show_note_lines` with an offset and limit
- **THEN** the response returns body-relative line numbers, anchors, plain-string line `text`, and total/window metadata
- **AND THEN** out-of-range windows produce `InvalidLineWindow` guidance

#### Scenario: Anchored search results

- **WHEN** a client invokes `search_note_lines`
- **THEN** the response returns matching body lines with anchors, byte offsets, and plain-string `text`
- **AND THEN** empty patterns are rejected

### Requirement: Text-first structured show presentation

The `show` tool SHALL present a text-first structured envelope derived from `NbClient::show_note`: `selector`, `path`, `kind`, `todo_state`, `title` (Option<String>), `tags`, `body` (full note content decoded as UTF-8), `body_contiguous`, and `fingerprint`. The no-base64 guarantee is scoped to this change's textual read/edit MCP tool surface: the server SHALL NOT expose base64 fields (`source`, `body_fragments`, lossy `text`, `non_utf8`) in the MCP result; nb-api's internal `ByteString` fields SHALL be decoded to text before serialization. When the note source is not valid UTF-8, or the target is non-textual, `show` SHALL return a typed error carrying the detected content type and recovery guidance toward an external raw-retrieval facility outside MCP (this change adds no raw MCP tool) rather than base64 bytes.

#### Scenario: Structured show result

- **WHEN** a client invokes `show` on a valid-UTF-8 note
- **THEN** the response includes the note path, kind, fingerprint, decoded `title`, tags, and the full decoded `body`
- **AND THEN** no base64 `ByteString`, `source`, `body_fragments`, `text`, or `non_utf8` field is present in the result

#### Scenario: Non-UTF-8 source rejected

- **WHEN** a client invokes `show` on a note whose source is not valid UTF-8
- **THEN** the server returns a typed error carrying the detected content type and guidance toward an external raw-retrieval facility outside MCP (e.g. the `nb` CLI)
- **AND THEN** the server does NOT return base64-encoded content or a `non_utf8` flag

#### Scenario: Non-text target remains an error

- **WHEN** `show` resolves to a folder or non-text target
- **THEN** the server rejects the request with `UnsupportedShowTarget` guidance
- **AND THEN** the server does not silently route `show` to another command

### Requirement: Structured mutation outcomes

Mutating tools (`add`, `todo`, `bookmark`, `mkdir`, `delete`, `move`, `do`, `undo`, and the body-aware edit tools) SHALL present a structured `CommitOutcome` result including commit status, revision, and per-operation path/selector/noop/fingerprint, rather than raw `nb` stdout.

#### Scenario: Commit outcome fields

- **WHEN** a mutating tool succeeds
- **THEN** the response includes `commit_created`, `revision_id`, `pre_revision`, and per-operation outcomes
- **AND THEN** no raw `nb` CLI stdout is returned in place of the structured outcome

#### Scenario: Idempotent no-op mutation

- **WHEN** a mutating request results in no meaningful change
- **THEN** the response reports `commit_created: false`
- **AND THEN** per-operation outcomes mark the operation as `noop`

### Requirement: Typed error translation

The server SHALL translate the new typed `NbError` variants into concise recovery-oriented MCP diagnostics on both direct and multiplexed surfaces, including `DirtyBaseline`, `FingerprintMismatch`, `AnchorMismatch`, `OccurrenceMismatch`, `OverlappingEdits`, `InvalidLineWindow`, `EmptySubstringPattern`, `FragmentedBody`, `IndeterminateCommit`, `RecoveryRequired`, `GateTimeout`, `PathCollision`, `PathIgnored`, `PlanValidation`, `UnsupportedStructure`, and `UnsupportedShowTarget`.

#### Scenario: Dirty baseline refusal

- **WHEN** a mutation commits against a dirty notebook worktree/index
- **THEN** the server rejects the request with `DirtyBaseline` guidance
- **AND THEN** the diagnostic tells the caller to commit or clean the worktree before retrying

#### Scenario: Indeterminate commit

- **WHEN** a commit's completion is unknown
- **THEN** the server reports `IndeterminateCommit` guidance
- **AND THEN** the diagnostic instructs the caller not to auto-retry and to inspect HEAD/status

#### Scenario: Error parity across surfaces

- **WHEN** equivalent direct and multiplexed requests for a command available on BOTH surfaces produce the same typed `NbError`
- **THEN** both responses contain the same error wording and recovery guidance

### Requirement: Cross-surface behavioral parity for shared commands

The direct and multiplexed MCP surfaces SHALL apply identical validation, `nb-api` invocation, typed error translation, and output handling for equivalent operations on commands available on both surfaces (the retained commands). The new direct-only tools and the removed `edit` subcommand are outside this parity contract.

#### Scenario: Equivalent successful requests

- **WHEN** equivalent direct and multiplexed requests for a shared command succeed
- **THEN** both responses contain equivalent operation output

#### Scenario: Equivalent rejected requests

- **WHEN** equivalent direct and multiplexed requests for a shared command fail validation or return an `NbError`
- **THEN** both responses contain equivalent error status, wording, and guidance

### Requirement: Default gate timeout

The server SHALL construct `nb_api::Config` without overriding `gate_timeout`, using the nb-api default (60 seconds). The server MUST NOT expose a `gate_timeout` configuration surface in this change.

#### Scenario: Default gate timeout applied

- **WHEN** the server constructs `nb_api::Config`
- **THEN** the `gate_timeout` field retains the nb-api default
- **AND THEN** notebook-gated operations wait at most the default before returning `GateTimeout`
