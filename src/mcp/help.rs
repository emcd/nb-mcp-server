//! `help` tool: exact schemas and descriptions for both surfaces.

use anyhow::Result;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content},
};

use super::schemas::*;

const ADD_TOOL_DESCRIPTION: &str = "Create a new note (DocumentKind::Note). Do NOT use for todos or checklists — use the `todo` tool instead; todo-shaped content (checkboxes, Tasks heading) is rejected. The folder field is required by default; use nb.mkdir to create folders and nb.folders to list them.";

/// Shared `todo` description: mirrors the `#[tool(name = "todo")]`
/// registration literally. Keep the two in sync (see above).
const TODO_TOOL_DESCRIPTION: &str = "Create a todo item (DocumentKind::Todo) with checkbox state and optional checklist tasks. Use this — not `add` — for anything with checkboxes or a Tasks section. The folder field is required by default; title is required; optional description/content and tasks[] create checklist items.";

pub(crate) fn help_tool(
    params: HelpParams,
) -> Result<CallToolResult, McpError> {
    let query = params.query.trim();

    let response = match query {
        "nb" => serde_json::json!({
            "namespace": "nb",
            "shape_hints": [
                "Invoke the nb tool with params: {\"command\":\"nb.<subcommand>\",\"args\":{...}}.",
                "This MCP API is a curated subset of nb CLI behavior and flags.",
                "args must be a JSON object; stringified JSON args are rejected.",
                "Unknown args are rejected instead of ignored; use exact schema fields or documented aliases.",
                "Common fields: id (alias selector), folder path, tags array, optional notebook override, plus task_number/status for todo commands.",
                "New note commands require folder by default. Use nb.mkdir to create new folders and nb.folders to list existing folders.",
                "The notebook field is only for selecting a notebook. Use folder, not notebook, to place new notes in folders.",
                "notebook must be a bare notebook name without ':' or '/'. folder must be a folder path without a notebook qualifier.",
                "Existing-item commands may accept copied id/selector values like notebook:folder/id, but reject conflicts with the notebook field.",
                "Returned ids such as coordination/mcp/1 or notebook:coordination/mcp/1 are nb selectors, not filesystem paths in the current repository.",
                "nb.search takes queries[] (required, non-empty), plus mode: any (default OR) or all (AND).",
                "nb.todo requires title; optional description (alias content) maps to nb --description; optional tasks[] creates checklist items via repeated --task flags.",
                "Compatibility aliases: note commands selector->id, nb.todo content->description, nb.folders folder->parent, nb.mkdir folder->path.",
                "nb.tasks is recursive by default; pass recursive:false to limit to the selected folder.",
                "In nb.list output, todo state comes from [ ] / [x] in titles; leading glyphs like ✔️ are item-type markers from nb.",
                "The legacy nb.edit command is removed in the nb-api 0.3 cutover. Use the direct body-aware tools replace_note_body, edit_note_substring, edit_note_lines, retitle_note, or edit_note_tags.",
                "Body-aware tools (replace_note_body, edit_note_substring, edit_note_lines, retitle_note, edit_note_tags) and line tools (show_note_lines, search_note_lines) are direct-only: they have no multiplexed aliases.",
                "Call help with query 'nb.<command>' for exact command schemas.",
                "First-class tools: add, show, delete, move, list, search, todo, do, undo, tasks, bookmark, folders, mkdir, import, status, notebooks, replace_note_body, edit_note_substring, edit_note_lines, retitle_note, edit_note_tags, show_note_lines, search_note_lines. These expose typed schemas directly and bypass the multiplexed command dispatch. Some clients may display these with server-prefixed names (e.g., nb_add)."
            ],
            "commands": [
                {"command": "nb.status", "description": "Show current notebook and stats"},
                {"command": "nb.notebooks", "description": "List available notebooks (list-only; no add/delete in MCP)"},
                {"command": "nb.add", "description": "Create a new note (DocumentKind::Note; not todos — use `todo`)"},
                {"command": "nb.show", "description": "Read a note's content (structured envelope)"},
                {"command": "nb.delete", "description": "Delete a note"},
                {"command": "nb.move", "description": "Move or rename a note"},
                {"command": "nb.list", "description": "List notes with optional filtering (todo state is [ ] / [x], not leading glyph icons)"},
                {"command": "nb.search", "description": "Full-text search notes (queries[] + mode any|all)"},
                {"command": "nb.todo", "description": "Create a todo item (DocumentKind::Todo; use for checklists, not `add`)"},
                {"command": "nb.do", "description": "Mark a todo as complete (optional task_number)"},
                {"command": "nb.undo", "description": "Reopen a completed todo (optional task_number)"},
                {"command": "nb.tasks", "description": "List todo items recursively (optional status: open|closed)"},
                {"command": "nb.bookmark", "description": "Save a URL as a bookmark (folder required by default)"},
                {"command": "nb.folders", "description": "List folders in notebook"},
                {"command": "nb.mkdir", "description": "Create a folder"},
                {"command": "nb.import", "description": "Import a file or URL into notebook (folder required by default)"},
            ],
            "first_class_tools": [
                {"tool": "status", "description": "Show current notebook and stats"},
                {"tool": "notebooks", "description": "List available notebooks"},
                {"tool": "add", "description": "Create a new note (DocumentKind::Note; not todos — use `todo`)"},
                {"tool": "show", "description": "Read a note's content (structured envelope)"},
                {"tool": "delete", "description": "Delete a note"},
                {"tool": "move", "description": "Move or rename a note"},
                {"tool": "list", "description": "List notes with optional filtering"},
                {"tool": "search", "description": "Full-text search notes (queries[] + mode any|all)"},
                {"tool": "todo", "description": "Create a todo item (DocumentKind::Todo; use for checklists, not `add`)"},
                {"tool": "do", "description": "Mark a todo as complete"},
                {"tool": "undo", "description": "Reopen a completed todo"},
                {"tool": "tasks", "description": "List todo items (optional status: open|closed)"},
                {"tool": "bookmark", "description": "Save a URL as a bookmark (folder required by default)"},
                {"tool": "folders", "description": "List folders in notebook"},
                {"tool": "mkdir", "description": "Create a folder"},
                {"tool": "import", "description": "Import a file or URL into notebook (folder required by default)"},
                {"tool": "replace_note_body", "description": "Replace the entire note body (fingerprint required, plain UTF-8 text)"},
                {"tool": "edit_note_substring", "description": "Replace occurrences of a text pattern in the body (plain UTF-8)"},
                {"tool": "edit_note_lines", "description": "Apply a batch of disjoint anchored line edits"},
                {"tool": "retitle_note", "description": "Change a note's title without changing its path"},
                {"tool": "edit_note_tags", "description": "Add and/or remove tags atomically"},
                {"tool": "show_note_lines", "description": "List body lines with anchors and window metadata"},
                {"tool": "search_note_lines", "description": "Search body line texts and return anchored matches"},
            ],
            "invoke": {
                "tool": "nb",
                "params": {"command": "nb.<subcommand>", "args": {}},
            },
        }),
        "nb.status" => command_help(
            "nb.status",
            "Show notebook status",
            json_schema_for::<StatusArgs>(),
        ),
        "nb.add" => command_help(
            "nb.add",
            ADD_TOOL_DESCRIPTION,
            json_schema_for::<AddArgs>(),
        ),
        "nb.show" => command_help(
            "nb.show",
            "Read a note's content as a text-first structured envelope (selector, path, kind, todo_state, title, tags, body, body_contiguous, fingerprint)",
            json_schema_for::<ShowArgs>(),
        ),
        "nb.delete" => command_help(
            "nb.delete",
            "Delete a note",
            json_schema_for::<DeleteArgs>(),
        ),
        "nb.move" => command_help(
            "nb.move",
            "Move or rename a note. Can move between folders or rename the file.",
            json_schema_for::<MoveArgs>(),
        ),
        "nb.list" => command_help(
            "nb.list",
            "List notes with optional filtering. Todo status is [ ] / [x] in titles; leading glyphs (for example ✔️) are item markers from nb.",
            json_schema_for::<ListArgs>(),
        ),
        "nb.search" => command_help(
            "nb.search",
            "Full-text search notes (queries[] required; mode:any default OR, mode:all for AND)",
            json_schema_for::<SearchArgs>(),
        ),
        "nb.todo" => command_help(
            "nb.todo",
            TODO_TOOL_DESCRIPTION,
            json_schema_for::<TodoArgs>(),
        ),
        "nb.do" => command_help(
            "nb.do",
            "Mark a todo as complete (optional task_number)",
            json_schema_for::<TaskIdArgs>(),
        ),
        "nb.undo" => command_help(
            "nb.undo",
            "Reopen a completed todo (optional task_number)",
            json_schema_for::<TaskIdArgs>(),
        ),
        "nb.tasks" => command_help(
            "nb.tasks",
            "List todo items recursively (optional status: open|closed, set recursive:false for folder-only)",
            json_schema_for::<TasksArgs>(),
        ),
        "nb.bookmark" => command_help(
            "nb.bookmark",
            "Save a URL as a bookmark. The folder field is required by default.",
            json_schema_for::<BookmarkArgs>(),
        ),
        "nb.folders" => command_help(
            "nb.folders",
            "List folders in notebook",
            json_schema_for::<FoldersArgs>(),
        ),
        "nb.mkdir" => command_help(
            "nb.mkdir",
            "Create a folder",
            json_schema_for::<MkdirArgs>(),
        ),
        "nb.import" => command_help(
            "nb.import",
            "Import a file or URL into notebook. The folder field is required by default.",
            json_schema_for::<ImportArgs>(),
        ),
        "nb.notebooks" => command_help(
            "nb.notebooks",
            "List available notebooks (list-only; notebook creation/deletion is not exposed via MCP)",
            serde_json::json!({"type": "object", "properties": {}}),
        ),
        // First-class tool help.
        "status" => first_class_help(
            "status",
            "Show current notebook and stats.",
            json_schema_for::<StatusArgs>(),
        ),
        "notebooks" => first_class_help(
            "notebooks",
            "List available notebooks (list-only; notebook creation/deletion is not exposed via MCP).",
            serde_json::json!({"type": "object", "properties": {}}),
        ),
        "add" => first_class_help(
            "add",
            ADD_TOOL_DESCRIPTION,
            json_schema_for::<AddArgs>(),
        ),
        "show" => first_class_help(
            "show",
            "Read a note's content as a text-first structured envelope (selector, path, kind, todo_state, title, tags, body, body_contiguous, fingerprint).",
            json_schema_for::<ShowArgs>(),
        ),
        "delete" => first_class_help(
            "delete",
            "Delete a note.",
            json_schema_for::<DeleteArgs>(),
        ),
        "move" => first_class_help(
            "move",
            "Move or rename a note.",
            json_schema_for::<MoveArgs>(),
        ),
        "list" => first_class_help(
            "list",
            "List notes with optional filtering.",
            json_schema_for::<ListArgs>(),
        ),
        "search" => first_class_help(
            "search",
            "Full-text search notes (queries[] required; mode: any default OR, mode: all for AND).",
            json_schema_for::<SearchArgs>(),
        ),
        "todo" => first_class_help(
            "todo",
            TODO_TOOL_DESCRIPTION,
            json_schema_for::<TodoArgs>(),
        ),
        "do" => first_class_help(
            "do",
            "Mark a todo as complete (optional task_number).",
            json_schema_for::<TaskIdArgs>(),
        ),
        "undo" => first_class_help(
            "undo",
            "Reopen a completed todo (optional task_number).",
            json_schema_for::<TaskIdArgs>(),
        ),
        "tasks" => first_class_help(
            "tasks",
            "List todo items (optional status: open|closed, recursive defaults to true).",
            json_schema_for::<TasksArgs>(),
        ),
        "bookmark" => first_class_help(
            "bookmark",
            "Save a URL as a bookmark. The folder field is required by default.",
            json_schema_for::<BookmarkArgs>(),
        ),
        "folders" => first_class_help(
            "folders",
            "List folders in notebook.",
            json_schema_for::<FoldersArgs>(),
        ),
        "mkdir" => first_class_help(
            "mkdir",
            "Create a folder.",
            json_schema_for::<MkdirArgs>(),
        ),
        "import" => first_class_help(
            "import",
            "Import a file or URL into notebook. The folder field is required by default.",
            json_schema_for::<ImportArgs>(),
        ),
        "replace_note_body" => first_class_help(
            "replace_note_body",
            "Replace the entire body of a note. Requires the body fingerprint from a preceding `show`.",
            json_schema_for::<ReplaceNoteBodyArgs>(),
        ),
        "edit_note_substring" => first_class_help(
            "edit_note_substring",
            "Replace one or more occurrences of a byte pattern in a note body.",
            json_schema_for::<EditNoteSubstringArgs>(),
        ),
        "edit_note_lines" => first_class_help(
            "edit_note_lines",
            "Apply a batch of disjoint anchored line edits to a note body.",
            json_schema_for::<EditNoteLinesArgs>(),
        ),
        "retitle_note" => first_class_help(
            "retitle_note",
            "Change a note's title without changing its path.",
            json_schema_for::<RetitleNoteArgs>(),
        ),
        "edit_note_tags" => first_class_help(
            "edit_note_tags",
            "Add and/or remove tags on a note atomically.",
            json_schema_for::<EditNoteTagsArgs>(),
        ),
        "show_note_lines" => first_class_help(
            "show_note_lines",
            "List body lines with anchors and window metadata.",
            json_schema_for::<ShowNoteLinesArgs>(),
        ),
        "search_note_lines" => first_class_help(
            "search_note_lines",
            "Search body line texts and return anchored matches.",
            json_schema_for::<SearchNoteLinesArgs>(),
        ),
        _ => {
            return Err(McpError::invalid_params(
                "unknown query; try 'nb' for command list",
                Some(serde_json::json!({"query": query})),
            ));
        },
    };

    Ok(CallToolResult::success(vec![Content::json(response)?]))
}

fn command_help(
    command: &str,
    description: &str,
    schema: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "command": command,
        "description": description,
        "args_schema": schema,
        "invoke": {
            "tool": "nb",
            "params": {"command": command, "args": {}},
        },
    })
}

fn first_class_help(
    tool: &str,
    description: &str,
    schema: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "tool": tool,
        "description": description,
        "args_schema": schema,
        "invoke": {
            "tool": tool,
            "params": {},
        },
    })
}

fn json_schema_for<T: schemars::JsonSchema>() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(T))
        .unwrap_or(serde_json::Value::Null)
}
