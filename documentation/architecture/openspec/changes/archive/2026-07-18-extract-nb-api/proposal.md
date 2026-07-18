# Change: Extract nb-api crate from nb-mcp-server

## Why

The `nbspec` project (notebook-first OpenSpec orchestration) needs the nb primitives — note CRUD, search, folders, todos, notebook resolution — but not the MCP transport layer (`rmcp`, tool schemas, dispatch). Currently these primitives live inside `nb-mcp-server` as `pub mod nb`, with no standalone package boundary. Depending on `nb-mcp-server` would pull in MCP-specific dependencies that `nbspec` does not need, blurring ownership boundaries and making future reuse awkward.

## What Changes

- Extract `src/nb.rs` and its helpers into a new `nb-api` workspace member crate.
- `nb-api` contains: `NbClient`, note CRUD/search/folders/todos primitives, selector/notebook/folder validation, shared error/enum types (`NbError`, `EditMode`, `SearchMode`, `TaskStatus`).
- `nb-mcp-server` depends on `nb-api` via a dual path+version dependency; `nb-api` publishes no later than the first post-split `nb-mcp-server` release.
- MCP-specific argument aliases, tool schemas, dispatch, and `schemars`/`rmcp` integration remain in `nb-mcp-server`.
- Existing integration tests continue to exercise the MCP server against the path crate, preserving same-workspace validation.

## Impact

- Affected specs: tool-surface (no behavioral change), new nb-api capability
- Affected code: `src/nb.rs`, `src/lib.rs`, `Cargo.toml`, `tests/`
- No consumer-facing behavioral change; this is a structural refactor
