# nb MCP Server Tool Surface Proposal

## Problem Statement

Using `nb` via shell has two key issues for LLM assistants:

1. **Backtick escaping**: Markdown content with backticks triggers shell command substitution
2. **Notebook context**: `nb` assumes a default notebook, making per-project use awkward

## Design Goals

- Pass content as structured parameters (avoiding shell escaping entirely)
- Automatically qualify all operations with the appropriate notebook
- Expose operations most useful for LLM-assisted workflows
- Keep the tool surface minimal but complete

## Notebook Resolution Strategy

The server should resolve notebook context in this order:

1. **Explicit parameter**: `notebook: "project-foo"` on any tool call
2. **Server configuration**: Set via environment variable or config file at startup
3. **Working directory inference**: Map CWD to a notebook via configurable rules

For project-specific use, the recommended pattern is configuring the MCP server
per-project in `claude_desktop_config.json` or `.mcp.json` with an explicit
notebook name or CWD-to-notebook mapping.

## Proposed Tool Surface

### Meta-Tool: `nb`

Single entry point with subcommand dispatch (following the litrpg pattern).

```
nb(command: string, args: object)
```

### Subcommands

#### Note Operations

| Command | Description | Key Parameters |
|---------|-------------|----------------|
| `nb.add` | Create a new note | `title`, `content`, `tags[]`, `folder` |
| `nb.edit` | Replace note content | `id`, `content` |
| `nb.show` | Read a note's content | `id` |
| `nb.delete` | Delete a note | `id`, `confirm: bool` |
| `nb.list` | List notes | `folder`, `tags[]`, `limit`, `sort` |
| `nb.search` | Full-text search | `query`, `tags[]`, `type` |

#### Todo Operations

| Command | Description | Key Parameters |
|---------|-------------|----------------|
| `nb.todo` | Create a todo item | `description`, `due`, `tags[]` |
| `nb.do` | Mark todo complete | `id` |
| `nb.undo` | Reopen a todo | `id` |
| `nb.tasks` | List todos | `status`, `due_filter`, `tags[]` |

#### Bookmark/Import Operations

| Command | Description | Key Parameters |
|---------|-------------|----------------|
| `nb.bookmark` | Save a URL | `url`, `title`, `tags[]`, `comment` |
| `nb.import` | Import a file | `path`, `title`, `tags[]` |

#### Notebook Management

| Command | Description | Key Parameters |
|---------|-------------|----------------|
| `nb.notebooks` | List available notebooks | — |
| `nb.status` | Show current notebook and stats | — |

#### Introspection

| Command | Description | Key Parameters |
|---------|-------------|----------------|
| `nb.help` | Get subcommand schemas | `query` |

## Parameter Design Notes

### Content Handling

All content parameters accept raw strings. The MCP server handles escaping
when invoking `nb`. This solves the backtick problem entirely.

```json
{
  "command": "nb.add",
  "args": {
    "title": "Code Snippet",
    "content": "Use `grep -r` to search recursively.",
    "tags": ["cli", "tips"]
  }
}
```

### Identifiers

Notes can be referenced by:
- Numeric ID: `42`
- Filename: `example.md`
- Title (partial match): `"Code Snippet"`
- Relative path: `folder/example.md`

The server passes these through to `nb` which handles resolution.

### Tags

Tags use `#hashtag` format in `nb`, but the MCP interface accepts bare strings:

```json
{ "tags": ["project", "important"] }
```

The server prefixes with `#` when constructing the `nb` command.

## Implementation Approach

### Shell Invocation

The server invokes `nb` as a subprocess, constructing commands like:

```bash
nb notebook:add --title "Code Snippet" --content "..." --tags "#cli" "#tips"
```

Key points:
- Always prefix with `notebook:` to avoid default-notebook issues
- Use `--content` flag (reads from argument, not stdin) to avoid escaping
- Capture stdout/stderr and parse for structured responses

### Response Parsing

`nb` outputs vary by command. The server should:
- Parse "Added: [id] filename" for creation confirmations
- Return note content as-is for `show`
- Parse list output into structured arrays

### Error Handling

- Notebook not found → clear error with available notebooks
- Note not found → clear error with search suggestions
- Command failed → include nb's stderr in error details

## Future Considerations

### Not in Initial Scope

- **Encryption**: `nb` supports encrypted notes; defer to later
- **Sync**: `nb` has git-based sync; complex, defer
- **Plugins**: `nb` is extensible; out of scope
- **Browser interface**: `nb browse` starts a web UI; not relevant for MCP

### Potential Enhancements

- **Batch operations**: Add multiple notes in one call
- **Templates**: Support `nb`'s template system
- **Pinning**: Support pinned notes for quick access
- **Linking**: `[[wiki-style]]` links between notes

## Example Workflows

### Session Note-Taking

```json
// Start of session
{"command": "nb.add", "args": {
  "title": "Session 2024-01-15",
  "content": "## Goals\n- Fix auth bug\n- Review PR #42",
  "tags": ["session", "project-x"]
}}

// During session
{"command": "nb.edit", "args": {
  "id": "session-2024-01-15.md",
  "content": "## Goals\n- [x] Fix auth bug\n- [ ] Review PR #42\n\n## Notes\n..."
}}
```

