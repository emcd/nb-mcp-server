---
name: notebook-hygiene
description: Use when auditing, cleaning, or reorganizing an nb notebook; scopes work to one named notebook and conservatively handles stale notes, todos, handoffs, reviews, proposals, and folders.
---

# Notebook Hygiene

## Purpose

Restore a project notebook to a small, predictable working set without losing
active work, durable rationale, or lifecycle-managed records. Treat cleanup as
an information-architecture task, not a bulk deletion exercise.

## Entry Conditions

1. Identify one explicit target notebook. If the user did not name one, ask
   before continuing.
2. Read the repository instructions and local notebook guidance first. Prefer
   `AGENTS.md`, the root README, and files such as
   `documentation/agents/notebook.md` when present.
3. Use the `nb` MCP tools for notebook operations. Notebook selectors are not
   repository filesystem paths.
4. Do not inspect or modify other notebooks merely because the target notebook
   references them.
5. If the user requested an audit or assessment, do not mutate anything. If
   the user explicitly requested cleanup, apply only unambiguous changes and
   present ambiguous cases together for a decision.

For a non-trivial cleanup, track the audit, classification, mutation, and
verification phases with the session task-list tool.

## Audit

Build the inventory before changing anything:

1. List top-level folders and root-level items.
2. List child folders and their contents. Follow the depth allowed by local
   policy; do not assume every notebook uses the same component names.
3. Inventory open and completed todos. Confirm todo state from `[ ]` and `[x]`
   rather than decorative item glyphs.
4. Inspect candidate stale records, including old handoffs, completed reviews,
   resolved issues, session notes, test notes, root-level notes, and duplicated
   trackers.
5. Search for status tags, old selectors, references to completed trackers,
   and notes hidden at a category root alongside component subfolders.
6. Read each candidate before classifying it. Titles and age alone are not
   enough evidence for deletion.

Classify each candidate into one of these groups:

| Class | Treatment |
|---|---|
| Active | Keep in the local issue-type/component taxonomy. |
| Lifecycle-managed | Leave in place and use its owning workflow or CLI. |
| Durable history | Move to `artifacts/<component>` or the local equivalent. |
| Procedure | Move to `procedures/<component>` or the local equivalent. |
| Completed | Delete when the outcome is preserved in code, tests, releases, or durable documentation. |
| Stale or duplicate | Delete after confirming that no unique action or rationale remains. |
| Ambiguous | Preserve and ask one bundled clarification question. |

## Decision Rules

- Local project guidance overrides this skill's suggested taxonomy.
- Prefer an issue-type/component shape such as `todos/<component>`,
  `issues/<component>`, and `coordination/<component>` when local policy agrees.
- Keep one rolling handoff per component. Replace its body at checkpoints;
  never retain an append-only checkpoint log as the active handoff.
- Keep actionable work as todos. Do not preserve completed todos merely as
  history when commits, tests, release notes, or artifacts already record the
  result.
- Delete resolved defect reports when the implementation and regression tests
  are canonical. Preserve unique incident analysis as an artifact when it
  remains useful.
- Move completed reviews, investigations, experiments, and continuity records
  to artifacts when they retain useful evidence but no longer drive work.
- Remove scratch probes, test-shape notes, obsolete handoffs, duplicated
  procedures, and stale history-only notes with no owner or action.
- When an idea has become a formal proposal, keep the formal proposal
  canonical and delete or archive the draft according to local policy.
- Never manually move, rewrite, or delete OpenSpec-, Nbspec-, or otherwise
  lifecycle-managed proposal state. Record the exception and use the owning
  lifecycle command when its owner is available.
- Never delete a record only because it is old, has an unfamiliar tag, or
  references another project.

## Apply Changes

1. Create destination folders before moving records.
2. Serialize every mutation within one notebook. `nb` operations commit to the
   same Git repository, so parallel moves, edits, or deletes can contend on the
   index and fail nondeterministically.
3. Move durable records before deleting stale records. Capture the canonical
   selector returned by each move.
4. After every group of moves, update active notes that reference old
   selectors. Do not rewrite historical artifacts solely to modernize their
   contemporary references or status tags.
5. For content edits, read the complete current note immediately before an
   overwrite and preserve all intended content. Whole-note overwrite is not a
   substring replacement.
6. Shorten unwieldy active todo titles and remove stale ownership/status tags
   only when the intended current state is clear.
7. Delete completed and stale records one at a time. Do not batch concurrent
   deletions.
8. Delete empty placeholder and legacy folders leaf-first only after listing
   them and confirming they contain no records.
9. Do not create a cleanup log note. Update the rolling handoff only when the
   durable current agenda or notebook structure changed materially.

## Verification

Before declaring the cleanup complete:

1. Relist the notebook root and every active top-level category.
2. Confirm active todos, issues, ideas, procedures, coordination notes, and
   artifacts are in their intended folders.
3. Search for selectors that were removed or changed.
4. Search active work for stale status and owner tags. Historical artifacts may
   retain their original tags as evidence.
5. Confirm there is one concise rolling handoff per active component.
6. Confirm lifecycle-managed proposals remain intact.
7. Confirm no out-of-scope notebook was changed.
8. Report the remaining ambiguous or lifecycle-managed exceptions.

## Completion Report

Keep the report compact and factual:

- Target notebook and policy consulted.
- Counts or concise lists of records deleted, moved, and normalized.
- New or removed folders.
- Active work deliberately preserved.
- Managed or ambiguous records deferred, with the required next action.

Do not describe a cleanup as successful merely because the root looks tidy.
Success means active work is discoverable, historical material is separated,
selectors are coherent, and no authoritative state was lost.
