## 1. Repository Setup

- [x] 1.1 Create `emcd/nb-api` GitHub repository
- [x] 1.2 Push `nb-api/` source to new repository (fresh history)
- [x] 1.3 Update `nb-api/Cargo.toml` metadata: `repository` and `homepage` to `https://github.com/emcd/nb-api`
- [x] 1.4 Bump `nb-api` version to `0.1.1` (new repo's first publish, not backfilling `v0.1.0`)

## 2. CI and Release Automation

- [x] 2.1 Add CI workflow to `emcd/nb-api` (cargo test, clippy, package)
- [x] 2.2 Add crates.io publish workflow to `emcd/nb-api` (triggered on `v*` tag push)

## 3. nb-mcp-server Migration

- [x] 3.1 Remove `nb-api/` workspace member from `emcd/nb-mcp-server`
- [x] 3.2 Switch `nb-api` dependency from path+version to crates.io version only (`nb-api = "0.1"`)
- [x] 3.3 Update `nb-mcp-server` README to reference `emcd/nb-api` repository (N/A — root README has no nb-api references)
- [x] 3.4 Run full test suite against published `nb-api 0.1.1` crate
- [x] 3.5 Verify `cargo package` succeeds for `nb-mcp-server`

## 4. Release and Documentation

- [x] 4.1 Tag `v0.1.1` in `emcd/nb-api` and publish to crates.io
- [ ] 4.2 Tag new `nb-mcp-server` release (v0.14.0 or similar) after migration
- [ ] 4.3 Update any cross-references between repos (README links, documentation)
