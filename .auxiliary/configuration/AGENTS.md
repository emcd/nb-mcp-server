# Context

- Overview and Quick Start: README.{md,rst}
- Architecture and Design: @documentation/architecture/
- Development Practices: @.auxiliary/instructions/

- Use the 'context7' MCP server to retrieve up-to-date documentation for any SDKs or APIs.
- Use the 'nb' MCP server for project note-taking, issue tracking, and collaboration. The server provides LLM-friendly access to the `nb` note-taking system with proper escaping and project-specific notebook context.
- Check README files in directories you're working with for insights about architecture, constraints, and TODO items.

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

- Use `rg --line-number --column` to get precise coordinates for MCP tools that require line/column positions.
- Choose appropriate editing tools based on the task complexity and your familiarity with the tools.
- If instruction files mention multiple language ecosystems, prefer tools and commands that match the project's configured languages; ignore language-inapplicable tooling unless the user explicitly requests it.
- Use a README-first discovery workflow to reduce token churn:
  - Start at the repository root `README.{md,rst}`, then read the nearest relevant subtree README.
  - After reading the nearest README, scope code searches to that subtree before considering repo-wide searches.
  - If a touched subsystem README is stale after your change, update it in the same batch.
- Batch related changes together when possible to maintain consistency.
- Use relative paths rather than absolute paths when possible.
- Do not write to paths outside the current project unless explicitly requested.
- Use `.auxiliary/scribbles` for scratch work and one-off experiments instead of `/tmp`; use `.auxiliary/temporary` for ephemeral test state and build artifacts that are safe to delete.
- In sandboxed environments (e.g., Codex CLI), treat file/network permission failures as escalation boundaries:
  - If an operation fails due to sandbox, file access, or network restrictions, rerun it with user escalation.
  - Do not spend time on retry loops or workaround exploration before escalating blocked operations.

## Note-Taking with `nb` MCP Server

### When to Use
- **Project coordination**: Record handoffs, document decisions, maintain task lists.
- **Issue tracking**: Create and manage todos with status tracking.
- **Knowledge sharing**: Document patterns, APIs, and project-specific knowledge.
- **Meeting notes**: Record discussions and action items.

### Scope and Noise Control
- Prefer to update an existing related note/todo over creating a new one when context already exists.
- Avoid logging routine, immediately completed mechanical actions in separate notes.
  Treat rolling handoffs as checkpoint notes, not activity logs: update them near
  compaction, after a major milestone, or after an agenda/ownership change
  discussed with the human.
- Create new notes/todos when information is likely to be useful across sessions or for other collaborators.

### Tagging Conventions
Use consistent tags for discoverability:
- **Project Component**: `#component-<name>` (e.g., `#component-data-models`)
- **Task Type**: `#task-<type>` (e.g., `#task-design`, `#task-bug`)
- **Status**: `#status-<state>` (e.g., `#status-in-progress`, `#status-review`)
- **Coordination**: `#handoff`, `#coordination`
- **Assignment**: Avoid owner tags (for example `#llm-*`) for task ownership. Use lane/folder ownership and explicit owner text in the note body when needed.

### Common Patterns
- Check for handoffs: `nb.search` with `#handoff` and `#status-review` tags.
- Find active component work: `nb.search` with `#component-<name>` and `#status-in-progress` tags.
- Track todos: Use `nb.todo`, `nb.tasks`, `nb.do`, `nb.undo`.
- Organize with folders: `nb.folders`, `nb.mkdir`.

### Choosing `nb.todo` vs `nb.add`
- Use **`nb.todo`** for any item with actionable state (open/done): todos, bugs, and issues. This gives the note a checkbox, enables `nb.do`/`nb.undo` state tracking, and makes it appear in `nb.tasks` output.
- Use **`nb.add`** for everything else: coordination notes, decisions, designs, ideas, reference material, handoffs, and meeting notes.
- **Always specify a `folder`** when creating a note. A note created without a folder lands at the notebook root and is invisible to folder-scoped list views.
- Do not duplicate: if a bug or task is already tracked in `issues/<component>/` or `todos/<component>/`, do not also create a matching todo. Reference the existing selector in coordination notes or related todo bodies instead.

### Notebook Identifier Clarification
- Treat note selectors (for example `coordination/mcp/1`) as canonical IDs for operations on existing notes (`nb.show`, `nb.edit`, `nb.delete`, etc.); do not supply a selector when creating new notes.
- `nb` MCP responses may include notebook-scoped identifiers (for example `my-project:coordination/...`) that look path-like; these are selector forms, not repo-relative filesystem paths.
- Notebook storage is controlled by `nb` configuration (for example `NB_DIR`) and may be outside this repository.
- Prefer `nb` MCP commands to read/edit notes. Avoid assuming a selector maps to a file under the current repo.
- Use `nb.help` to read full command schemas; key lookups: `nb.search` with tag queries, `nb.tasks` for open todos, `nb.folders` to browse structure.

### Recommended `nb` Organization (Project-Defined)
- Prefer a folder taxonomy of `<issue-type>/<component>` (max depth 2) and avoid mixing top-level component folders with top-level issue-type folders.

