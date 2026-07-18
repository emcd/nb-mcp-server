# Change: Add --version flag to nb-mcp CLI

## Why
Users need a quick way to verify which nb-mcp version is installed without
starting the server.

## What Changes
- Add a `--version` CLI flag that prints the nb-mcp version and exits.
- Document the flag in help text and README usage.

## Impact
- Affected specs: `server-cli` (new capability)
- Affected code: `src/main.rs`, `README.md`
