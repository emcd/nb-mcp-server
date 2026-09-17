## Context

`master` is 5 commits ahead of `origin/master` with an unshipped `nb-api 0.3.1` adaptation (`ba9567d` text-first flat-`id` surface, `42e15a9` review fixes, `38ab50c` doc merge). That work decodes `nb-api::ByteString { base64 }` to `String` at the MCP boundary (`src/mcp.rs:520-775`), exposes per-line `terminator`, assumes opaque `{epoch}-{seq}.md` filenames, and synthesizes a `ValidationError` MIME shim for non-UTF-8. `Cargo.toml:32,61` pins `nb-api 0.3.1`.

`nb-api 0.4.1` (crates.io, tag `v0.4.1`; vendor copy in `.auxiliary/temporary/nb-api-0.4.1/`) removes `ByteString` entirely (crate is base64-free), makes all structured text native `String`, replaces `LineTerminator`/`NoteLine.terminator` with document-level `eol: Option<LineEol>` + `has_final_eol: bool`, maintains folder `.index` files with positional numeric ids (`OpOutcome.numeric_id`, `ShowNote.numeric_id`, `ShowNoteLines.numeric_id`, numeric `selector` — including created folders per the `0.4.1` `add_folder` fix), mangles create filenames nb-faithfully, and adds typed `NonUtf8` / `IndexLockTimeout` errors plus `read_note_source_bytes`. Migration contract: `documentation/migration-0.4.0.md` in the vendor copy. Operator direction: ship nothing on `0.3.1`; rework in place, single release. Stakeholders: NbApi Owner (upstream done), Reviewer General + operator (approval gate).

## Goals / Non-Goals

**Goals:**
- Compile solely against published `nb-api 0.4.1`; delete every `ByteString` import, decode path, and `base64` dev-dep.
- Reshape the `show_note_lines` envelope to `eol` + `has_final_eol`, bare-text `LineEdit.content` with library-appended EOL (eliminates the smoke-test terminator-merge class). `search_note_lines` is unchanged apart from `String` hit text (upstream exposes no EOL fields and no `numeric_id` on that type).
- Surface numeric `.index` identity (read + outcome) while keeping path canonical; accept `<folder>/<id>` / `<id>` selectors.
- Translate `NonUtf8` / `IndexLockTimeout` into recovery-oriented MCP diagnostics; point raw-bytes needs at `read_note_source_bytes` via external `nb` CLI guidance (no new MCP raw tool).
- Fold in agreed `add`-vs-`todo` misuse guard + description hardening.
- Update docs, changelog, and regression suites; leave version bump/tag/publish to the release step after review.

**Non-Goals:**
- A raw-bytes MCP tool; multiplexed aliases for direct-only tools; `gate_timeout` knob; history/recovery tools; MCPB/registry work.
- Maintaining `.index` ourselves or writing numeric ids pre-listing (upstream owns it; we resolve).
- Rewriting published history or the `0.3.1` tag decision beyond discarding unshipped shims.

## Decisions

