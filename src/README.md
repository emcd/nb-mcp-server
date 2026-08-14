# MCP Surface Contract

This server intentionally exposes a curated MCP API rather than the full native
`nb` CLI grammar. Native `nb` often accepts compact positional selectors that
combine notebook, folder, and item identity. That shape is convenient for humans
at a shell, but it is easy for agents to misuse.

## Routing Fields

Use separate fields for routing whenever a command creates, lists, searches, or
organizes notebook content:

- `notebook` is a bare notebook name only.
- `folder` is a folder path only.
- `id` / `selector` identifies an existing note, todo, or item.

`notebook` must not contain selector syntax such as `project:todos/mcp`.
`folder` must not contain notebook-qualified syntax such as
`project:todos/mcp`.

## Existing Items

Commands that operate on existing items may accept copied native `nb` selectors
in the `id` field, with `selector` as an alias. Examples include:

- `nb.show`
- `nb.delete`
- `nb.move` source item
- `nb.do`
- `nb.undo`

For these commands, values like `project:todos/mcp/35` are useful because they
come directly from `nb` output and refer to an existing item. If a copied
selector includes a notebook and the separate `notebook` field is also present,
the values must agree. Conflicts are rejected instead of guessed.

Embedded notebook selectors are checked as existing notebooks and must not
auto-create notebooks. This prevents typoed titles or selector-shaped strings
from creating stray notebooks.

## Creation Commands

Commands that create new items must use explicit routing fields:

- `nb.add`
- `nb.todo`
- `nb.bookmark`
- `nb.import`

These commands require `folder` by default. They may accept a bare `notebook`
override for cross-notebook collaboration, but they must not accept native
all-in-one selector syntax as a creation destination.

Do not add `selector` as an alias for creation destinations. If an agent passes
`selector` to a creation command, that should be an unknown-argument error.

## Filenames

Do not expose explicit filename control for new notes unless a concrete use case
requires it. Agents should normally address notes by the `nb`-generated selector
or id, not by storage filenames. Filename control increases overlap with
`title`, `folder`, and selector-like paths, making routing mistakes more likely.

`nb.import` may keep its `filename` argument because imported artifacts commonly
have meaningful external filenames.

## Unknown Arguments

Unknown command arguments must be rejected instead of ignored. Silent ignores
turn spelling mistakes and native-CLI assumptions into misplaced notes.

Keep intentional aliases narrow and documented. Current aliases exist for common
agent ergonomics, not for reproducing every native `nb` spelling:

- `selector` -> `id` on existing-item commands.
- `content` -> `description` on `nb.todo`.
- `folder` -> `parent` on `nb.folders`.
- `folder` -> `path` on `nb.mkdir`.
- `state` -> `status` on `nb.tasks`.
- `recurse` -> `recursive` on `nb.tasks`.
- `task` -> `task_number` on `nb.do` and `nb.undo`.

## First-Class Tools

The server also registers direct first-class tools for all subcommands:
`status`, `notebooks`, `add`, `show`, `delete`, `move`, `list`,
`search`, `todo`, `do`, `undo`, `tasks`, `bookmark`, `folders`, `mkdir`,
`import`, plus the body-aware tools `replace_note_body`,
`edit_note_substring`, `edit_note_lines`, `retitle_note`,
`edit_note_tags`, and the line tools `show_note_lines`,
`search_note_lines`. These bypass the multiplexed `nb` command dispatch
and expose typed schemas directly. The multiplexed `nb` tool remains as
the compact/backcompat compatibility surface for the retained commands.

The legacy `edit` tool and its `EditMode` (`overwrite`/`append`/`prepend`)
surface are removed. Multiplexed `nb.edit` is rejected with recovery
guidance naming the replacement body-aware tools. The body-aware and line
tools are **direct-only**: they have no multiplexed aliases, and invoking
them through the multiplexed `nb` tool is rejected.

Note: MCP clients may display these tools with server-prefixed names (e.g.,
`nb_add` instead of `add`). The server-registered names are unprefixed.

## Design Principle

Prefer one obvious way for agents to say where new content goes. Preserve copied
native selectors only for operations on existing content, where they reduce
friction without creating new routing ambiguity.

## Body-Aware Editing

The body-aware tools address notes by `target`, mirroring the `nb-api 0.3`
`NoteTarget` wire type:

- `replace_note_body` requires the body `fingerprint` from a preceding
  `show`; a stale fingerprint is rejected so a caller cannot overwrite a
  note it has not just read.
- `edit_note_substring` replaces `pattern` occurrences; `expected_count`
  must match, and an optional fingerprint guards against stale edits.
- `edit_note_lines` applies a batch of disjoint `edits` verified against
  line anchors from one original snapshot.
- `retitle_note` changes the title without changing the path.
- `edit_note_tags` adds/removes tags atomically.

Line-level reads (`show_note_lines`, `search_note_lines`) return bounded,
anchored results for search-to-edit workflows.

## Structured Results

- `show` returns a structured envelope: base64 `source`/`body` are the
  byte-exact authority (arbitrary bytes allowed); `text` is lossy UTF-8
  decoding present only when the source is valid UTF-8, else `non_utf8`
  is `true` and `text` is absent. `fingerprint`, `kind`, `tags`, and
  body-fragment metadata accompany the content.
- Mutating tools return a structured `CommitOutcome` (`commit_created`,
  `revision_id`, `pre_revision`, per-op `path`/`selector`/`noop`/
  `fingerprint`) rather than raw `nb` stdout.

## Typed Error Translation

`nb-api 0.3` typed failures are translated identically across the
multiplexed `nb` tool and every first-class surface:

- `UnsupportedShowTarget` (from `show` on a non-text selector):
  names the offending selector and actual non-text type, states
  that `show` reads text notes only, and steers the caller toward
  `folders` or `list`. The server never silently re-routes `show` to
  another command.
- `DuplicateTitleHeading` (from `add` with both `title` and a
  content whose first nonblank line is an H1 that duplicates the
  title): names the title and the detected heading and offers either
  recovery (remove the duplicate H1, or omit the separate
  `title`).
- `FingerprintMismatch`, `AnchorMismatch`, `OccurrenceMismatch`:
  stale-editing guards that tell the caller to re-read the note and
  retry with fresh fingerprint/anchors/counts.
- `FragmentedBody`: a multi-fragment body refuses line/substring/
  body-replace operations; metadata operations still apply.
- `DirtyBaseline`: a mutation refuses when the notebook worktree/index
  is dirty; the caller should commit or clean it first.
- `IndeterminateCommit` / `RecoveryRequired`: the caller must not
  auto-retry; inspect HEAD/status before acting.
- `GateTimeout`: the notebook is busy; retry later.
- `PathCollision` / `PathIgnored` / `UnsupportedStructure` /
  `PlanValidation`: surface the specific path/plan guidance.

All other `nb-api` errors pass through with the upstream message
preserved.
