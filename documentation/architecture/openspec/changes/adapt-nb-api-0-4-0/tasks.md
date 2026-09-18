## 1. Dependency bump

- [x] 1.1 Raise `nb-api` to `0.4.1` in `Cargo.toml` (main + dev-deps), drop `base64` dev-dep, run `cargo update -p nb-api` and record `Cargo.lock`
- [x] 1.2 Run `cargo build` to enumerate `ByteString` / `LineTerminator` / signature breakage as the work list

## 2. MCP surface rework

- [x] 2.1 Delete `ByteString` byte-decoding logic in `src/mcp.rs` (decodes inside `ShowEnvelope::from_show`, `from_wire` converters, `convert_mcp_line_edits` wrapping); retain the envelope conversion functions for field selection and title mapping — MCP `title` stays normalized via upstream `title_text` on `show` and via the retained normalization mapping over raw `ShowNoteLines.title` on `show_note_lines` (add normalized-title test for both paths)
- [x] 2.2 Reshape `show_note_lines` envelope to `eol` + `has_final_eol`, drop per-line `terminator` from schemas and results; leave `search_note_lines` envelope unchanged apart from `String` hit text (no EOL fields, no `numeric_id` upstream)
- [x] 2.3 Surface `selector` + `numeric_id` on `ShowNote`/`ShowNoteLines` reads and `CommitOutcome` ops where upstream provides them (`SearchNoteLines` has neither — pass through); accept `<folder>/<id>` / `<id>` selectors
- [x] 2.4 Update `src/nb.rs` re-exports (`ByteString`, `LineTerminator` out; `LineEol` in)
- [x] 2.5 Translate `NonUtf8` and `IndexLockTimeout` in the error adapter with raw-bytes / retry-later guidance

## 3. Add-versus-todo guard

- [x] 3.1 Reject todo-shaped `add.content` (checkbox `^\s*[-*]\s*\[[ xX]\]` outside fenced blocks, ATX `Tasks` heading outside fenced blocks) with `use todo` hint before `nb-api` invocation; cover unchecked, checked-only, fenced-checkbox-allowed, and fenced-Tasks-heading-allowed cases in tests
- [x] 3.2 Strengthen `add` / `todo` tool descriptions (`Note` vs `Todo` kind)

## 4. Tests and docs

- [x] 4.1 Rewrite `tests/integration/mcp_stdio.rs` + `real_nb_typed_errors.rs` for `String` wire, `eol` envelopes, numeric ids, mangled filenames, new errors
- [x] 4.2 Update `README.md`, `src/README.md`, `CHANGELOG.md` (single unreleased entry)
- [x] 4.3 Run `cargo fmt`, `clippy --all-targets --all-features -- -D warnings`, `cargo test --locked`, `cargo package --list`

## 5. Review readiness

- [ ] 5.1 `openspec validate adapt-nb-api-0-4-0 --strict` passes
- [ ] 5.2 Request Reviewer General implementation review + operator approval (no version bump/tag until approved)
