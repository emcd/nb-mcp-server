# nb-mcp-server TODO

## Session 2025-01-26 (Claude Opus 4.5)

Drafted initial tools proposal. See `tools-proposal.md` for full design.

### Next Steps

1. [ ] **Project setup**
   - Create `Cargo.toml` with `rmcp`, `tokio`, `serde`, `schemars` deps
   - Create `src/main.rs` entry point
   - Create `src/mcp/` module structure (mirror litrpg pattern)

2. [ ] **Core infrastructure**
   - Implement `NbClient` to shell out to `nb` command
   - Handle notebook qualification (prefix all commands with `notebook:`)
   - Parse `nb` output into structured responses

3. [ ] **MVP tools** (in priority order)
   - `nb.status` — test the infrastructure
   - `nb.add` — note creation (the killer feature)
   - `nb.show` — note reading
   - `nb.list` — browsing
   - `nb.search` — finding notes
   - `nb.edit` — updating notes

4. [ ] **Configuration**
   - Environment variable for default notebook
   - Optional CWD-to-notebook mapping

### Open Questions

- Should we create a test notebook for development?
- What's the best way to handle `nb`'s interactive prompts (e.g., delete confirmation)?
  Probably: use `--force` flags where available, require `confirm: true` in MCP params

### Reference

- `nb` docs: https://github.com/xwmx/nb
- Existing MCP server pattern: `../rust-litrpg/src/mcp/`
- `rmcp` crate: https://crates.io/crates/rmcp
