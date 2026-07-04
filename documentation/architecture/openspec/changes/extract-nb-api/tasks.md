## 1. Workspace Setup
- [ ] 1.1 Create `nb-api` crate directory and `Cargo.toml`
- [ ] 1.2 Add workspace member to root `Cargo.toml`
- [ ] 1.3 Add dual path+version dependency from `nb-mcp-server` to `nb-api`
- [ ] 1.4 Verify/reserve `nb-api` crate name on crates.io

## 2. Code Extraction
- [ ] 2.1 Move `src/nb.rs` to `nb-api/src/lib.rs`
- [ ] 2.2 Move ANSI stripping helpers to `nb-api`
- [ ] 2.3 Move git notebook name derivation and shared `git_rev_parse` to `nb-api`
- [ ] 2.4 Deduplicate `paths.rs` to consume shared git detection from `nb-api`
- [ ] 2.5 Define `nb-api::Config` with nb-relevant fields only
- [ ] 2.6 Update `NbClient::new` to accept `nb-api::Config`
- [ ] 2.7 Add optional `schemars` feature flag to `nb-api` (`cfg_attr` on enums)

## 3. Integration
- [ ] 3.1 Update `nb-mcp-server` to import from `nb-api` with `schemars` feature enabled
- [ ] 3.2 Re-export `nb_api::{NbClient, NbError, EditMode, SearchMode, TaskStatus}` through `nb_mcp_server::nb` module for backward compatibility
- [ ] 3.3 Update `nb-mcp-server::Config` to convert to `nb-api::Config`
- [ ] 3.4 Move `NB_MCP_NOTEBOOK` env var resolution to `nb-mcp-server` config conversion (out of `nb-api`)
- [ ] 3.5 Keep `git_signing.rs` in server as orchestration over `nb-api` primitives

## 4. Test Migration
- [ ] 4.1 Move `tests/unit/nb_types.rs` to `nb-api` test tree
- [ ] 4.2 Split `tests/unit/config.rs` across the two `Config` types
- [ ] 4.3 Add compatibility test proving `nb_mcp_server::nb::{NbClient, NbError, EditMode, SearchMode, TaskStatus}` resolves
- [ ] 4.4 Verify all test suites pass (mcp_stdio, cli, startup_signing, unit)

## 5. Validation
- [ ] 5.1 `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] 5.2 `cargo test --test integration` passes in `nb-mcp-server`
- [ ] 5.3 `cargo test` passes in `nb-api`
- [ ] 5.4 No behavioral change in MCP tool surface
