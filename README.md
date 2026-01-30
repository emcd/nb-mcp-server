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

Install `nb`:

```bash
# macOS
brew install xwmx/taps/nb

# Debian/Ubuntu
sudo apt install nb

# Or see https://github.com/xwmx/nb#installation
```

### Build

```bash
cargo build --release
```

### Run

```bash
# With default notebook from environment
NB_MCP_NOTEBOOK=myproject ./target/release/nb-mcp

# Or via CLI argument (takes precedence)
./target/release/nb-mcp --notebook myproject
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

## Tagging Convention

For multi-LLM projects, use consistent tag prefixes:

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
4. `nb`'s default notebook (lowest)

## License

MIT
