## Why

`nb-api 0.4.1` is a breaking pre-1.0 release that removes `ByteString`, declares line endings once per document, and makes notebook identity nb-faithful (mangled filenames + positional `.index` numeric ids, including created folders). Our unshipped `0.3.1` adaptation (`ba9567d`, `42e15a9`) carries `ByteString`-decode shims, per-line `terminator` envelopes, and opaque `{epoch}-{seq}.md` assumptions that are now dead code. Per operator direction we ship nothing on `0.3.1`; we rebuild on `0.4.1` before any release.

## What Changes

- **BREAKING**: Raise `nb-api` floor `0.3.1` → `0.4.1` in `Cargo.toml` (main + dev-deps); remove `base64` dev-dep used only for `0.3.1` byte-exact assertions.
- **BREAKING**: Delete `ByteString` byte-decoding logic in `src/mcp.rs` (`ShowEnvelope::from_show`, `ShowNoteLinesEnvelope::from_wire`, `SearchNoteLinesEnvelope::from_wire` decodes, `convert_mcp_line_edits` wrapping; `deserialize_plain_string_*` guards stay for input validation only); consume native `String` fields (`body`, `source`, `NoteLine.text`, `NoteLineHit.text`, `LineEdit.content`, substring `pattern`/`replacement`, `&str` one-shots). MCP `title` stays normalized plain text on both read paths: `show` maps upstream `title_text`, while `show_note_lines` retains its normalization mapping over raw `ShowNoteLines.title` (upstream exposes no `title_text` there) — the envelope conversion functions remain for field selection and title mapping; only byte decoding is deleted. The raw `title` H1 line (leading `#`, trailing newline) is NOT exposed on either path.
- **BREAKING**: Replace per-line `terminator` with document-level `eol: Option<LineEol>` (`lf`/`crlf`) + `has_final_eol: bool` on `show_note_lines`; `LineEdit.content` becomes bare text with library-appended `eol` (fixes the smoke-test `LINE TWOline three` merge class).
- Surface numeric `.index` identity: expose `selector` + `numeric_id` from `ShowNote`/`ShowNoteLines`/`OpOutcome` where upstream provides them (created folders occupy a numeric id per the `0.4.1` `add_folder` fix); `SearchNoteLines` carries neither EOL fields nor `numeric_id` upstream and stays unchanged apart from `String` hit text. Accept `<folder>/<id>` / `<id>` via `NoteTarget::Selector`; keep path as canonical, numeric as resolved convenience.
- Adopt nb-faithful creates: mangled `hello_world_title.md` (`-1`, `-2` retries), titleless `%Y%m%d%H%M%S.md`; drop `{epoch}-{seq}.md` test fixtures.
- Handle new typed errors `NonUtf8 { selector, path, kind, mime_hint }` (replaces our ad-hoc `ValidationError` MIME shim) and `IndexLockTimeout { path, timeout_ms }`; route raw bytes to `read_note_source_bytes` guidance outside MCP (no new raw MCP tool).
- Harden `add` vs `todo`: reject todo-shaped `add.content` (unchecked `- [ ]` and checked `- [x]`/`- [X]` checkbox lines, or a `## Tasks` heading — both outside fenced code blocks) with `use todo` hint; strengthen `add`/`todo` tool descriptions.
- Update `README.md` / `src/README.md`, `CHANGELOG.md` (single unreleased entry, no `0.15.0` tag until implementation lands).

## Capabilities

### New Capabilities

- `nb-api-0-4-wire`: text-first `String` wire surface, document-level `eol` + `has_final_eol` line contract with bare-text edits, numeric selector/id exposure, mangled-create filenames, `NonUtf8`/`IndexLockTimeout` diagnostics with `read_note_source_bytes` guidance, and `add`-vs-`todo` misuse guard.

### Modified Capabilities

- `tool-surface`: `show_note_lines` envelope drops per-line `terminator`, carries `eol`/`has_final_eol` + `numeric_id`; `search_note_lines` envelope is unchanged apart from `String` hit text (no EOL fields, no `numeric_id` upstream); mutation outcomes carry numeric selectors; error translation covers new variants.
- `nb-api`: dependency floor `0.3.1` → `0.4.1`, `ByteString` removal, `&str` signatures, `.index` maintenance assumptions (incl. `0.4.1` `add_folder` parent-`.index` recording).

## Impact

- `src/mcp.rs` (envelopes, schemas, dispatch, error adapter), `src/nb.rs` re-exports (`ByteString`, `LineTerminator` → `LineEol`), `Cargo.toml`/`Cargo.lock`, `tests/integration` (stdio + real-nb typed-error suites, probe artifacts), docs (`README.md`, `src/README.md`, `CHANGELOG.md`).
- Discards unshipped `0.3.1` shim code before release; no published artifact or tag is rewritten. The 5 unpushed `0.3.1` commits remain as base history; this change reworks them in place.
- Upstream coordination: NbApi Owner `0.4.x` text-first/identity threads close out (`0.4.1` adds the `add_folder` parent-`.index` fix); Reviewer General + operator approval required before implementation apply.