- **Delete byte decoding, keep envelope mapping.** Remove the `ByteString` decode steps inside `ShowEnvelope::from_show`, `ShowNoteLinesEnvelope::from_wire` / `SearchNoteLinesEnvelope::from_wire`, and the `convert_mcp_line_edits` `ByteString::from_bytes` wrapping — but RETAIN the envelope conversion functions themselves for field selection and title mapping. Pass `String`/`&str` straight through, EXCEPT title: MCP `title` MUST stay normalized plain text on both paths — `show` maps upstream `ShowNote.title_text`, while `show_note_lines` retains its normalization mapping over the raw `ShowNoteLines.title` H1 line (upstream `ShowNoteLines` exposes no `title_text`; verified in `src/types.rs:167-187`, `src/client/read.rs:201-250`). Only base64 byte decoding is deleted. *Alternative (compat decode accepting both `String` and `{base64}`)* rejected: 0.4.x deserialization of old object form fails by design; dual-accept would reintroduce the exact opacity being removed.
- **Keep `deserialize_plain_string_*` for inputs.** They now guard against `null`/number/array/object on `new_body`/`pattern`/`replacement`/`title`/`content`, not against `ByteString` objects. No special-case messaging for unshipped shapes.
- **Expose `eol` verbatim, hide nothing.** `show_note_lines` returns `eol: "lf"|"crlf"|null` + `has_final_eol: bool`, no per-line field. `eol: null` = empty / single-line-no-EOL / CR-only (binary already rejected via `NonUtf8`). Bare `\r` stays in `text` verbatim per upstream first-occurrence rule.
- **Path canonical, numeric secondary.** Return upstream `selector` (numeric `<notebook>:<id>` when resolvable on `ShowNote`/`ShowNoteLines`/`OpOutcome`) alongside `path` and pinned `numeric_id`. `SearchNoteLines` exposes neither EOL fields nor `numeric_id` upstream and is passed through unchanged apart from `String` text. Accept numeric selectors through existing `NoteTarget::selector` path (upstream resolves via `.index`; blanks → `NotFound`). Never construct `.index` lines or allocate ids client-side.
- **Error mapping is total.** `NonUtf8 { selector, path, kind, mime_hint }` → typed MCP error with `mime_hint` + `nb show <selector> --print --no-color` guidance; `IndexLockTimeout { path, timeout_ms }` → retry-later diagnostic with path/timeout. Old `UnsupportedShowTarget` path unchanged for non-textual targets.
- **Misuse guard lives in MCP, not API.** `add` rejects todo-shaped content before calling `nb-api`: any line matching `^\s*[-*]\s*\[[ xX]\]` outside fenced code blocks (delimited by ```` ``` ```` or `~~~` fences; inline code spans are NOT excluded) or any ATX heading matching `^\s*#{1,6}\s+Tasks\s*$` outside fenced code blocks, naming `todo`. Upstream `Transaction::add_note` stays untouched. Strengthen `add` ("Do NOT use for todos") / `todo` descriptions in tool schemas.

## Risks / Trade-offs

- [Risk] `0.3.1`-era tests asserting `ByteString` rejection / `terminator` fields / `{epoch}-{seq}.md` names fail after bump → Mitigation: rewrite `tests/integration/mcp_stdio.rs` + `real_nb_typed_errors.rs` expectations in the same commit; drop `base64` dev-dep.
- [Risk] Numeric selectors surprise LLM callers used to path-ids → Mitigation: return both `selector` and `path` + `numeric_id`; document positional/never-reused semantics; keep path accepted everywhere.
- [Risk] Same-folder rename-to-longer refused at plan validation; longer-name edit flows break → Mitigation: surface upstream guidance verbatim (delete + recreate, new id).
- [Risk] `eol: null` + `Dollar`-insert edge semantics (`Caret`-only on empty body) confuse agents → Mitigation: map upstream `ValidationError` with boundary guidance; cover in integration tests.
- [Risk] `.index` lock contention surfaces new `IndexLockTimeout` in normal use → Mitigation: retry-later diagnostic, no auto-retry; not a dirty-baseline conflation.

## Migration Plan

1. `cargo update -p nb-api --precise 0.4.1` (manifest floor `0.4.1`), `cargo build` to enumerate breakage.
2. Apply `src/mcp.rs` + `src/nb.rs` rework, error adapter, descriptions + `add` guard, docs.
3. Rewrite integration expectations; `cargo fmt`, `clippy --all-targets --all-features -- -D warnings`, `cargo test --locked`, `cargo package --list`.
4. Review (Reviewer General) + operator approval, then version bump/tag/publish as the single release. Rollback pre-publish is revert; post-publish is forward patch.

## Open Questions

- None blocking. Residual upstream micro-window on non-atomic `.index` range removal (non-Linux) is documented upstream and out of scope.
