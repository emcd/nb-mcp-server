## ADDED Requirements

### Requirement: OPSX workflow instructions
The project SHALL provide agent workflow instructions for OpenSpec 1.x (OPSX) via generated skills and commands, replacing the deleted `openspec/AGENTS.md` 0.x workflow guide.

#### Scenario: Agent discovers OPSX workflow
- **WHEN** an agent needs to create or validate an OpenSpec change proposal
- **THEN** the agent follows OPSX skill/command instructions rather than the deleted 0.x `openspec/AGENTS.md` guide

### Requirement: Minimum version pin
The project documentation SHALL specify OpenSpec minimum version `>= 1.4.0` and SHALL NOT depend on 1.5 features (stores, worksets).

#### Scenario: Version documented
- **WHEN** a contributor reads the project's OpenSpec workflow documentation
- **THEN** the documentation specifies `>= 1.4.0` and notes that 1.5 stores are out of scope

### Requirement: Instruction file consistency
`.auxiliary/configuration/AGENTS.md` (the real file behind `CLAUDE.md` and `AGENTS.md` symlinks) SHALL reference the OPSX skill/command model rather than the deleted `openspec/AGENTS.md` file.

#### Scenario: No broken references
- **WHEN** an agent reads `CLAUDE.md` or `AGENTS.md` for OpenSpec workflow guidance
- **THEN** all referenced files and include directives resolve correctly

### Requirement: Generated artifact policy
OPSX-generated skills and commands SHALL be treated as regenerable local artifacts, not vendored through agentsmgr defaults. The `.opencode` coder `.gitignore` SHALL include `commands/` (plural, written by OPSX 1.3+).

#### Scenario: Gitignore coverage
- **WHEN** `openspec init` or `openspec update` generates `commands/` under `.opencode/`
- **THEN** the `commands/` directory is gitignored and does not appear as untracked noise

### Requirement: Existing content preservation
All existing specs and completed change archives SHALL pass `openspec validate --all` unchanged after migration.

#### Scenario: Backward compatibility
- **WHEN** the migration to OPSX is complete
- **THEN** all existing spec content validates without modification

### Requirement: Pilot scope declaration
This migration SHALL cover `nb-mcp-server` only. The durable home for fleet-wide migration SHALL be the agents-common Copier template.

#### Scenario: Scope boundary
- **WHEN** the migration is complete
- **THEN** the proposal documents that fleet-wide propagation belongs to agents-common Copier template, not this change