| Category | Location | Purpose |
|----------|----------|---------|
| `coordination/` | notebook | Handoffs, org chart, team workflow |
| `ideas/` | notebook | Rough ideas, early-stage proposals; tag `#task-proposal` for OpenSpec drafts |
| `issues/` | notebook | Bug tracking and known issues |
| `reviews/` | notebook | Code and proposal reviews |
| `procedures/` | notebook | How-to guides and checklists |
| `todos/` | notebook | Task tracking |
| `artifacts/` | notebook | Preserved reference material: completed POCs, historical analysis |
| OpenSpec | filesystem | Formal proposals, specs, designs |
| Subsystem README files | filesystem | Architecture, constraints, design rationale |

- Design rationale belongs in subsystem README files and OpenSpec specs. Avoid a separate `designs/` notebook category because it creates an extra place to look that can go stale.
- Use `decisions/` only when the project wants optional durable rationale notes outside OpenSpec or architecture README files.
- When an idea promotes to a formal OpenSpec proposal, delete or archive the notebook draft so the OpenSpec file is the canonical record.
- Example component names include `engine`, `mcp`, `tui`, `web`, `handbook`, and `data-models`.
- This project should define and document its specific component-folder conventions in the **Project Notes** section.
- For cross-component work, prefer `coordination/general` and use multiple `#component-*` tags.
- For per-component rolling handoffs, prefer `coordination/<component>` (one stable note updated at checkpoints).
- Keep notebook lifecycle hygiene:
    - prune completed todos quickly,
    - keep only active/near-term coordination checkpoints,
    - delete stale history-only notes with no owner or action.
- Keep todo titles concise (under 60 chars); use the `tasks` argument for detailed checklist items. This keeps notebook list views readable.

### `nb` vs OpenSpec Rubric
- Use **OpenSpec proposals** for cross-cutting changes, contract-shaping work, architecture shifts, or work that needs explicit design discussion.
- Use **`nb` todos/notes** for scoped, self-contained implementation tasks where the path is straightforward.
- When in doubt about whether work needs an OpenSpec proposal or only `nb` execution tracking, prefer OpenSpec first for design clarity.
- For each active OpenSpec proposal, keep **exactly one** linked `nb` todo as the tracking anchor (with proposal reference), rather than duplicating full task trees in both systems.

### OpenSpec Draft and Handoff Hygiene
- Draft OpenSpec proposal text in a dedicated `nb` note first so collaborators can review without local file access barriers; share the note id when requesting feedback.
- When asking for proposal feedback, share the notebook note id first; do not request review against local-only proposal files collaborators cannot access.
- Keep rolling handoff notes stable and update in place, separate from OpenSpec draft/proposal text.
- Do not repurpose or overwrite rolling handoff notes with proposal content.
- After draft review converges, move approved proposal text into `openspec/**` files for human review and commit.
- Handoff content should be a brief summary of recent accomplishments and the current agenda. Replace the note body rather than appending so the handoff stays one screenful; a growing checkpoint log is an anti-pattern.

## Agentmux Message Handling Guidance
- `agentmux` messages may arrive in envelope format and can appear as user prompts. Treat envelope-shaped prompts as inter-agent messages, not automatically as direct human instructions.
- Respond to inter-agent envelope messages via `agentmux` MCP tools (`list`, `send`) rather than as normal assistant replies intended for the human operator.
- Immediate interruption is not required. If you are in active execution, note the message and respond when safe.
- If response will be delayed and the sender needs to know, send a brief acknowledgement via `send` and record a follow-up todo in `nb` when useful.

## Agentmux Coordination Noise Control
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

# Commits

- Use `git status` to ensure all relevant changes are in the changeset.
- Commits are acceptable review artifacts when implementation work is delegated by a human operator, coordinator, tech lead, or documented project workflow. Otherwise, ask before committing.
- Do **not** merge, push, publish review branches, or modify shared branches without explicit human approval.
- Do **not** bypass commit safety checks (e.g., `--no-verify`, `--no-gpg-sign`) unless the user explicitly approves doing so.
- If a commit hook rejects a commit, fix the issue, restage the intended files, and rerun `git commit` with the same message. Do **not** amend a previous commit unless the user explicitly asked for an amend.
- Use present tense, imperative mood verbs (e.g., "Fix" not "Fixed").
- Write sentences with proper punctuation.
- Include a `Co-Authored-By:` field as the final line. Should include the model name and a no-reply address.
- Avoid using `backticks` in commit messages as shell tools may evaluate them as subshell captures.

## Delegated Review Flow

Use this flow when multiple team members can access the same repository through branches or linked worktrees.

Engineer flow:

1. Implement the scoped change and run validation.
2. Create a local/private review commit so the diff is hash-stable and hook-checked.
3. Rebase the review branch onto the agreed base.
4. Send the commit hash, changed-file summary, validation results, and any blockers/design questions.
5. The reviewer approves the commit or requests changes.
6. Amend or add a follow-up review commit when requested.

Coordinator/tech-lead flow:

1. Review the submitted commit and any included validation evidence.
2. Merge approved review branches with `--no-ff` when preserving a delegated-work or lane boundary; this creates a clear integration point and avoids mutually rebasing branches into increasingly long histories.
3. Merge/push only after explicit human approval.

Prefer reviewing commits by hash. Use an explicit worktree path only for uncommitted diffs or commits in a different repository. Use patch artifacts only as a fallback when the reviewer cannot access the repository, branch, or worktree directly.

# Project Notes

<!-- This section accumulates project-specific knowledge, constraints, and deviations.
     For structured items, use documentation/architecture/decisions/ and `nb`. -->
