## ADDED Requirements

### Requirement: nb-api repository SHALL exist as a standalone GitHub repository
The `nb-api` crate SHALL be hosted in a standalone `emcd/nb-api` GitHub repository, separate from `emcd/nb-mcp-server`.

#### Scenario: Repository exists and is accessible
- **WHEN** a consumer navigates to `https://github.com/emcd/nb-api`
- **THEN** the repository SHALL be accessible and contain the `nb-api` crate source

#### Scenario: Repository contains crate source
- **WHEN** a consumer clones `emcd/nb-api`
- **THEN** the repository SHALL contain `Cargo.toml`, `src/`, `tests/`, and `README.md` for the `nb-api` crate

### Requirement: nb-api repository SHALL have unprefixed release tags
The `emcd/nb-api` repository SHALL use plain `vX.Y.Z` tags for releases (e.g., `v0.1.1`, `v0.2.0`), without any crate-name prefix. The already-published `nb-api 0.1.0` (with `emcd/nb-mcp-server` metadata) SHALL NOT be backfilled as a tag in the new repo; the new repo's first tag corresponds to its first published version.

#### Scenario: Tag format
- **WHEN** a release is tagged in the `emcd/nb-api` repository
- **THEN** the tag SHALL match the pattern `v{major}.{minor}.{patch}`

#### Scenario: Tag matches published crate version
- **WHEN** a `vX.Y.Z` tag is pushed
- **THEN** the tagged commit's `Cargo.toml` version SHALL match the tag version

#### Scenario: No backfill of pre-split versions
- **WHEN** `emcd/nb-api` is created
- **THEN** the repository SHALL NOT contain tags for versions published before the split (e.g., `v0.1.0`)

### Requirement: nb-api repository SHALL have independent CI
The `emcd/nb-api` repository SHALL have its own CI pipeline that runs tests, clippy, and package validation on push and pull request.

#### Scenario: CI runs on push
- **WHEN** a commit is pushed to `emcd/nb-api`
- **THEN** CI SHALL run `cargo test`, `cargo clippy`, and `cargo package`

#### Scenario: CI is independent of nb-mcp-server
- **WHEN** CI runs in `emcd/nb-api`
- **THEN** it SHALL NOT depend on or reference `emcd/nb-mcp-server`

### Requirement: nb-api repository SHALL have crates.io publish automation
The `emcd/nb-api` repository SHALL have a GitHub Actions workflow that publishes to crates.io when a `v*` tag is pushed.

#### Scenario: Publish on tag push
- **WHEN** a `v*` tag is pushed to `emcd/nb-api`
- **THEN** the workflow SHALL run `cargo publish` to publish the crate to crates.io

### Requirement: nb-api crate metadata SHALL reference the new repository
The `nb-api` crate's `Cargo.toml` metadata (repository, homepage) SHALL point to `emcd/nb-api`, not `emcd/nb-mcp-server`.

#### Scenario: Repository URL in Cargo.toml
- **WHEN** a consumer reads `nb-api/Cargo.toml`
- **THEN** the `repository` field SHALL be `https://github.com/emcd/nb-api`

#### Scenario: Homepage URL in Cargo.toml
- **WHEN** a consumer reads `nb-api/Cargo.toml`
- **THEN** the `homepage` field SHALL be `https://github.com/emcd/nb-api`
