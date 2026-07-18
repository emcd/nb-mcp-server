## Context

`nb-api` v0.1.0 is published on crates.io and currently lives as a workspace member at `nb-api/` inside `emcd/nb-mcp-server`. The extraction from `src/nb.rs` into a standalone crate is complete (see `extract-nb-api` change). The crate is functional, tested, and published.

The remaining question is repository boundary: should `nb-api` stay in the `nb-mcp-server` workspace or move to its own repo?

History: we considered renaming `nb-mcp-server` to `nb-api` and nesting the server, but rejected it because multiple artifact tag streams in one repository are painful (similar friction experienced in `python-project-common`).

## Goals / Non-Goals

**Goals:**
- `emcd/nb-api` is a standalone repository with one crate, one tag stream, one publish workflow.
- `emcd/nb-mcp-server` depends on published `nb-api` from crates.io (no path dependency).
- Each repo has independent CI and release automation.
- Clean `vX.Y.Z` tags in each repo without prefixing.
- Migration is reversible (rollback to workspace member if needed).

**Non-Goals:**
- Changing `nb-api`'s public API or internal structure.
- Redesigning the `NbClient` API.
- Moving MCP-specific code into `nb-api`.
- Changing `nb-mcp-server`'s MCP tool surface or behavior.

## Decisions

### 1. Separate repo with unprefixed tags

`emcd/nb-api` uses plain `vX.Y.Z` tags (e.g., `v0.1.0`, `v0.2.0`). `emcd/nb-mcp-server` also uses plain `vX.Y.Z` tags. No prefixing needed because each repo owns one crate.

**Alternatives considered:**
- Monorepo with tag prefixes (`nb-api/v0.1.0`): rejected — tooling friction, CI coupling, consumer confusion.
- Rename `nb-mcp-server` to `nb-api` with server subdirectory: rejected — multiple tag streams still painful.

### 2. Published-only dependency

After the split, `nb-mcp-server/Cargo.toml` uses `nb-api = "0.1"` (crates.io version only). No path dependency. This means:
- `nb-mcp-server` CI tests against the published crate, matching what consumers see.
- Local development of `nb-api` + `nb-mcp-server` together uses `cargo patch` or a local registry.
- Breaking `nb-api` changes require a published release before `nb-mcp-server` can consume them.

**Trade-off:** Slower iteration cycle for coupled changes. Mitigated by the fact that `nb-api` changes are infrequent after initial stabilization, and `cargo patch` works for local development.

### 3. Migration sequence

The migration is a coordinated cut-over, not a gradual move:

1. Create `emcd/nb-api` repo with current `nb-api/` source.
2. Update `nb-api/Cargo.toml` metadata (repository, homepage) to point to `emcd/nb-api`. Bump version to `0.1.1`.
3. In `emcd/nb-mcp-server`: remove `nb-api/` workspace member, switch to crates.io dependency (`nb-api = "0.1"`).
4. Update `nb-mcp-server` docs to reference `emcd/nb-api`.
5. Add CI and publish workflows to `emcd/nb-api`.
6. Tag `v0.1.1` in `emcd/nb-api` and publish to crates.io.
7. Run `nb-mcp-server` full test suite against published `nb-api 0.1.1`.
8. Tag new `nb-mcp-server` release (v0.14.0 or similar).

**No backfill of `v0.1.0`:** The already-published `nb-api 0.1.0` on crates.io has `repository = emcd/nb-mcp-server` metadata. Backfilling a `v0.1.0` tag in the new repo would either misrepresent the published source or trigger a failed re-publish. The new repo's first tag and publish is `v0.1.1`.

**Rollback:** If the split causes problems, revert step 3 (restore workspace member + path dependency) and continue development. The published `nb-api` crates on crates.io are unaffected.

### 4. CI and release workflows

**emcd/nb-api:**
- CI: `cargo test`, `cargo clippy`, `cargo package` on push/PR.
- Release: GitHub Actions workflow triggered by `v*` tag push → `cargo publish`.
- No MCP-specific CI steps.

**emcd/nb-mcp-server:**
- CI: `cargo test`, `cargo clippy`, `cargo package` on push/PR (tests against published `nb-api`).
- Release: GitHub Actions workflow triggered by `v*` tag push → `cargo publish`.
- No `nb-api` publish sequencing needed.

### 5. Crate metadata updates

`nb-api/Cargo.toml` changes:
- `repository` → `https://github.com/emcd/nb-api`
- `homepage` → `https://github.com/emcd/nb-api`
- `documentation` → `https://docs.rs/nb-api` (unchanged — docs.rs is crate-level, not repo-level)

`nb-mcp-server/Cargo.toml` changes:
- Remove `nb-api` from `[workspace]` members
- Change `nb-api = { path = "nb-api", version = "0.1.0", features = ["schemars"] }` to `nb-api = { version = "0.1", features = ["schemars"] }`

### 6. Coordination between repos

Both repos live under the same GitHub owner (`emcd`). No cross-repo automation needed beyond standard crates.io dependency resolution. Future `nb-api` releases are independent; `nb-mcp-server` updates its `nb-api` version constraint when it wants to consume a new release.

## Risks / Trade-offs

- **Risk:** Path+version to published-only slows coupled development.
  - **Mitigation:** `cargo patch` for local dev; `nb-api` changes are infrequent post-stabilization.
- **Risk:** Two repos to maintain instead of one.
  - **Mitigation:** Each repo is simpler (one crate, one CI, one tag stream). Net reduction in complexity.
- **Risk:** Consumers must check two repos to understand the full picture.
  - **Mitigation:** `nb-mcp-server` README links to `nb-api` repo; `nb-api` README is self-contained.
- **Risk:** Migration coordination — getting the sequence wrong could leave `nb-mcp-server` in a broken state.
  - **Mitigation:** Coordinated cut-over with rollback plan (restore workspace member if needed).

## Migration Plan

1. Create `emcd/nb-api` GitHub repository.
2. Push current `nb-api/` source to new repo (fresh history).
3. Update `nb-api/Cargo.toml` metadata (repository, homepage) and bump version to `0.1.1`.
4. In `emcd/nb-mcp-server`: remove `nb-api/` directory and workspace member.
5. Switch `nb-mcp-server` dependency to published `nb-api = "0.1"`.
6. Update `nb-mcp-server` README and docs.
7. Add CI and publish workflows to `emcd/nb-api`.
8. Tag `v0.1.1` in `emcd/nb-api` and publish to crates.io.
9. Run `nb-mcp-server` full test suite against published `nb-api 0.1.1`.
10. Tag new `nb-mcp-server` release.

**Rollback:** Steps 4-5 are revertable by restoring the workspace member. Published crates on crates.io are unaffected by repo structure.

## Open Questions

- Should `nb-mcp-server` pin a minimum `nb-api` version or use a range?
  - Recommendation: pin minimum (`"0.1"`) and let Cargo resolve latest compatible. Update explicitly for new features.
