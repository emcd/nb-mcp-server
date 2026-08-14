# Nbspec Instructions

Nbspec is the workflow for changes that touch normative specifications
or introduce new ones. The notebook is the source of truth for
proposals; the repository receives only durable documents at merge
time (specifications, designs, decisions, archives). The upstream
`openspec` CLI appears only as an optional, version-pinned CI
conformance oracle scoped to grammar-level fixtures — it never holds
in-flight proposals.

## Applicability

| Change | Workflow |
|---|---|
| Correct README, AGENTS, procedures, or agent guidance | Direct edit |
| Document existing behavior outside normative specifications | Direct edit |
| Routine implementation, refactor, or bug fix that does not alter a specification | Direct edit |
| Modify an existing normative specification, including editorial changes | Nbspec |
| Add a specification for a substantial codebase change | Nbspec |
| Continue work already covered by an active proposal | Update that proposal; do not create another |
| Operator explicitly requests a proposal | Nbspec |

"Substantial codebase change" includes: introducing a new public
capability or contract (a new API, a new CLI verb, a new config
option); breaking a backward-compatible contract; changing the
architecture or migrating between architectures; security or
performance work that touches the implementation surface. Trivial
implementation, refactor, or bug-fix code changes that do not
touch a specification are not substantial; they belong to row 3
(direct edit). When in doubt, ask the operator before creating a
proposal.

Any content edit to a merged normative specification changes its hash
and affects provenance. Use Nbspec for both semantic and editorial
specification changes so merge ownership and provenance remain
coherent. Do not treat `nbspec merge --force` as the routine path for
specification maintenance.

## Lifecycle (brief)

A change proceeds: author in the notebook → validate → record a
proposal review checkpoint approving verdict → implement → record an
implementation review checkpoint approving verdict → merge. The
runtime exposes one tool-enforced gate: `nbspec merge` refuses
without a current approving verdict (no verdict, stale approval,
outstanding revise, or unparseable verdict note). Recording both
verdicts in the right order — proposal review checkpoint before
implementation, implementation review checkpoint after — is operator
discipline; the runtime cannot enforce the sequencing. The Nbspec
CLI implementation is the source of truth for runtime enforcement;
this file is the procedure reference, not the implementation.

## Process discipline

- **Implementation does not start until a proposal review
  checkpoint approving verdict is recorded.** The first
  `nbspec review --verdict approve` binds to the proposal-stage
  rendered content. Implementation before the proposal review
  checkpoint violates procedural ordering — only durable-document
  edits mechanically stale a recorded verdict. The runtime
  cannot distinguish procedural freshness from content
  freshness, so a stale proposal-checkpoint verdict is the
  agent's responsibility to detect and re-record.

- **`work` tasks are updated as implementation progresses.** The
  work note is the live execution record. Mark each task `[x]`
  as it completes, not all at once at the end. Reviewers and the
  operator use `nbspec display <change-id>` to check progress
  during review; tasks not marked are tasks not done.

- **`nbspec merge` runs only after a fresh implementation review
  checkpoint approving verdict is recorded.** When implementation
  leaves durable docs unchanged, the aggregate is unchanged and
  `nbspec merge` would transfer identical content — the runtime
  gate does not catch the procedural-freshness gap. Recording a
  fresh implementation review checkpoint is operator discipline
  confirming the implementation still matches the approved
  durable docs.

- **The `work` note does not include meta-tasks.** No "update the
  work todo", "archive the proposal after merge", or
  "nbspec merge the change" tasks. The work note tracks the change
  being made (what the implementation does), not the lifecycle of
  the proposal itself (the lifecycle is the Nbspec CLI's
  responsibility). Meta-tasks duplicate tooling mechanics and
  bloat the live tracker.

## CLI surface

- `nbspec create <change-id>` — scaffold a notebook change namespace.
- `nbspec display <change-id> [--full]` — show change state and artifacts.
- `nbspec merge <change-id>` — transfer durable documents with
  provenance and drift protection.
- `nbspec render <change-id>` — deterministic scratch-tree render.
- `nbspec review <change-id>` — record a verdict against a rendered set.
- `nbspec validate <change-id>` — OpenSpec grammar validation with
  note-level diagnostics.
- `nbspec import <path>` (v0.3.0, forthcoming) — filesystem → notebook
  change interchange (and legacy archive tree conversion to
  deterministic tar.zst).
- `nbspec export <change-id> <path>` (v0.3.0, forthcoming) — notebook →
  filesystem change interchange.
- `nbspec serve mcp` — start the MCP server (one tool per CLI verb).

## MCP surface

`nbspec serve mcp` exposes one MCP tool per CLI verb. Tool descriptions
in the server schema carry the same conventions as this file; parameters
mirror the CLI flags (with the leading `--` dropped). Notebook context
is resolved by the server from the agent's working directory plus an
optional `--notebook` parameter.

## Conventions

- Change IDs are kebab-case, verb-led (`add-`, `update-`, `remove-`,
  `refactor-`).
- Every requirement uses SHALL/MUST and carries at least one
  `#### Scenario:` (exactly four hashtags).
- The `openspec/` directory is a bootstrap-only artifact (managed by
  agentsmgr; slated for removal in section 6.2 of the foundation
  change). New proposals never land there; the `openspec` CLI runs
  only in scratch fixture trees for grammar-level conformance checks.
- For rough ideas and early-stage proposals, use the notebook
  `ideas/` folder with the `#task-proposal` tag; convert to a formal
  proposal via `nbspec create` when ready for review.

## Authoring workflow

Treat Nbspec proposals like code: `nbspec display <change-id> [--full]`
is the review surface (notebook content with state, work progress, and
per-artifact readiness). For filesystem-tooling reviewers,
`nbspec render <change-id>` emits a deterministic scratch tree and
`nbspec render <change-id> --diff` emits a unified diff between the
rendered change and its current merge targets — both are suitable
for tools like `difit` (the rendered output lives in a scratch
workspace and does not land in git history, so `git show` does not
apply). Authors amend the change via notebook edits; merge happens
via the project's documented workflow.
