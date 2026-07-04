# nb-api

Typed Rust interface to the [nb](https://github.com/xwmx/nb) note-taking CLI.

## Purpose

`nb-api` provides a programmatic Rust client for `nb`, the command-line
note-taking tool. It wraps `nb` as a subprocess, handling argument escaping,
notebook qualification, output parsing, and error recovery.

This crate is designed for use by:

- **`nb-mcp-server`** — the MCP server that exposes `nb` to LLM assistants
- **`nbspec`** — notebook-first OpenSpec orchestration
- Any Rust application that needs to drive `nb` programmatically

`nb-api` is intentionally free of MCP-specific dependencies (`rmcp`,
`schemars`). The `schemars` feature is available as an optional add-on for
consumers that need JSON Schema generation (e.g., MCP tool parameters).

## Quick Start

### Prerequisites

Install `nb` by following the official instructions:
[nb installation guide](https://github.com/xwmx/nb#installation).

### Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
nb-api = "0.1"
```

With optional JSON Schema support:

```toml
[dependencies]
nb-api = { version = "0.1", features = ["schemars"] }
```

### Example

```rust
use nb_api::{Config, NbClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        notebook: Some("myproject".to_string()),
        ..Default::default()
    };
    let client = NbClient::new(&config)?;

    // Create a note
    let result = client.add(
        Some("My Note"),
        "Note content with `backticks` works fine.",
        &["design".to_string(), "api".to_string()],
        Some("docs"),
        None,
    ).await?;
    println!("{}", result);

    // Search notes
    let results = client.search(
        &["API".to_string()],
        nb_api::SearchMode::Any,
        &[],
        None,
        None,
    ).await?;
    println!("{}", results);

    Ok(())
}
```

## API Surface

All operations return `Result<String, NbError>` with raw ANSI-stripped CLI
output. Typed accessor methods may be added in future versions.

### Notes

| Method | Description |
|--------|-------------|
| `add` | Create a note (title, content, tags, folder) |
| `show` | Read a note's content |
| `edit` | Update a note (replace, append, or prepend) |
| `delete` | Delete a note |
| `move_note` | Move or rename a note |
| `list` | List notes with optional filtering |
| `search` | Full-text search with OR/AND semantics |

### Todos

| Method | Description |
|--------|-------------|
| `todo` | Create a todo item with optional checklist |
| `do_task` | Mark a todo as complete |
| `undo_task` | Reopen a completed todo |
| `tasks` | List todos with optional status filter |

### Organization

| Method | Description |
|--------|-------------|
| `bookmark` | Save a URL as a bookmark |
| `import` | Import a file or URL |
| `folders` | List folders in a notebook |
| `mkdir` | Create a folder |
| `notebooks` | List available notebooks |
| `status` | Show notebook status |
| `notebook_path` | Get the filesystem path for a notebook |

### Types

| Type | Description |
|------|-------------|
| `NbClient` | Async client for invoking nb commands |
| `NbError` | Error type for all operations |
| `Config` | Configuration for constructing `NbClient` |
| `EditMode` | Content update mode (replace, append, prepend) |
| `SearchMode` | Query matching mode (any, all) |
| `TaskStatus` | Todo status filter (open, closed) |

## Configuration

`Config` contains only nb-relevant fields:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `notebook` | `Option<String>` | `None` | Default notebook name (overrides Git-derived fallback) |
| `create_notebook` | `bool` | `true` | Automatically create missing notebooks |
| `allow_top_level_notes` | `bool` | `false` | Allow notes at notebook root without a folder |
| `disable_git_signing` | `bool` | `false` | Disable Git commit/tag signing for nb subprocesses |

### Notebook Resolution

Priority order:

1. Per-command `notebook` argument (highest)
2. `Config.notebook` field
3. Git-derived default from the master worktree path

The `NB_MCP_NOTEBOOK` environment variable is **not** read by `nb-api`.
That is an MCP-server-specific convention resolved by `nb-mcp-server` before
constructing `Config`.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `schemars` | disabled | Adds `JsonSchema` derive to `EditMode`, `SearchMode`, `TaskStatus` |

## Contributing

See the contribution guide and code of conduct:

- [Contribution guide](https://github.com/emcd/nb-mcp-server/blob/master/documentation/contribution.md)
- [Code of conduct](https://github.com/emcd/nb-mcp-server/blob/master/documentation/conduct.md)

## License

[Apache 2.0](https://github.com/emcd/nb-mcp-server/blob/master/LICENSE)
