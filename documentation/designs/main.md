<!-- nbspec: change=adapt-nb-api-0-3 notebook=nb-mcp-server note=proposals/adapt-nb-api-0-3/designs/20260813235524.md hash=sha256:26b8917ce45db68621232b7dc1cc22f70110332e6c1910d6fadce51ba6f448f5 -->
# main

## Context

`nb-mcp-server 0.14.0` depends on `nb-api 0.2.1`. The 0.3 release removes the legacy editing surface entirely: `NbClient::edit_note` and `EditMode` are gone, `show_note` now returns a structured `ShowNote`, and mutating one-shots return `CommitOutcome`. Body-aware editing arrives through `replace_note_body` (fingerprint-required), `edit_note_substring`, `edit_note_lines`, `retitle_note`, and `edit_note_tags`, backed by a collect-then-commit `Transaction`. The MCP server has two dispatch surfaces (direct first-class tools and the multiplexed `nb` tool) that must present identical behavior for commands available on both surfaces, so a local adaptation that updates only one path would create externally visible drift.

The migration is a hard pre-1.0 surface change by design: `todos/mcp/55` documents that two partial-correction attempts accidentally replaced entire notes, and the new surface makes destructive replacement explicit, requires source fingerprints, provides targeted alternatives, and returns verification evidence.

nb-api 0.3's structured wire types (`ShowNote`, `ShowNoteLines`, `SearchNoteLines`, `LineEdit`) serialize note content as base64 `ByteString` fields. That is a library-internal representation; the MCP boundary MUST NOT expose it to LLM consumers. This design decodes to text at the MCP layer and rejects non-UTF-8 content with a typed, MIME-aware error. Making nb-api's own types text-first is an upstream follow-up with the NbApi Owner, tracked out of scope here.

## Goals / Non-Goals

**Goals:**

- Compile and test solely against the published `nb-api 0.3.0` package (floor may rise to `0.3.1` for Windows fixes).
- Remove the legacy `edit`/`EditMode` tool and its prepend/append convenience modes from both MCP surfaces.
- Expose the body-aware replacement tools: `replace_note_body`, `edit_note_substring`, `edit_note_lines`, `retitle_note`, `edit_note_tags` — direct-only.
- Add bounded `show_note_lines` and anchored `search_note_lines` schemas — direct-only.
- Present a **text-first** `show` envelope and text-first line/search results, with NO base64 on this change's textual read/edit MCP tool surface (future raw-bytes MCP tool explicitly outside this guarantee).
- Translate the new typed conditional failures into recovery-oriented MCP diagnostics.
- Preserve direct/multiplexed equivalence for commands available on both surfaces and existing selector routing for the retained tools.
- Use `nb_api`'s default `gate_timeout`; add no server configuration knob.

**Non-Goals:**

- History/recovery tools (`nb-api:ideas/3`) — not in 0.3.
- `list_todos` (deferred upstream).
- Cross-process gate / `index.lock` coordination.
- A `gate_timeout` configuration knob.
- Multiplexed aliases for the new direct-only tools.
- Making nb-api's structured wire types text-first upstream (NbApi Owner follow-up).
- MCPB packaging or registry work (`todos/mcp/56`).
- Reorganizing the notebook or retired OpenSpec archive.

## Tool Surface Transition

| 0.2.x MCP surface | 0.3 MCP surface | `NbClient` backing |
|---|---|---|
| `edit` (mode: overwrite/append/prepend) | **removed** | — |
| (new, direct-only) | `replace_note_body` | `replace_note_body(target, new_body, fingerprint)` |
| (new, direct-only) | `edit_note_substring` | `edit_note_substring(target, pattern, replacement, occurrence, expected_count, fingerprint?)` |
| (new, direct-only) | `edit_note_lines` | `edit_note_lines(target, edits)` |
| (new, direct-only) | `retitle_note` | `retitle_note(target, title)` |
| (new, direct-only) | `edit_note_tags` | `edit_note_tags(target, add, remove)` |
| `show` (text) | `show` (text-first structured envelope) | `show_note(id, notebook)` |
| (new, direct-only) | `show_note_lines` | `show_note_lines(target, offset, limit, notebook)` |
| (new, direct-only) | `search_note_lines` | `search_note_lines(target, pattern, notebook)` |
| `add`/`todo`/`bookmark`/`mkdir`/`delete`/`move`/`do`/`undo` | same names, `CommitOutcome` result | corresponding 0.3 one-shot |
| `import` | unchanged (one-shot, `String`) | `import_note` |
| `status`/`notebooks`/`list`/`search`/`tasks`/`folders` | unchanged (`String`) | 0.3 reads |