### Research Bookmarking

```json
{"command": "nb.bookmark", "args": {
  "url": "https://example.com/article",
  "tags": ["research", "rust", "async"],
  "comment": "Good explanation of Pin and Unpin"
}}
```

### Todo Management

```json
// Create
{"command": "nb.todo", "args": {
  "description": "Update README with new API",
  "tags": ["docs"]
}}

// Complete
{"command": "nb.do", "args": {"id": "42"}}

// List pending
{"command": "nb.tasks", "args": {"status": "open"}}
```

## Design Decisions

### 1. Folder Structure — YES, expose it

Based on real-world usage patterns (e.g., rust-litrpg notes), folders carry
semantic meaning that tags alone don't capture well:

```
data-models/
  harms-plights/     # Sub-domain grouping
  legacy-ports/      # Migration artifacts
game/
  proposals/         # Design proposals by author
  handoffs/          # Session continuity notes
supervision/         # Cross-session coordination
```

This hierarchy says "what kind of document" while tags say "what it's about."

**Proposed folder operations:**

| Command | Description | Parameters |
|---------|-------------|------------|
| `nb.folders` | List folders in notebook | `parent` (optional) |
| `nb.mkdir` | Create a folder | `path` |

Notes can specify `folder` on creation:
```json
{"command": "nb.add", "args": {
  "folder": "game/proposals",
  "title": "new-feature--claude.md",
  "content": "..."
}}
```

### 2. List Output — Tiered verbosity

Different use cases need different detail levels:

| Mode | Returns | Use Case |
|------|---------|----------|
| `summary` (default) | id, title, folder, tags, modified | Browsing, discovery |
| `ids` | id only | Batch operations, piping |
| `full` | Above + content preview (first N lines) | Quick triage |

```json
{"command": "nb.list", "args": {
  "folder": "game/proposals",
  "verbosity": "summary",
  "limit": 20
}}
```

For full content, use `nb.show`. This keeps `nb.list` context-friendly.

### 3. Destructive Operations — Require confirmation

The MCP layer should require `confirm: true` for:
- `nb.delete`
- Any future bulk operations

This prevents accidental deletion when an LLM misunderstands intent.

### 4. Images/Attachments — Post-MVP

For MVP, defer image support. When added later:

- `nb.import` should accept image paths
- `nb.show` should return file path for binary content (LLM can use Read tool)
- Consider `nb.attach` for adding images to existing notes

## Refined Tool Summary

### MVP Scope

**Core Notes**
- `nb.add` — create note (with folder support)
- `nb.edit` — update content
- `nb.show` — read content
- `nb.delete` — remove (with confirm)
- `nb.list` — browse (with verbosity levels)
- `nb.search` — find by content/tags

**Todos**
- `nb.todo` — create
- `nb.do` / `nb.undo` — toggle status
- `nb.tasks` — list todos

**Bookmarks**
- `nb.bookmark` — save URL

**Organization**
- `nb.folders` — list folders
- `nb.mkdir` — create folder
- `nb.notebooks` — list notebooks
- `nb.status` — current notebook info

**Meta**
- `nb.help` — schemas and usage

### Post-MVP

- `nb.import` — file/image import
- `nb.move` — relocate notes between folders
- `nb.rename` — change note filename
- `nb.pin` / `nb.unpin` — quick access
- Encryption support
- Template support

## LLM Usage Patterns — What I'd Actually Use

Speaking as Claude, here's my honest assessment of frequency:

### High Frequency (every session)

1. **`nb.add`** — Session notes, capturing decisions, documenting what we did
2. **`nb.search`** — "What did we decide about X?" / "Where's that design doc?"
3. **`nb.show`** — Reading notes found via search
4. **`nb.list`** — Orienting in a project's notes structure

### Medium Frequency (most sessions)

5. **`nb.edit`** — Updating session notes, marking progress
6. **`nb.todo` / `nb.tasks`** — Task management across sessions
7. **`nb.do`** — Marking tasks complete

### Lower Frequency (as needed)

8. **`nb.bookmark`** — Saving references from web searches
9. **`nb.folders` / `nb.mkdir`** — Initial project setup, occasional reorganization
10. **`nb.delete`** — Cleaning up obsolete notes

### The Killer Feature

The real value isn't any single tool — it's **context persistence across sessions**.
Right now, when a conversation ends, context is lost. With nb-mcp:

- Session N ends: I write notes summarizing state, decisions, blockers
- Session N+1 starts: I search/read those notes to rebuild context
- Human doesn't have to re-explain everything

This is why `nb.add` and `nb.search` are the core primitives. Everything else
is nice-to-have layered on top.

### What I Don't Need

- Complex query languages — simple keyword + tag search is enough
- Real-time sync — I'm not collaborating with other LLMs mid-session
- Rich formatting in lists — I'll read the full note if I need details
- Undo history — just show me current state

---

*Draft: 2025-01-26*
