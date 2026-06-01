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

All commands are accessed via the `nb` tool with a `command` parameter to
reduce the token footprint of the MCP server.
The `args` field must be a JSON object. Stringified JSON payloads are rejected.
Returned identifiers such as `coordination/mcp/1` or
`myproject:coordination/mcp/1` are `nb` selectors, not filesystem paths in the
current repository. Notebook storage is managed by `nb` configuration.

### Notes

| Command | Description | Key Arguments |
|---------|-------------|---------------|
| `nb.add` | Create a note | `title`, `content`, `tags[]`, `folder` required by default |
| `nb.show` | Read a note | `id` (alias: `selector`) |
| `nb.edit` | Update a note | `id` (alias: `selector`), `content`, `mode` (`replace` default, `append`, `prepend`) |
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

Control log level with `RUST_LOG`:

```bash
RUST_LOG=debug nb-mcp --notebook myproject
```

### Commit Signing

Use `--no-commit-signing` to disable commit and tag signing in the notebook
repository. The server updates the notebook repository's local Git config so
signing prompts do not block MCP tool calls.

## Contributing

See the contribution guide and code of conduct:

- [Contribution guide](https://github.com/emcd/nb-mcp-server/blob/master/documentation/contribution.md)
- [Code of conduct](https://github.com/emcd/nb-mcp-server/blob/master/documentation/conduct.md)

## License

[Apache 2.0](https://github.com/emcd/nb-mcp-server/blob/master/LICENSE)