## Design Decisions

- **Remove `edit` entirely; do not keep a compat alias.** The 0.2 migration kept `edit` because the 0.2 API still had `edit_note`; 0.3 removes the API method, so the MCP tool must be removed too. Callers choose the replacement tool explicitly. This matches the `todos/mcp/55` hard-migration contract.
- **Multiplexed surface policy (F1).** The new body-aware, line, and metadata tools are **direct-only**; no `nb.*` aliases are added for them. The multiplexed `nb` tool keeps compact/backcompat routing for the retained commands only. Cross-surface parity is defined only for commands available on BOTH surfaces (the retained commands); the new tools have no multiplexed form, so no parity contract or parity tests exist for them. The removed `edit` subcommand is rejected on the multiplexed path with a recovery-oriented message naming the replacement direct tools. This keeps the multiplexed surface small and backcompat while new capabilities land direct-only.
- **Target addressing (Option A).** Body-aware and line tools address notes by a flat `id` (alias `selector`) string, exactly like `show`, `delete`, and `move`. The server internally constructs `NoteTarget::selector(id)`. The `NoteTarget` `path` variant is NOT exposed on the MCP surface: notebook-relative storage filenames are not knowable by agents, and the qualified selectors returned by `show` / `show_note_lines` / `search_note_lines` round-trip directly as `id`. This keeps one addressing idiom across the whole tool surface and removes the unusable `path` affordance.

- **Text-first content and results; NO base64 on this change's textual read/edit MCP tool surface.** Body-content parameters are plain UTF-8 strings — the common, ergonomic form. `pattern`, `replacement`, `title`, `new_body`, and line-edit `content` are accepted as plain text, never as base64. `replace_note_body.new_body` is a plain UTF-8 string with NO `new_body_base64` opt-in; a body that is not valid UTF-8 is rejected with an actionable validation error. On the read side, `show` returns the note `body` and `title` as decoded UTF-8 text; `show_note_lines` returns `lines[].text` and `title` as strings; `search_note_lines` returns `hits[].text` as strings. The server decodes nb-api's internal `ByteString` fields to text before serializing MCP results; it never forwards a base64 field on this change's textual read/edit tool surface. This keeps byte-fidelity reachable upstream while making every MCP tool friction-free for agents.
- **Fingerprint requirement.** `replace_note_body` requires the body `Fingerprint` from a preceding `show`. `edit_note_substring` makes the fingerprint optional but requires `expected_count`. `edit_note_lines` verifies `LineRef` anchors against the original snapshot. The MCP tools surface these as required parameters where the API requires them.
- **Structured outcomes.** Mutating tools return a structured result composed from `CommitOutcome` (commit_created, revision_id, pre_revision, per-op path/selector/noop/fingerprint), not raw `nb` stdout.
- **Text-first `show` envelope (F2).** The MCP `show` result is a text-first envelope: `{ selector, path, kind, todo_state, title: Option<String>, tags, body: String, body_contiguous, fingerprint }`. `body` is the full note content decoded as UTF-8; `title` is decoded text. NO `source`, `body_fragments`, `text`-as-lossy, or `non_utf8` base64 fields exist on this change's textual read/edit MCP tool surface. When `ShowNote.source` is not valid UTF-8, `show` returns a typed `UnsupportedShowTarget`-style error carrying the detected content type and guidance toward an external raw-retrieval facility outside MCP (the `nb` CLI; this change adds no raw MCP tool); it never returns base64 bytes. The exact envelope is defined in the specification scenarios.
- **Text-first line/search results (F2b).** `show_note_lines` returns `{ selector, path, kind, total_lines, offset, limit, next_offset, lines: [{number, anchor, text, terminator}], title: Option<String>, body_fingerprint }` with `text` and `title` as plain strings. `search_note_lines` returns `{ selector, path, kind, hits: [{number, anchor, start_byte, end_byte, text}], body_fingerprint }` with `text` as a plain string. nb-api's `ByteString` line/hit fields are decoded before serialization.
- **Error translation.** Add a shared result adapter converting `Result<T, NbError>` to MCP responses, pattern-matching the new typed variants into concise recovery guidance:
  - `DirtyBaseline`: tell the caller to commit or clean the notebook worktree/index before retrying.
  - `FingerprintMismatch`/`AnchorMismatch`/`OccurrenceMismatch`: tell the caller to re-read the note and retry with fresh fingerprint/anchors.
  - `FragmentedBody`: note the body is multi-fragment; metadata ops still apply.
  - `IndeterminateCommit`/`RecoveryRequired`: do not auto-retry; inspect HEAD/status before acting.
  - `GateTimeout`: retry later; the notebook is busy.
  - `PathCollision`/`PathIgnored`/`UnsupportedStructure`/`PlanValidation`: surface the specific path/plan guidance.
  Both dispatch surfaces use this adapter so parity is structural for shared commands.
