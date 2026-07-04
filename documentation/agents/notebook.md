# `nb` MCP Tools

The `nb` MCP server provides LLM-friendly access to the `nb` note-taking system with proper escaping and project-specific notebook context.

## When to Use
- **Project coordination**: Record handoffs, document project policies and procedures, maintain task lists.
- **Issue tracking**: Create and manage todos with status tracking.
- **Knowledge sharing**: Document patterns, APIs, and project-specific knowledge.
- **Meeting notes**: Record discussions and action items.

## Scope and Noise Control
- Prefer to update an existing related note/todo over creating a new one when context already exists.
- Avoid logging routine, immediately completed mechanical actions in separate notes.
- Create new notes/todos when information is likely to be useful across sessions or for other collaborators.

## Tagging Conventions
Use consistent tags for discoverability:
- **Project Component**: `#component-<name>` (e.g., `#component-data-models`)
- **Task Type**: `#task-<type>` (e.g., `#task-design`, `#task-bug`)
- **Status**: `#status-<state>` (e.g., `#status-in-progress`, `#status-review`)
- **Coordination**: `#handoff`, `#coordination`
- **Assignment**: Avoid owner tags (for example `#llm-*`) for task ownership. Use lane/folder ownership and explicit owner text in the note body when needed.

## Common Patterns
- Check for handoffs: use `search` with `#handoff` and `#status-review` tags.
- Find active component work: use `search` with `#component-<name>` and `#status-in-progress` tags.
- Track todos: use the `todo`, `tasks`, `do`, and `undo` tools.
- Organize with folders: use `folders` and `mkdir`.

## Choosing `todo` vs `add`
- Use the **`todo`** tool for any item with actionable state (open/done): todos, bugs, and issues. This gives the note a checkbox, enables `do`/`undo` state tracking, and makes it appear in `tasks` output.
- Use the **`add`** tool for everything else: coordination notes, decisions, designs, ideas, reference material, handoffs, and meeting notes.
- **Always specify a `folder`** when creating a note. A note created without a folder lands at the notebook root and is invisible to folder-scoped list views.
- Do not duplicate: if a bug or task is already tracked in `issues/<component>/` or `todos/<component>/`, do not also create a matching todo. Reference the existing selector in coordination notes or related todo bodies instead.

## Notebook Identifier Clarification
- Treat note selectors (for example `coordination/mcp/1`) as canonical IDs for operations on existing notes (`show`, `edit`, `delete`, etc.); do not supply a selector when creating new notes.
- Notebook MCP responses may include notebook-scoped identifiers (for example `my-project:coordination/...`) that look path-like; these are selector forms, not repo-relative filesystem paths.
- Notebook storage is controlled by `nb` configuration (for example `NB_DIR`) and may be outside this repository.
- Prefer notebook MCP commands to read/edit notes. Avoid assuming a selector maps to a file under the current repo.
- Use `help` to read full command schemas; key lookups: `search` with tag queries, `tasks` for open todos, `folders` to browse structure.

## Recommended `nb` Organization (Project-Defined)
- Prefer a folder taxonomy of `<issue-type>/<component>` (max depth 2) and avoid mixing top-level component folders with top-level issue-type folders.

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

## `nb` vs OpenSpec Rubric
- Use **OpenSpec proposals** for cross-cutting changes, contract-shaping work, architecture shifts, or work that needs explicit design discussion.
- Use **`nb` todos/notes** for scoped, self-contained implementation tasks where the path is straightforward.
- When in doubt about whether work needs an OpenSpec proposal or only `nb` execution tracking, prefer OpenSpec first for design clarity.
- For each active OpenSpec proposal, keep **exactly one** linked `nb` todo as the tracking anchor (with proposal reference), rather than duplicating full task trees in both systems.

## Handoff Hygiene
- Keep rolling handoff notes stable and update in place, separate from OpenSpec proposal content.
- Do not repurpose or overwrite rolling handoff notes with proposal content.
- Handoff content should be a brief summary of recent accomplishments and the current agenda. Replace the note body rather than appending so the handoff stays one screenful; a growing checkpoint log is an anti-pattern.
