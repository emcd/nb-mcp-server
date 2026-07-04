## Context

`nb-mcp-server` is currently a single crate that is both a library (`lib.rs` exposes `pub mod nb`, `pub mod mcp`, etc.) and a binary (`nb-mcp`). The `NbClient` in `src/nb.rs` is the core primitive — it wraps the `nb` CLI with escaping, validation, and output parsing. The `src/mcp.rs` module is the MCP server layer that consumes `NbClient`.

The `nbspec` project needs `NbClient` but not the MCP layer. Extracting `nb-api` as a standalone crate enables clean dependency boundaries.

## Goals / Non-Goals

- Goals:
  - `nb-api` is a standalone crate with no MCP-specific dependencies.
  - `nb-mcp-server` depends on `nb-api` via path dependency.
  - Same-workspace validation: fixes to `nb-api` are immediately testable in the MCP server.
  - Full existing test coverage preserved through the refactor.
  - Minimal behavior changes during extraction; API ergonomics iterate later.

- Non-Goals:
  - Redesigning the `NbClient` API during extraction.
  - Moving MCP tool schemas or dispatch into `nb-api`.
  - Publishing `nb-api` to crates.io immediately (publish once API stabilizes).

## Decisions

- **Workspace layout:** Cargo workspace with `nb-api` and `nb-mcp-server` as members. `nb-mcp-server` depends on `nb-api` using dual path+version form: `nb-api = { path = "../nb-api", version = "0.1" }`. This enables same-workspace development while satisfying crates.io's requirement for published dependencies. `nb-api` publishes in lockstep with the first post-split server release. Crate name `nb-api` chosen over alternatives (`nb-driver`, `nb-client`, `nb-core`, `nb-sdk`); description leads with "Typed Rust interface to the nb note-taking CLI" to avoid HTTP-client connotation common for X-api crates.
- **What moves to `nb-api`:**
  - `src/nb.rs` (all public types and methods)
  - Selector/notebook/folder validation helpers
  - `NbError`, `EditMode`, `SearchMode`, `TaskStatus` enums (with unconditional `serde::Deserialize`; optional `schemars::JsonSchema` behind feature flag)
  - ANSI stripping utility
  - Git notebook name derivation and shared `git_rev_parse` helper (deduplicated from `paths.rs`)
- **What stays in `nb-mcp-server`:**
  - `src/mcp.rs` (MCP server, tool schemas, dispatch)
  - `src/paths.rs` (log path, config path — consumes shared git detection from `nb-api`)
  - `src/git_signing.rs` (takes server `Config`, constructs `NbClient`; orchestration layer stays in server)
  - `src/main.rs` (binary entrypoint)
  - `Config` struct (mixes MCP and nb concerns; `nb-api` gets its own config type)
  - `schemars` derive on arg structs (MCP-specific)
- **Config boundary:** `NbClient::new` takes only nb-relevant config (notebook name, create_notebook flag, allow_top_level_notes flag, disable_git_signing flag). MCP-specific fields (show_paths) stay in `nb-mcp-server::Config`.
- **Schema boundary:** `nb-api` defines `EditMode`, `SearchMode`, `TaskStatus` with unconditional `serde::Deserialize`. An optional `schemars` feature flag enables `#[derive(JsonSchema)]` via `#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]`. `nb-mcp-server` enables the `schemars` feature; `nbspec` and other consumers get a schemars-free build by default. This avoids maintaining parallel mirror enums and conversion glue.
- **Backward compatibility:** `nb-mcp-server` preserves the existing `pub mod nb` module path. After extraction, `nb_mcp_server::nb` re-exports `nb_api::{NbClient, NbError, EditMode, SearchMode, TaskStatus}` so that downstream consumers using the `0.13.0` import paths continue to compile. This is not a breaking change.
- **Env var resolution:** `NB_MCP_NOTEBOOK` is an MCP-server-specific env var. After extraction, `nb-mcp-server` resolves `NB_MCP_NOTEBOOK` and passes the value into `nb-api::Config::notebook`. `nb-api` provides the Git-derived notebook name fallback (current `derive_git_notebook_name` behavior) but does not read `NB_MCP_NOTEBOOK` itself.
- **Test migration:** Tests live in `tests/unit/` per project standards (no inline `#[cfg(test)]` in `src/`). `tests/unit/nb_types.rs` moves to `nb-api`'s test tree. `tests/unit/paths.rs` stays with the server. `tests/unit/config.rs` splits across the two `Config` types. Integration tests (`tests/integration/`) stay in `nb-mcp-server`.

## Risks / Trade-offs

- **Risk:** Config type duplication between crates.
  - **Mitigation:** `nb-api` defines its own minimal config; `nb-mcp-server::Config` wraps or converts.
- **Risk:** `git_signing.rs` takes server `Config` and constructs its own `NbClient`.
  - **Mitigation:** Keep `git_signing.rs` in server as orchestration layer. It calls `nb-api` primitives but does not move into `nb-api`.
- **Risk:** Publishing constraint — crates.io rejects path-only dependencies.
  - **Mitigation:** Dual `path + version` dependency. `nb-api` publishes in lockstep with first post-split server release. Reserve crate name early.

## Migration Plan

1. Verify/reserve `nb-api` crate name on crates.io.
2. Create `nb-api` crate in workspace.
3. Move `src/nb.rs` and helpers to `nb-api/src/`.
4. Deduplicate `git_rev_parse` shared between `nb.rs` and `paths.rs`; `nb-api` exports the shared helper.
5. Define `nb-api::Config` with nb-relevant fields only.
6. Update `nb-mcp-server` to depend on `nb-api` via dual path+version dependency.
7. Enable `schemars` feature in `nb-mcp-server`'s dependency on `nb-api`.
8. Re-export `nb-api` types through `nb_mcp_server::nb` for backward compatibility.
9. Move `NB_MCP_NOTEBOOK` env var resolution to server-side config conversion.
10. Migrate tests: `tests/unit/nb_types.rs` to `nb-api`, split `tests/unit/config.rs`.
11. Run full test suite; fix any compilation issues.

## Open Questions

- Should `nb-api` re-export `anyhow` for `NbClient::new` return type, or use its own error type?
  - Recommendation: use `anyhow::Result` for constructor, `NbError` for operations — matches current pattern.
- Should `nb-api` have its own `README.md` and documentation?
  - Recommendation: minimal README with usage example; full docs come when publishing.
