# Change: Split nb-api into separate repository

## Why

`nb-api` currently lives as a workspace member inside `emcd/nb-mcp-server`. This creates several friction points:

- **Tag collision risk**: Both crates share one repository, so release tags must be prefixed (`nb-mcp-server-v0.13.0`, `nb-api-v0.1.0`) or scoped with tooling workarounds. A separate repo gets clean unprefixed `vX.Y.Z` tags.
- **Publish automation friction**: crates.io release workflows expect one crate per repo. Dual-crate repos need custom publish sequencing, path+version dependency bookkeeping, and conditional CI steps.
- **Blurred ownership**: Consumers of `nb-api` (e.g., `nbspec`) must clone the MCP server repo and navigate a workspace to find the library they actually depend on.
- **CI coupling**: Server CI runs on every `nb-api` change and vice versa, even when unrelated.

Splitting `nb-api` into `emcd/nb-api` gives each repo a single crate, single tag stream, single publish workflow, and clearer ownership boundary.

## What Changes

- Create new `emcd/nb-api` repository containing only the `nb-api` crate source.
- Remove `nb-api/` workspace member from `emcd/nb-mcp-server`.
- `nb-mcp-server` depends on `nb-api` from crates.io (published version only, no path dependency).
- Each repo gets its own CI, release workflow, and tag policy.
- Update `nb-api` crate metadata (repository, homepage, documentation URLs) to point to `emcd/nb-api`.
- Update `nb-mcp-server` documentation to reference the new repository location.

## Capabilities

### New Capabilities

- `nb-api-repository`: Standalone `emcd/nb-api` repository with its own CI, release workflow, crates.io publish automation, and tag policy.

### Modified Capabilities

- `tool-surface`: No behavioral change, but `nb-mcp-server` dependency on `nb-api` switches from path+version to crates.io published version only.

## Impact

- Affected repos: `emcd/nb-mcp-server` (removes workspace member, switches to published dependency), `emcd/nb-api` (new repo)
- Affected crates.io: `nb-api` crate metadata updates (repository/homepage URLs)
- Affected consumers: `nbspec` and any future `nb-api` consumers get a cleaner repo to reference
- No consumer-facing behavioral change; this is a repository structure change
