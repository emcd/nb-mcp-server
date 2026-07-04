# Change: Migrate nb-mcp-server from OpenSpec 0.17.2 to 1.x (OPSX)

## Why

The upcoming `nbspec` project (notebook-first OpenSpec orchestration) targets OpenSpec 1.x for its schema extensibility and action-based artifact graph. `nb-mcp-server` currently uses OpenSpec 0.17.2. The repo needs to migrate off 0.x before `nbspec` adoption. OpenSpec 0.x is a dead line; 1.x has the extension points `nbspec` needs.

Empirical testing confirms:
- `openspec init` on 1.x detects and migrates the 0.x layout automatically.
- Existing specs and completed changes pass `openspec validate --all` unchanged — no content migration needed.
- The `openspec` symlink survives the migration intact.

## What Changes

- Replace `openspec/AGENTS.md` (455-line 0.x workflow guide, deleted by `openspec init`) with OPSX skill/command model.
- Update `CLAUDE.md` OpenSpec Instructions section to reference OPSX skills/commands instead of `@openspec/AGENTS.md`.
- Create `documentation/agents/openspec.md` with OPSX workflow guidance.
- Generate `.opencode/commands/` and skills via `openspec init --tools opencode`.
- Create `openspec/config.yaml` with `spec-driven` schema.
- Pin minimum OpenSpec version to `>= 1.4.0` (1.5 stores feature is unstable beta — do not depend on it).

## Impact

- Affected specs: openspec-workflow (new capability for OPSX workflow instructions)
- Affected code: `CLAUDE.md`, `openspec/AGENTS.md` (replaced), `documentation/agents/openspec.md` (new), `.opencode/commands/` (generated), `openspec/config.yaml` (new)
- No change to existing spec content or completed change archives
- Pilot scope: `nb-mcp-server` only; durable home for fleet migration is the agents-common Copier template
