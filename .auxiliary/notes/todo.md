# nb-mcp-server TODO

## Session 2025-01-30 (Claude Opus 4.5)

Implemented MVP functionality. All core commands working.

### Completed

- [x] **Project setup** — Cargo.toml, module structure
- [x] **Core infrastructure** — NbClient with notebook qualification
- [x] **MVP tools**:
  - [x] `nb.status` — notebook status
  - [x] `nb.notebooks` — list notebooks
  - [x] `nb.add` — create notes (with backtick support!)
  - [x] `nb.show` — read notes
  - [x] `nb.edit` — update notes
  - [x] `nb.delete` — delete notes (with confirm requirement)
  - [x] `nb.list` — list notes/folders
  - [x] `nb.search` — full-text search
  - [x] `nb.todo` — create todos
  - [x] `nb.do` / `nb.undo` — toggle todo status
  - [x] `nb.tasks` — list todos
  - [x] `nb.bookmark` — save URLs
  - [x] `nb.folders` — list folders
  - [x] `nb.mkdir` — create folders
- [x] **Configuration** — NB_MCP_NOTEBOOK environment variable

### Next Steps

1. [ ] **Integration testing** — Test via actual MCP client
2. [ ] **Documentation** — README with setup instructions
3. [ ] **Error handling improvements** — Better messages for common failures
4. [ ] **Output parsing** — Structured JSON responses instead of raw text

### Test Notebook

Created `nb-mcp-test` notebook for development testing.

```bash
# Quick test commands
nb nb-mcp-test:list --no-color
nb nb-mcp-test:tasks --no-color
```

### Design Decisions Made

- **Auto-install nb**: No. Detect missing `nb` and return helpful install instructions.
- **Confirmation for delete**: Required via `confirm: true` parameter.
- **Tag format**: Accept bare strings, prefix with `#` when invoking nb.
- **ANSI codes**: Use `--no-color` flag to avoid escape sequences in output.

### Reference

- `nb` docs: https://github.com/xwmx/nb
- Existing MCP server pattern: `../rust-litrpg/src/mcp/`
- `rmcp` crate: https://crates.io/crates/rmcp
