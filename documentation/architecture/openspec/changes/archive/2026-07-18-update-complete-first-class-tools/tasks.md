## 1. Implementation

- [x] 1.1 Add first-class tool definitions for remaining 13 subcommands in `src/mcp.rs`
- [x] 1.2 Add dispatch methods for each new first-class tool
- [x] 1.3 Ensure all `Option<T>` fields use `#[schemars(with="InnerType")]` for MiMo compatibility
- [x] 1.4 Update help tool to list all first-class tools (remove "experimental" label from `first_class_tools` list)
- [x] 1.5 Remove "experimental" language from existing first-class tool `#[tool(description)]` attributes (add, search, todo, list)
- [x] 1.6 Align empty-query error wording between multiplexed `nb.search` and first-class `search`

## 2. Testing

- [x] 2.1 Add integration tests for each new first-class tool (basic invocation, array args where applicable)
- [x] 2.2 Add `tools/list` tests verifying all 17 first-class tools are exposed
- [x] 2.3 Add schema regression tests for MiMo nullable-union fix on new tools
- [x] 2.4 Verify cross-surface equivalence between multiplexed and first-class tools

## 3. Documentation

- [x] 3.1 Update `README.md` with complete first-class tools section (remove "experimental" label)
- [x] 3.2 Update `src/README.md` MCP surface contract (remove "experimental" label)
- [x] 3.3 Update OpenSpec `tool-surface` spec with first-class tool requirements

## 4. Validation

- [x] 4.1 Run `cargo fmt`
- [x] 4.2 Run `cargo test`
- [x] 4.3 Run `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 4.4 Run `openspec validate update-complete-first-class-tools --strict`
