## ADDED Requirements

### Requirement: Text-first String wire

The system SHALL consume `nb-api 0.4.1` structured text as native JSON `String` with no `ByteString` handling. Fields `ShowNote.body`/`source` (`String`), `BodyFragment.bytes` (`String` with `start_byte`/`end_byte` offsets into `source`), `NoteLine.text` (`String`), `NoteLineHit.text` (`Option<String>`), `LineEdit` Insert/Replace `content` (`String`), substring `pattern`/`replacement` (`String`), and `replace_note_body`/`retitle_note`/`search_note_lines` (`&str`) SHALL pass through without base64 encode or decode. MCP `title` SHALL stay normalized plain text on both read paths, sourced differently per upstream type: `show` SHALL map upstream `ShowNote.title_text` (existing trim behavior preserved) and SHALL NOT expose the raw `ShowNote.title` H1 line (leading `#`, trailing newline); `show_note_lines` SHALL retain a small normalization mapping over the raw `ShowNoteLines.title` (`Option<String>`, raw H1 — upstream exposes no `title_text` on that type) applying the same trim, since the envelope conversion functions remain necessary for field selection and title mapping. Only the base64 byte-decoding logic is deleted. The crate SHALL carry no `base64` dependency for structured text.

#### Scenario: Native string show

- **WHEN** a client invokes `show` on a valid-UTF-8 note
- **THEN** the response carries decoded `body`/`title` strings produced without any base64 step
- **AND THEN** no `base64`, `source`-as-bytes, or `body_fragments`-as-bytes field appears in the MCP result

#### Scenario: MCP title stays normalized

- **WHEN** a client invokes `show` on a note titled `# New Title`
- **THEN** the MCP `title` is `"New Title"` sourced from upstream `title_text`, not the raw `"# New Title\n"` H1 line

#### Scenario: Show-note-lines title stays normalized

- **WHEN** a client invokes `show_note_lines` on a note titled `# New Title`
- **THEN** the MCP `title` is `"New Title"` via the retained normalization mapping over raw `ShowNoteLines.title`, matching the `show` path

#### Scenario: Old base64 object form rejected

- **WHEN** a caller submits the removed `{ "base64": "..." }` object form for any text field
- **THEN** deserialization fails with a plain-UTF-8-string validation message naming the field

### Requirement: Document-level EOL contract

`show_note_lines` SHALL declare `eol: Option<LineEol>` (`lf`/`crlf`, first supported EOL scanning left-to-right) plus `has_final_eol: bool` instead of any per-line terminator. `NoteLine` SHALL carry only `number`/`anchor`/`text` (line without terminator); bare `\r` SHALL remain verbatim in `text`. `eol: None` SHALL mean empty, single-line-no-EOL, or CR-only. `LineEdit.content` SHALL be bare text with the library appending document `eol`; `Insert` at `Dollar` (or `After` the final line) on a final line lacking EOL SHALL first terminate it, then append `content + eol`; `Insert` at `Dollar` on an empty body SHALL be rejected (`Caret` only); edits materializing a boundary on an `eol: None` document SHALL adopt `Lf`.

#### Scenario: LF document listing

- **WHEN** a client invokes `show_note_lines` on an LF note ending with newline
- **THEN** the response carries `eol: "lf"`, `has_final_eol: true`, and lines without per-line terminator fields

#### Scenario: Bare-text replace preserves line breaks

- **WHEN** a client replaces a single interior line with bare-text `content` lacking terminator bytes
- **THEN** the following line remains a separate line (no `LINE TWOline three` merge)

#### Scenario: Dollar insert on unterminated final line

- **WHEN** a client inserts at `Dollar` when the final line lacks EOL
- **THEN** the final line is terminated first and `content + eol` is appended

### Requirement: Numeric identity alongside path

Reads (`ShowNote`, `ShowNoteLines`) and mutation outcomes (`OpOutcome`) SHALL expose upstream numeric `.index` identity as `selector` (`<notebook>:<id>` or `<notebook>:<folder>/<id>`) plus pinned `numeric_id: Option<u32>` whenever resolvable via `.index` scan. `SearchNoteLines` exposes neither EOL fields nor `numeric_id` upstream and SHALL pass through unchanged apart from `String` hit text — no enrichment lookup is required. Path SHALL remain canonical and accepted everywhere; numeric ids SHALL be positional line numbers, never reused (deletes blank the line, within-folder moves update in place, cross-folder moves blank source + append destination). `NoteTarget::Selector` SHALL accept `<folder>/<id>` / `<id>` (blank lines resolve to `NotFound`). One-shot creates SHALL use nb-faithful mangled filenames (ASCII lowercased, non-ASCII preserved, other-ASCII runs collapsed to `_`, collisions `-1`/`-2` retries; titleless `%Y%m%d%H%M%S.md`); the opaque `{epoch}-{seq}.md` scheme SHALL be gone.

#### Scenario: Numeric selector round-trip

- **WHEN** a create returns a numeric `selector` + `numeric_id`
- **THEN** a subsequent `show` with that numeric `id` resolves to the same note as the path form

#### Scenario: Mangled create filename

- **WHEN** a client creates a note titled `Hello World Title`
- **THEN** the outcome path is `hello_world_title.md` (collision retries `-1`, `-2`)

### Requirement: Typed NonUtf8 and IndexLockTimeout handling

The system SHALL translate `NonUtf8 { selector, path, kind, mime_hint }` into a typed MCP error carrying the MIME hint and `read_note_source_bytes` / external `nb show <selector> --print --no-color` guidance (never base64 bytes); non-textual targets SHALL keep returning `UnsupportedShowTarget`. The system SHALL translate `IndexLockTimeout { path, timeout_ms }` into a retry-later diagnostic naming path and timeout. No raw-bytes MCP tool SHALL be added; `NbClient::read_note_source_bytes` remains the single escape hatch outside MCP.

#### Scenario: Non-UTF-8 text diagnostic

- **WHEN** `show` targets a non-UTF-8-but-text file
- **THEN** the MCP error names `mime_hint` and directs raw retrieval outside MCP

#### Scenario: Index lock contention

- **WHEN** the `.index` read-append lock cannot be acquired
- **THEN** the MCP error reports path + `timeout_ms` with retry-later guidance and no auto-retry

### Requirement: Add-versus-todo misuse guard

`add` SHALL reject todo-shaped content before invoking `nb-api`: any line matching `^\s*[-*]\s*\[[ xX]\]` (unchecked box, checked `x`/`X`, any leading whitespace, `-` or `*` marker) outside fenced code blocks (spans delimited by ```` ``` ```` or `~~~` fences; inline code spans do NOT exempt a line), or any ATX heading matching `^\s*#{1,6}\s+Tasks\s*$` outside fenced code blocks, with a diagnostic naming `todo` as the correct tool. Tool descriptions SHALL state `add` creates `DocumentKind::Note` (never use for todos/checklists) and `todo` creates `DocumentKind::Todo`.

#### Scenario: Todo-shaped add rejected

- **WHEN** a client calls `add` with content containing `- [ ] buy milk`
- **THEN** the request is rejected with a `use todo, not add` hint and no note is created

#### Scenario: Checked-only checkbox add rejected

- **WHEN** a client calls `add` with content containing only `- [x] done` and no unchecked box
- **THEN** the request is rejected with a `use todo, not add` hint and no note is created

#### Scenario: Fenced checkbox add allowed

- **WHEN** a client calls `add` with content where the only checkbox-like line sits inside a fenced code block
- **THEN** the request is accepted and the note is created
