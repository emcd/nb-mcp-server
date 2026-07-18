# Change: Complete first-class MCP tool surface

## Why

The current implementation exposes only 4 of 17 subcommands as first-class MCP
tools (`add`, `search`, `todo`, `list`). This partial coverage forces agents to
use the multiplexed `nb` tool for most operations, which has two known issues:

1. **MiniMax M3 array-argument failure**: The multiplexed `args` dispatch path
   fails on array-valued fields (`queries[]`, `tags[]`, `tasks[]`) with
   `invalid type: map, expected a sequence`. This is a client-side tool-call
   parser limitation in MiniMax M3, not a server bug.
2. **MiMo/Xiaomi nullable-union truncation**: Xiaomi's tool-call serializer
   truncates JSON at the first `Option<T>` field rendered as `[T, "null"]` by
   schemars. This affects all tools with optional parameters.

First-class tools with typed schemas bypass the multiplexed dispatch path
(solving issue 1) and use `#[schemars(with="InnerType")]` to render optional
fields as plain types (solving issue 2). Completing the surface makes all
commands directly accessible without the multiplexed path.

## What Changes

- Hoist remaining 13 `nb` subcommands to first-class MCP tools:
  `status`, `notebooks`, `show`, `edit`, `delete`, `move`, `do`, `undo`,
  `tasks`, `bookmark`, `folders`, `mkdir`, `import`.
- Keep the multiplexed `nb` tool as the compact/backcompat compatibility surface.
- Drop the "experimental" label from all first-class tools; make them stable
  public API.
- Align empty-query error wording between multiplexed `nb.search` and
  first-class `search` (fixes `todos/mcp/47`).
- Ensure all `Option<T>` fields use `#[schemars(with="InnerType")]` for
  MiMo/Xiaomi schema compatibility.
- Update `README.md`, `src/README.md`, and OpenSpec specs.

## Design Decisions

### Tool naming

All first-class tools use unprefixed server-local names matching the `nb`
subcommand: `add`, `show`, `edit`, `delete`, `move`, `list`, `search`, `todo`,
`do`, `undo`, `tasks`, `bookmark`, `folders`, `mkdir`, `import`, `status`,
`notebooks`.

These names are intentionally generic (e.g., `list`, `do`, `status`). This is
accepted because: (a) the MCP tool descriptions provide sufficient context for
agents to select the correct tool, (b) clients that namespace by server (e.g.,
`nb_list`) already disambiguate, and (c) matching the `nb` CLI names reduces
cognitive load for users familiar with `nb`.

**Tradeoff: `do`/`undo` vs `complete`/`reopen`**

- `do` and `undo` are short and match the `nb` CLI exactly. However, `do` is
  less self-describing in raw MCP tool lists than `complete` -- an agent seeing
  `do` without context may not infer its purpose.
- `complete` and `reopen` are clearer for agents that encounter the tool
  without `nb` context, but diverge from the CLI and require alias mapping.
- **Decision**: Keep `do` and `undo` for CLI consistency. The tool description
  ("Mark a todo as complete") provides the necessary context. Internally, use
  `do_` (or `r#do`) since `do` is a reserved keyword in Rust; the
  `#[tool(name = "do")]` attribute registers the external name as `do`.

### MiMo/Xiaomi schema compatibility

All `Option<T>` fields on first-class tool arg structs MUST use
`#[schemars(with="InnerType")]` to render as plain single types instead of
nullable unions `[T, "null"]`. This is required for Xiaomi's tool-call
serializer, which truncates JSON at the first nullable-union property.

### Multiplexed `nb` preservation

The multiplexed `nb` tool remains as-is for backcompat and compact usage. No
changes to its behavior or schema. Agents that already use `nb` with
`command`/`args` continue to work.

## Impact

- Affected specs: `tool-surface`
- Affected code: `src/mcp.rs` (tool definitions, dispatch methods, help tool),
  `tests/integration/mcp_stdio.rs` (integration tests), `README.md`,
  `src/README.md`
