# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.14.0] - 2026-07-19

### Added

- Added actionable MCP diagnostics for folder/non-text `show` targets and
  duplicate title headings, with matching behavior across first-class and
  multiplexed tools.

### Changed

- Required callers to choose an explicit `edit` mode. The advertised
  whole-note mode is `overwrite`; `append` and `prepend` remain available, and
  `replace` remains accepted as a compatibility alias.
- Removed native usage hints from empty list and folder responses.

### Fixed

- Prevented inherited Git routing variables from redirecting notebook Git
  operations into the caller's repository.
- Corrected commit-signing overrides when the parent process already defines
  Git configuration environment entries.

## [0.13.0] - 2026-06-19

### Added

- Completed the typed first-class MCP tool surface alongside the compact
  multiplexed `nb` tool.
- Added direct tools for note, todo, organization, status, and notebook
  operations.

## [0.12.0] - 2026-06-02

### Changed

- Rejected unknown MCP command arguments instead of silently ignoring them.

## [0.11.0] - 2026-06-01

### Changed

- Hardened notebook routing validation for agent callers.
- Required folders for new content by default to protect notebook
  organization.

## [0.10.0] - 2026-05-11

### Changed

- Aligned todo creation arguments and documentation with the native `nb` CLI.

## [0.9.0] - 2026-04-04

### Changed

- Required MCP `args` payloads to be JSON objects.
- Required array-based search queries with an explicit matching mode.
- Returned actionable tool errors for argument validation failures.

## [0.8.0] - 2026-03-31

### Added

- Added checklist items to todo creation and recursive task listing by default.

### Changed

- Removed the confirmation field from delete operations.
- Normalized folder creation paths and clarified todo status output.

## [0.7.0] - 2026-02-24

### Changed

- Expanded todo task command shapes for native CLI parity.
- Added compatibility aliases and clearer argument-shape and scope guidance.

## [0.6.0] - 2026-02-17

### Added

- Added explicit `replace`, `append`, and `prepend` edit modes.

## [0.5.1] - 2026-02-12

### Fixed

- Prevented notebook initialization from blocking on Git signing prompts.

## [0.5.0] - 2026-02-05

### Added

- Added `--show-paths` for inspecting resolved notebook and state paths.

## [0.4.0] - 2026-02-05

### Changed

- Created missing notebooks automatically by default.

## [0.3.0] - 2026-02-04

### Added

- Added the `--version` CLI flag.

### Changed

- Hardened notebook resolution and the MCP tool surface.
- Renamed notebook configuration for clearer project routing.

## [0.2.0] - 2026-01-31

### Added

- Published the initial Rust MCP server for safe JSON-based access to the `nb`
  CLI.
- Added note, todo, bookmark, folder, notebook, import, move, and search
  operations with explicit notebook routing.
- Added XDG-compliant logging, ANSI stripping, and prebuilt release binaries.

[Unreleased]: https://github.com/emcd/nb-mcp-server/compare/v0.14.0...HEAD
[0.14.0]: https://github.com/emcd/nb-mcp-server/compare/v0.13.0...v0.14.0
[0.13.0]: https://github.com/emcd/nb-mcp-server/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/emcd/nb-mcp-server/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/emcd/nb-mcp-server/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/emcd/nb-mcp-server/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/emcd/nb-mcp-server/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/emcd/nb-mcp-server/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/emcd/nb-mcp-server/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/emcd/nb-mcp-server/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/emcd/nb-mcp-server/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/emcd/nb-mcp-server/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/emcd/nb-mcp-server/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/emcd/nb-mcp-server/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/emcd/nb-mcp-server/releases/tag/v0.2.0
