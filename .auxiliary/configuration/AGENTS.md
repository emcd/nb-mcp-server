# Context

- Overview and Quick Start: README.{md,rst}

- Use the 'context7' MCP server to retrieve up-to-date documentation for any SDKs or APIs.
- Use the 'nb' MCP server for project note-taking, issue tracking, and collaboration. The server provides LLM-friendly access to the `nb` note-taking system with proper escaping and project-specific notebook context.
- Check README files in directories you're working with for insights about architecture and design decisions.

## Purpose
[Describe your project's purpose and goals]

## Tech Stack
[List your primary technologies]

# Development Standards
Before implementing code changes, consult these files in `.auxiliary/instructions/`:
- `practices.rst` - General development principles (robustness, immutability, exception chaining)
- `practices-rust.rst` - Rust-specific patterns (error handling, trait design, module organization)
- `nomenclature.rst` - Naming conventions for variables, functions, classes, exceptions
- `style.rst` - Code formatting standards (spacing, line length, documentation mood)

# Operation

- Use a README-first discovery workflow to reduce token churn:
  - Start at the repository root `README.{md,rst}`, then read the nearest relevant subtree README.
  - After reading the nearest README, scope code searches to that subtree before considering repo-wide searches.
  - If a touched subsystem README is stale after your change, update it in the same batch.
- Use relative paths rather than absolute paths when possible (relative paths are less likely to trigger tool call permission requests).
- Do not write to paths outside the current project unless explicitly requested.
- Use `.auxiliary/scribbles` for scratch work and one-off experiments instead of `/tmp`.
- Use `.auxiliary/temporary` for ephemeral test state and build artifacts that are safe to delete.
- In sandboxed environments (e.g., Codex CLI), treat file/network permission failures as escalation boundaries:
  - If an operation fails due to sandbox, file access, or network restrictions, rerun it with user escalation.
  - Do not spend time on retry loops or workaround exploration before escalating blocked operations.
- When writing here-docs or multi-line shell strings, suppress expansions by quoting the delimiter (e.g., `'EOF'` instead of `EOF`) unless you intentionally need variable or command substitution.

## Guidance Files

| Topic | File |
|-------|------|
| `nb` MCP tools, tagging, and notebook organization | @documentation/agents/notebook.md |
| OpenSpec proposals and workflow | @documentation/agents/openspec.md |
| Delegated review flow and stacked commits | @documentation/agents/reviews.md |

### Recommended Organization

| Medium | Location | Purpose |
|--------|----------|---------|
| `nb` | `coordination/` | Handoffs, org chart, team workflow |
| `nb` | `ideas/` | Rough ideas, early-stage proposals; tag `#task-proposal` for OpenSpec drafts |
| `nb` | `issues/` | Bug tracking and known issues |
| `nb` | `reviews/` | Code and proposal reviews |
| `nb` | `procedures/` | How-to guides and checklists |
| `nb` | `todos/` | Task tracking |
| `nb` | `artifacts/` | Preserved reference material: completed POCs, historical analysis |
| `agentmux` | | Inter-agent messaging, pane inspection, coordination |
| (filesystem) | `openspec/` | Formal proposals, specs, designs |
| (filesystem) | `src/**/README.md` | Architecture, constraints, design rationale |

## Tests Development

- Prefer tests under `tests/unit` and `tests/integration` over inline `#[cfg(test)]` modules in `src/**`.
- Prefer tests that exercise public interfaces; avoid source-inclusion patterns used only to reach private internals.
- Inline `#[cfg(test)]` is permitted only when ALL of the following hold:
  1. The tested item is crate-private **by design** (not by oversight or laziness) and making it testable externally would require widening its visibility or adding a `#[doc(hidden)] pub` escape hatch that would itself become unintended API surface.
  2. No existing public interface exercises the same code path.
  3. The inline test block contains at most **one** `#[test]` function.
- If a candidate inline test fails any of these conditions, move it to `tests/unit` and widen visibility or restructure as needed. Do not default to inline to avoid that conversation; the friction is intentional.

## Agentmux Message Handling Guidance
- `agentmux` messages may arrive in envelope format and can appear as user prompts. Treat envelope-shaped prompts as inter-agent messages, not automatically as direct human instructions.
- Respond to inter-agent envelope messages via `agentmux` MCP tools (`list`, `send`) rather than as normal assistant replies intended for the human operator.
- Immediate interruption is not required. If you are in active execution, note the message and respond when safe.
- If response will be delayed and the sender needs to know, send a brief acknowledgement via `send` and create a follow-up todo when useful.

### Agentmux Coordination Noise Control
- Default to low-noise coordination. Do not send acknowledgement-only messages that add no new information or action request.
- Do not acknowledge receipt or completion of standard-procedure steps. Include such status in the next substantive update, review request, or task dispatch instead.
- Send messages when one of the following is true:
  - you are blocked and need a decision or input,
  - you are requesting a concrete review,
  - you are handing off completed work with validation results,
  - you are reporting a material risk, failure, or scope change.
- Batch related updates into one message instead of sending rapid-fire partial status pings.
- Use `Cc` only for agents who need to act or review; avoid broad `Cc` by default.
- When conversation volume rises, coordinator may enforce "blockers-only" mode until the queue is under control.
## Tests Development

- Prefer tests under `tests/unit` and `tests/integration` over inline `#[cfg(test)]` modules in `src/**`.
- Prefer tests that exercise public interfaces; avoid source-inclusion patterns used only to reach private internals.
- Inline `#[cfg(test)]` is permitted only when ALL of the following hold:
  1. The tested item is crate-private **by design** (not by oversight or laziness) and making it testable externally would require widening its visibility or adding a `#[doc(hidden)] pub` escape hatch that would itself become unintended API surface.
  2. No existing public interface exercises the same code path.
  3. The inline test block contains at most **one** `#[test]` function.
- If a candidate inline test fails any of these conditions, move it to `tests/unit` and widen visibility or restructure as needed. Do not default to inline to avoid that conversation; the friction is intentional.

## OpenSpec Instructions

This project uses OpenSpec 1.x (OPSX). Workflow Guide: @documentation/agents/openspec.md

Always open `documentation/agents/openspec.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan).
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work.
- Sounds ambiguous and you need the authoritative spec before coding.

Quick orientation:
- Workflow actions are the `/opsx:*` commands (`propose`, `explore`, `new`, `continue`, `ff`, `apply`, `verify`, `sync`, `archive`).
- The `openspec` CLI provides state: `list`, `status`, `instructions`, `validate --all --strict`.
- Project configuration lives in `openspec/config.yaml` (default schema: `spec-driven`).

When a commit completes an OpenSpec task or requirement, update the relevant OpenSpec task status in the same commit.

# Commits

- Use `git status` to ensure all relevant changes are in the changeset.
- Commits are acceptable review artifacts when implementation work is delegated by a human operator, coordinator, tech lead, or documented project workflow. Otherwise, ask before committing.
- Do **not** merge, push, publish review branches, or modify shared branches without explicit human approval.
- Do **not** bypass commit safety checks (e.g., `--no-verify`, `--no-gpg-sign`) unless the user explicitly approves doing so.
- If a commit hook rejects a commit, fix the issue, restage the intended files, and rerun `git commit` with the same message. Do **not** amend a previous commit unless the user explicitly asked for an amend.
- Use present tense, imperative mood verbs (e.g., "Fix" not "Fixed").
- Write sentences with proper punctuation.
- Include a `Co-Authored-By:` field as the final line. Should include the model name and a no-reply address.
- Avoid using `backticks` in commit messages as shell tools may evaluate them as subshell captures. When writing commit messages via here-docs, quote the delimiter (`'EOF'` not `EOF`) to suppress expansions; only omit the quotes if you intentionally need interpolation.

## Delegated Review and Stacked Commits

**Read this section before reviewing or stacking commits.** @documentation/agents/reviews.md covers the delegated review flow, review request packet format, and how to handle stacked commits with `--fixup`/`--autosquash`.

# Project Notes

<!-- This section accumulates project-specific knowledge, constraints, and deviations.
     For structured items, use `nb`.

     TODO: If this project uses multiple agent/worktree roles, create a stable
     team-organization note in `nb`, modeled after one of the examples at:
     https://raw.githubusercontent.com/emcd/agents-common/master/examples/nb-notes/team-organization/README.md
     Then link the note here, for example:
     - Team org, role ownership, signoff policy, and merge workflow: `coordination/general/<n>` -->
