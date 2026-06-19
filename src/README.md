# MCP Surface Contract

This server intentionally exposes a curated MCP API rather than the full native
`nb` CLI grammar. Native `nb` often accepts compact positional selectors that
combine notebook, folder, and item identity. That shape is convenient for humans
at a shell, but it is easy for agents to misuse.

## Routing Fields

Use separate fields for routing whenever a command creates, lists, searches, or
organizes notebook content:

- `notebook` is a bare notebook name only.
- `folder` is a folder path only.
- `id` / `selector` identifies an existing note, todo, or item.

`notebook` must not contain selector syntax such as `project:todos/mcp`.
`folder` must not contain notebook-qualified syntax such as
`project:todos/mcp`.

## Existing Items

Commands that operate on existing items may accept copied native `nb` selectors
in the `id` field, with `selector` as an alias. Examples include:

- `nb.show`
- `nb.edit`
- `nb.delete`
- `nb.move` source item
- `nb.do`
- `nb.undo`

For these commands, values like `project:todos/mcp/35` are useful because they
come directly from `nb` output and refer to an existing item. If a copied
selector includes a notebook and the separate `notebook` field is also present,
the values must agree. Conflicts are rejected instead of guessed.

Embedded notebook selectors are checked as existing notebooks and must not
auto-create notebooks. This prevents typoed titles or selector-shaped strings
from creating stray notebooks.

## Creation Commands

Commands that create new items must use explicit routing fields:

- `nb.add`
- `nb.todo`
- `nb.bookmark`
- `nb.import`

These commands require `folder` by default. They may accept a bare `notebook`
override for cross-notebook collaboration, but they must not accept native
all-in-one selector syntax as a creation destination.

Do not add `selector` as an alias for creation destinations. If an agent passes
`selector` to a creation command, that should be an unknown-argument error.

## Filenames

Do not expose explicit filename control for new notes unless a concrete use case
requires it. Agents should normally address notes by the `nb`-generated selector
or id, not by storage filenames. Filename control increases overlap with
`title`, `folder`, and selector-like paths, making routing mistakes more likely.

`nb.import` may keep its `filename` argument because imported artifacts commonly
have meaningful external filenames.

## Unknown Arguments

Unknown command arguments must be rejected instead of ignored. Silent ignores
turn spelling mistakes and native-CLI assumptions into misplaced notes.

Keep intentional aliases narrow and documented. Current aliases exist for common
agent ergonomics, not for reproducing every native `nb` spelling:

- `selector` -> `id` on existing-item commands.
- `content` -> `description` on `nb.todo`.
- `folder` -> `parent` on `nb.folders`.
- `folder` -> `path` on `nb.mkdir`.
- `state` -> `status` on `nb.tasks`.
- `recurse` -> `recursive` on `nb.tasks`.
- `task` -> `task_number` on `nb.do` and `nb.undo`.

## Experimental First-Class Tools

The server also registers direct first-class tools (`add`, `search`, `todo`,
`list`) as experimental additive alternatives for commands with array arguments.
These bypass the multiplexed `nb` command dispatch and expose typed schemas
directly. The multiplexed `nb` tool remains the canonical surface. First-class
tools may be removed in a future release without deprecation period.

Note: MCP clients may display these tools with server-prefixed names (e.g.,
`nb_add` instead of `add`). The server-registered names are `add`, `search`,
`todo`, and `list`.

## Design Principle

Prefer one obvious way for agents to say where new content goes. Preserve copied
native selectors only for operations on existing content, where they reduce
friction without creating new routing ambiguity.
