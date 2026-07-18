## MODIFIED Requirements

### Requirement: Path dependency development workflow
After the split, `nb-mcp-server` SHALL depend on `nb-api` via a crates.io published version only, without a path dependency.

#### Scenario: Dependency declaration
- **WHEN** a consumer reads `nb-mcp-server/Cargo.toml`
- **THEN** the `nb-api` dependency SHALL specify a compatible published version without a `path` field

#### Scenario: Build succeeds with published crate
- **WHEN** `cargo build` is run in `nb-mcp-server`
- **THEN** it SHALL resolve `nb-api` from crates.io and compile successfully

#### Scenario: Tests pass with published crate
- **WHEN** `cargo test` is run in `nb-mcp-server`
- **THEN** all tests SHALL pass using the published `nb-api` crate
