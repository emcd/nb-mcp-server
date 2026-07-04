## 1. Workspace Setup
- [x] 1.1 Create `nb-api` crate directory and `Cargo.toml`
- [x] 1.2 Add workspace member to root `Cargo.toml`
- [x] 1.3 Add dual path+version dependency from `nb-mcp-server` to `nb-api`
- [ ] 1.4 Verify/reserve `nb-api` crate name on crates.io

## 2. Code Extraction
- [x] 2.1 Move `src/nb.rs` to `nb-api/src/lib.rs`
- [x] 2.2 Move ANSI stripping helpers to `nb-api`
- [x] 2.3 Move git notebook name derivation and shared `git_rev_parse` to `nb-api`
- [x] 2.4 Deduplicate `paths.rs` to consume shared git detection from `nb-api`
- [x] 2.5 Define `nb-api::Config` with nb-relevant fields only
- [x] 2.6 Update `NbClient::new` to accept `nb-api::Config`
- [x] 2.7 Add optional `schemars` feature flag to `nb-api` (`cfg_attr` on enums)

## 3. Integration
- [x] 3.1 Update `nb-mcp-server` to import from `nb-api` with `schemars` feature enabled
- [x] 3.2 Re-export `nb_api::{NbClient, NbError, EditMode, SearchMode, TaskStatus}` through `nb_mcp_server::nb` module for backward compatibility
- [x] 3.3 Update `nb-mcp-server::Config` to convert to `nb-api::Config`
- [x] 3.4 Move `NB_MCP_NOTEBOOK` env var resolution to `nb-mcp-server` config conversion (out of `nb-api`)
- [x] 3.5 Keep `git_signing.rs` in server as orchestration over `nb-api` primitives

## 4. Test Migration
- [x] 4.1 Move `tests/unit/nb_types.rs` to `nb-api` test tree
- [x] 4.2 Split `tests/unit/config.rs` across the two `Config` types
- [x] 4.3 Add compatibility test proving `nb_mcp_server::nb::{NbClient, NbError, EditMode, SearchMode, TaskStatus}` resolves
- [x] 4.4 Verify all test suites pass (mcp_stdio, cli, startup_signing, unit)

## 5. Validation
- [x] 5.1 `cargo clippy --all-targets --all-features -- -D warnings` passes
- [x] 5.2 `cargo test --test integration` passes in `nb-mcp-server`
- [x] 5.3 `cargo test` passes in `nb-api`
- [x] 5.4 No behavioral change in MCP tool surface
