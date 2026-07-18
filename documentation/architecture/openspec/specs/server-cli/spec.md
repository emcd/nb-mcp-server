# server-cli Specification

## Purpose
Define the command-line behavior used to inspect and launch the nb MCP server.
## Requirements
### Requirement: Version flag
The system SHALL provide a `--version` CLI flag that prints the nb-mcp version
and exits successfully without starting the server.

#### Scenario: Version output
- **WHEN** a user invokes `nb-mcp --version`
- **THEN** the version is printed to stdout
- **AND** the process exits with status code 0
