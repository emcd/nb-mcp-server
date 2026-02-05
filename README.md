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

```bash
# With default notebook from environment
NB_MCP_NOTEBOOK=myproject ./target/release/nb-mcp

# Or via CLI argument (takes precedence)
./target/release/nb-mcp --notebook myproject

# Disable commit and tag signing in the notebook repository
./target/release/nb-mcp --notebook myproject --no-commit-signing

# Print the installed version
./target/release/nb-mcp --version
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

For development with hot-reload via `reloaderoo`:

```json
{
  "mcpServers": {
    "nb": {
      "command": "reloaderoo",
      "args": ["--", "cargo", "run", "--release", "--", "--notebook", "myproject"]
    }
  }
}
```

## Commands

All commands are accessed via the `nb` tool with a `command` parameter.

### Notes

| Command | Description | Key Arguments |
|---------|-------------|---------------|
| `nb.add` | Create a note | `title`, `content`, `tags[]`, `folder` |
| `nb.show` | Read a note | `id` |
| `nb.edit` | Update a note | `id`, `content` |
| `nb.delete` | Delete a note | `id`, `confirm: true` (required) |
| `nb.list` | List notes | `folder`, `tags[]`, `limit` |
| `nb.search` | Full-text search | `query`, `tags[]` |

### Todos

| Command | Description | Key Arguments |
|---------|-------------|---------------|
| `nb.todo` | Create a todo | `description`, `tags[]` |
| `nb.do` | Mark complete | `id` |
| `nb.undo` | Reopen | `id` |
| `nb.tasks` | List todos | (none) |

### Organization

| Command | Description | Key Arguments |
|---------|-------------|---------------|
| `nb.bookmark` | Save a URL | `url`, `title`, `tags[]`, `comment` |
| `nb.import` | Import file/URL | `source`, `folder`, `filename`, `convert` |
| `nb.folders` | List folders | `parent` |
| `nb.mkdir` | Create folder | `path` |
| `nb.notebooks` | List notebooks | (none) |
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
    "query": "API",
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

### Logging

Logs are written to `~/.local/state/nb-mcp/{project}--{worktree}.log` (XDG-compliant).

For Git worktrees, logs are named after both the master project and the
worktree basename to avoid collisions between multiple MCP server instances.

### Commit Signing

Use `--no-commit-signing` to disable commit and tag signing in the notebook
repository. The server updates the notebook repository's local Git config so
signing prompts do not block MCP tool calls.

Control log level with `RUST_LOG`:

```bash
RUST_LOG=debug nb-mcp --notebook myproject
```

## Contributing

See the contribution guide and code of conduct:

- [Contribution guide](https://github.com/emcd/nb-mcp-server/blob/master/documentation/contribution.md)
- [Code of conduct](https://github.com/emcd/nb-mcp-server/blob/master/documentation/conduct.md)

## License

[Apache 2.0](https://github.com/emcd/nb-mcp-server/blob/master/LICENSE)
