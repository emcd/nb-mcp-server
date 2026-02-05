# Change: Update notebook resolution to avoid nb defaults

## Why
Using the nb default notebook is dangerous for a project-specific MCP server.
The server must always target a project-specific notebook, or fail clearly.

## What Changes
- **BREAKING**: Remove fallback to nb's default/current notebook.
- Derive a default notebook name from the Git master worktree path when
  no explicit notebook is configured.
- Fail with a clear error when no notebook can be resolved.

## Impact
- Affected specs: `notebook-resolution` (new capability)
- Affected code: `src/nb.rs`, `src/git_signing.rs`, `README.md`