- **Config (F3).** No server configuration surface for `gate_timeout` in this change. The server constructs `nb_api::Config` with the default `gate_timeout` (60s). A config knob is deferred to a follow-up change with a fully specified env/flag, duration grammar, precedence, and invalid-value handling.
- **Keep transport tests cheap; add real-`nb` tests narrowly.** Shim-based stdio tests remain the primary schema/dispatch/parity suite. Enable `nb-api`'s `testing` feature for integration targets that must prove fingerprint mismatch, anchor mismatch, dirty-baseline refusal, contiguous-body refusal, and commit outcomes against a real `nb` fixture.

## Verification Strategy

- Compile against `nb-api 0.3.0` to prove all call sites migrated; assert no `edit_note`/`EditMode` references remain.
- Assert `tools/list` no longer advertises `edit` and does advertise the new direct-only tools with typed schemas.
- Assert multiplexed `nb.edit` is rejected with recovery guidance naming the replacement tools.
- Assert no multiplexed aliases exist for the new tools, and parity tests cover only commands shared by both surfaces.
- Assert `show` returns the text-first envelope: decoded `body`/`title` strings, no base64 fields, correct `fingerprint`; a non-UTF-8 source produces a typed error with content-type guidance to an external facility, not base64.
- Assert `show_note_lines` and `search_note_lines` return plain-string `text`/`title` fields with correct anchors, numbers, and window metadata.
- Assert mutating tools present `CommitOutcome` fields (revision, per-op noop/fingerprint).
- Exercise fingerprint/anchor/occurrence/contiguous-body preconditions and the new typed errors with actionable parity for shared commands.
- Exercise `DirtyBaseline` and `IndeterminateCommit`/`RecoveryRequired` mapping with real-`nb` fixtures.
- Retain selector-routing, MiMo schema-compatibility, and cross-surface regressions.
- Run formatting, Clippy with warnings denied, locked unit/integration tests, package construction, packaged-binary smoke tests, and CI before release.

## Release Plan

1. Approve and merge the Nbspec durable specification/design.
2. Implement the dependency and call-site migration.
3. Implement the tool-surface transition (removal + direct-only new tools), structured outcomes, error adapter, and regressions.
4. Update `README.md` and `src/README.md`.
5. Bump the server version and validate the crate assembled from the published dependency.
6. Obtain implementation review and operator release approval.
7. Tag and publish, then verify crates.io installation and GitHub release state.

Rollback before publication is a normal commit revert. After publication, defects receive a forward patch release; the published artifact and tag are never rewritten.

## Risks / Trade-offs

- Removing `edit` is an intentional request-shape break. The safety benefit is the point of `todos/mcp/55`; recovery guidance names the replacement tools.
- New tools are direct-only in v1; multiplexed callers must use the direct surface. This is a deliberate narrowing; multiplexed aliases can be added in a follow-up if a need emerges.
- Changing `show` and mutation responses to structured, text-first shapes breaks callers parsing raw text. The text-first envelope is byte-exact for valid-UTF-8 notes; non-UTF-8 content is surfaced as a typed, MIME-aware error instead of opaque base64.
- nb-api's wire types remain base64 `ByteString` upstream; the MCP layer decodes them. NbApi Owner confirmed (2026-08-18) text-first structured types are the flagship 0.4.0 item (`String` fields, typed `NonUtf8` error, `Vec<u8>` escape-hatch accessor, `ByteString` removal); this change does not wait on 0.4.0. Boundary split: the library is base64-free; base64 remains a wire-layer transport encoding for the escape-hatch path only.
- Real-`nb` fixture tests depend on an installed `nb` executable; CI already installs a pinned `nb`. Tests that do not need native semantics remain shim-based.
