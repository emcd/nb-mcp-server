# nb-mcp

MCP server wrapping the [nb](https://github.com/xwmx/nb) CLI for LLM-friendly note-taking.

## Motivation

Using `nb` directly via shell has two problems for LLM assistants:

1. **Backtick escaping**: Markdown content with backticks triggers shell command substitution, corrupting notes.

2. **Notebook context**: `nb` assumes a default notebook, making per-project use awkward.

This MCP server solves both by:

- Accepting content as JSON parameters (no shell escaping needed)
- Qualifying all commands with an explicit notebook

## Quick Start

### Prerequisites

Install `nb` by following the official instructions:
[nb installation guide](https://github.com/xwmx/nb#installation).

### Installation

From [crates.io](https://crates.io/crates/nb-mcp-server):

```bash
cargo install nb-mcp-server
```

See the [changelog](CHANGELOG.md) for release history and upgrade notes.

Or download a prebuilt binary from [GitHub Releases](https://github.com/emcd/nb-mcp-server/releases).

### Build from Source

```bash
cargo build --release
```

### Run

With default notebook from environment:

```bash
NB_MCP_NOTEBOOK=myproject ./target/release/nb-mcp
```

Or via CLI argument (takes precedence):

```bash
./target/release/nb-mcp --notebook myproject
```

Disable commit and tag signing in the notebook repository:

```bash
./target/release/nb-mcp --notebook myproject --no-commit-signing
```

Allow new notes at the notebook root instead of requiring a folder:

```bash
./target/release/nb-mcp --notebook myproject --allow-top-level-notes
```

Print the installed version:

```bash
./target/release/nb-mcp --version
```

Show the resolved notebook path and state directory:

```bash
./target/release/nb-mcp --show-paths
```

### MCP Configuration

Add to your MCP client configuration (e.g., `.mcp.json`):

```json
{
  "mcpServers": {
    "nb": {
      "command": "/path/to/nb-mcp",
      "args": ["--notebook", "myproject"]
    }
  }
}
```

## Commands

The canonical access path is the multiplexed `nb` tool with a `command`
parameter, which reduces the token footprint of the MCP server.
The `args` field must be a JSON object. Stringified JSON payloads are rejected.
Unknown `args` fields are rejected instead of ignored; use the exact command
schema fields or documented aliases.
Returned identifiers such as `coordination/mcp/1` or
`myproject:coordination/mcp/1` are `nb` selectors, not filesystem paths in the
current repository. Notebook storage is managed by `nb` configuration.
The `notebook` argument must be a bare notebook name. Use `folder` for folder
paths and `id` / `selector` for note selectors. Existing-item commands accept
copied selectors such as `myproject:coordination/mcp/1`, but reject conflicts
with a separate `notebook` argument.

### First-Class Tools

All commands are also available as direct first-class tools with typed
schemas: `add`, `show`, `delete`, `move`, `list`, `search`, `todo`,
`do`, `undo`, `tasks`, `bookmark`, `folders`, `mkdir`, `import`,
`status`, `notebooks`, plus the body-aware tools `replace_note_body`,
`edit_note_substring`, `edit_note_lines`, `retitle_note`,
`edit_note_tags`, and the line tools `show_note_lines`,
`search_note_lines`. These bypass the multiplexed command dispatch. The
multiplexed `nb` tool remains as the compact/backcompat compatibility
surface for the retained commands.

### Notes

| Command | Description | Key Arguments |
|---------|-------------|---------------|
| `nb.add` | Create a note | `title`, `content`, `tags[]`, `folder` required by default |
| `nb.show` | Read a note (structured envelope) | `id` (alias: `selector`) |
| `nb.delete` | Delete a note | `id` (alias: `selector`) |
| `nb.move` | Move or rename a note | `id` (alias: `selector`), `destination` |
| `nb.list` | List notes | `folder`, `tags[]`, `limit` (`[ ]` / `[x]` indicate todo status; leading glyphs are item markers) |
| `nb.search` | Full-text search | `queries[]` (required), `mode` (`any` default, `all`), `tags[]` |

### Todos

| Command | Description | Key Arguments |
|---------|-------------|---------------|
| `nb.todo` | Create a todo | `folder` required by default, `title`, optional `description` (alias: `content`), optional `tasks[]`, `tags[]` |
| `nb.do` | Mark complete | `id` (alias: `selector`), optional `task_number` |
| `nb.undo` | Reopen | `id` (alias: `selector`), optional `task_number` |
| `nb.tasks` | List todos | optional `status` (`open` or `closed`), optional `recursive` (`true` default) |

### Organization

| Command | Description | Key Arguments |
|---------|-------------|---------------|
| `nb.bookmark` | Save a URL | `url`, `folder` required by default, `title`, `tags[]`, `comment` |
| `nb.import` | Import file/URL | `source`, `folder` required by default, `filename`, `convert` |
| `nb.folders` | List folders | `parent` |
| `nb.mkdir` | Create folder | `path` |
| `nb.notebooks` | List notebooks only | (none) |
| `nb.status` | Notebook info | (none) |

## Examples

Create a note with code:

```json
{
  "command": "nb.add",
  "args": {
    "title": "API Design Notes",
    "content": "# API Design\n\nUse `GET /items` for listing.\n\n```python\nresponse = client.get('/items')\n```",
    "tags": ["design", "api"],
    "folder": "docs"
  }
}
```

Search for notes:

```json
{
  "command": "nb.search",
  "args": {
    "queries": ["API", "design"],
    "mode": "any",
    "tags": ["design"]
  }
}
```

## Tagging Suggestions

For multi-LLM projects, consider using consistent tag prefixes (optional).
Example categories and prefixes:

| Category | Pattern | Examples |
|----------|---------|----------|
| Collaborator | `llm-<name>` | `llm-claude`, `llm-gpt` |
| Component | `component-<name>` | `component-api`, `component-ui` |
| Task type | `task-<type>` | `task-bug`, `task-feature` |
| Status | `status-<state>` | `status-review`, `status-blocked` |

## Body-Aware Editing

The legacy `nb.edit` tool (and its `overwrite`/`append`/`prepend` modes)
was removed in the `nb-api 0.3` cutover. The body-aware replacement tools
are **direct-only** (no multiplexed `nb.*` aliases) and are designed to
prevent the destructive whole-note-overwrite failure mode that the old
surface enabled:

- `replace_note_body` — replace the entire note body with plain UTF-8
  `new_body`. Requires the body `fingerprint` from a preceding `show`; a
  stale fingerprint is rejected with re-read guidance so you cannot
  overwrite a note you have not just read.
- `edit_note_substring` — replace one or more occurrences of a plain-text
  `pattern` with `replacement`. `expected_count` must match the actual
  match count; an optional fingerprint guards against stale edits.
- `edit_note_lines` — apply a batch of disjoint insert/delete/replace
  `edits` verified against line anchors from one original snapshot;
  `content` is plain UTF-8 text.
- `retitle_note` — change the title without changing the path.
- `edit_note_tags` — add and/or remove tags in one atomic operation.

These tools address notes by a flat `id` (alias `selector`) string exactly
like `show`/`delete`/`move`; the `NoteTarget` `path` variant and
notebook-relative filenames are not exposed, and qualified selectors
returned by `show`/`show_note_lines`/`search_note_lines` round-trip
directly as `id`. Body content (`pattern`, `replacement`, `title`,
`new_body`, line-edit `content`) and line/search `text`/`title` are plain
UTF-8 strings; there is no base64 on this change's textual read/edit MCP
tool surface (`new_body` has no `new_body_base64` opt-in; a future
raw-bytes MCP tool is explicitly outside this guarantee). Line-level reads
are available through `show_note_lines` (bounded, anchored windows) and
`search_note_lines` (anchored matches), which support search-to-edit
without reading the whole note.

Invoking multiplexed `nb.edit` is rejected with recovery guidance naming
the replacement tools. The body-aware and line tools are direct-only:
invoking them through the multiplexed `nb` tool is rejected.

## Structured Results

- `show` returns a text-first slim structured envelope: `selector`, `path`,
   `kind`, `todo_state`, `title` (`Option<String>`), `tags`, `body`
   (full decoded UTF-8 text), `body_contiguous`, and `fingerprint`. No
   base64 field is exposed on this change's textual read/edit MCP tool
   surface. When the source is not valid UTF-8 or the target is
   non-textual, `show` returns a typed error carrying the detected content
   type plus guidance to an external raw-retrieval facility outside MCP
   (the `nb` CLI); it never returns base64 bytes.
- Mutating tools (`add`, `todo`, `bookmark`, `mkdir`, `delete`, `move`,
  `do`, `undo`, and the body-aware tools) return a structured
  `CommitOutcome`: `commit_created`, `revision_id`, `pre_revision`, and
  per-operation `path`/`selector`/`noop`/`fingerprint`. Idempotent no-op
  mutations report `commit_created: false`.

## Typed Error Surfaces

`nb-api 0.3` introduces typed failures that the MCP layer translates into
actionable diagnostics on both the multiplexed `nb.*` surface and the
first-class tool surface:

- `show` on a non-text selector (folder, archive, image, ...): the
  error names the selector and the actual non-text type, states
  that `show` reads text notes only, and points the caller at
  `folders`/`list`. The server never silently re-routes `show` to
  another command.
- `add` with both a `title` and a `content` whose first nonblank
  line is an H1 that duplicates the title: the error names the
  title and the detected heading and tells the caller to remove
  the duplicate H1 or omit the separate `title`.
- Stale-editing guards: `FingerprintMismatch`, `AnchorMismatch`, and
  `OccurrenceMismatch` tell the caller to re-read the note and retry
  with fresh fingerprint/anchors/counts.
- `FragmentedBody`: a multi-fragment body (for example a bookmark with
  multiple body fragments) refuses line/substring/body-replace
  operations; metadata operations (`retitle_note`, `edit_note_tags`)
  still apply.
- `DirtyBaseline`: a mutation refuses when the notebook worktree/index
  is dirty; the diagnostic tells the caller to commit or clean it first.
- `IndeterminateCommit` / `RecoveryRequired`: commit completion is unknown
  or recovery is needed; the diagnostic instructs the caller not to
  auto-retry and to inspect HEAD/status before acting.
- `GateTimeout`: the notebook is busy; retry later.
- `PathCollision` / `PathIgnored` / `UnsupportedStructure` /
  `PlanValidation`: surface the specific path/plan guidance.

## Configuration

### Notebook Resolution

Priority order:

1. Per-command `notebook` argument (highest)
2. CLI `--notebook` flag
3. `NB_MCP_NOTEBOOK` environment variable
4. Git-derived default from the master worktree path

If no notebook can be resolved, commands fail with a configuration error. The
server does not fall back to `nb`'s default notebook.

If the resolved notebook does not exist, the server creates it automatically.
Use `--no-create-notebook` to disable automatic creation.

### Logging

Logs are written to `~/.local/state/nb-mcp/{project}--{worktree}.log` (XDG-compliant).

For Git worktrees, logs are named after both the master project and the
worktree basename to avoid collisions between multiple MCP server instances.

Use `--show-paths` to print the resolved notebook path and state directory.

### Folder Requirement

By default, note-creating commands require a `folder` argument so agents do not
accidentally litter project notebook roots. This applies to `nb.add`, `nb.todo`,
`nb.bookmark`, and `nb.import`. Use `nb.mkdir` to create new folders and
`nb.folders` to list existing folders.

Set `NB_MCP_ALLOW_TOP_LEVEL_NOTES=true` or pass `--allow-top-level-notes` to
permit root-level note creation.

### Notebook Overrides

Mutating commands warn after successful writes when the `notebook` argument
targets a notebook other than the project default. Cross-notebook writes remain
allowed for collaboration across teams, but the warning helps catch accidental
notebook/folder confusion.

The `notebook` argument accepts only bare notebook names, not selector syntax.
For example, use `notebook: "other-team"` with `folder: "todos/mcp"`, not
`notebook: "other-team:todos/mcp"`.

Control log level with `RUST_LOG`:

```bash
RUST_LOG=debug nb-mcp --notebook myproject
```

### Commit Signing

Use `--no-commit-signing` to disable commit and tag signing in the notebook
repository. The server updates the notebook repository's local Git config so
signing prompts do not block MCP tool calls.

## Related Projects

- [nb-api](https://github.com/emcd/nb-api) — Typed Rust interface to the `nb` CLI. Published on [crates.io](https://crates.io/crates/nb-api). This MCP server depends on `nb-api` for all note-taking primitives; the body-aware editing surface, typed errors, structured results, and sanitized empty listings all come from `nb-api 0.3`.

## Contributing

See the contribution guide and code of conduct:

- [Contribution guide](https://github.com/emcd/nb-mcp-server/blob/master/documentation/contribution.md)
- [Code of conduct](https://github.com/emcd/nb-mcp-server/blob/master/documentation/conduct.md)

## License

[Apache 2.0](https://github.com/emcd/nb-mcp-server/blob/master/LICENSE)
