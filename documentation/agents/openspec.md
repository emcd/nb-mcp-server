# OpenSpec Instructions

This project uses OpenSpec 1.x (OPSX), the action-based workflow. There is no
`openspec/AGENTS.md`; instructions are assembled dynamically by the CLI and
delivered through generated `/opsx:*` commands.

## Workflow

- `/opsx:propose "<idea>"` — create a complete change proposal in one step.
- `/opsx:explore` — think through ideas before committing to a change.
- `/opsx:new` / `/opsx:continue` / `/opsx:ff` — start a change, then create
  artifacts stepwise or all at once.
- `/opsx:apply` — implement tasks; mark checkboxes complete as you go.
- `/opsx:verify` — validate that implementation matches artifacts.
- `/opsx:sync` — sync delta specs into main specs.
- `/opsx:archive` — archive a completed change.

## CLI State Queries

- `openspec list` and `openspec list --specs` — active changes and specs.
- `openspec status --change <id>` — artifact state for a change.
- `openspec instructions <artifact> --change <id>` — authoring instructions.
- `openspec validate --all --strict` — validate changes and specs.

## Conventions

- Project configuration: `openspec/config.yaml` (default schema:
  `spec-driven`). Custom workflow schemas may be added under
  `openspec/schemas/` as the project evolves.
- Minimum OpenSpec version: >= 1.4.0. The 1.5 stores/worksets feature is
  unstable beta — do not depend on it.
- Change IDs: kebab-case, verb-led (`add-`, `update-`, `remove-`,
  `refactor-`).
- Every requirement uses SHALL/MUST and carries at least one
  `#### Scenario:` (exactly four hashtags).
- When a commit completes an OpenSpec task or requirement, update the
  relevant task status in the same commit.

Treat OpenSpec proposals like code: commit proposal files to a branch,
reviewers review the commit (`git show`), author amends as needed, merge when
settled. No notebook draft step.
