//! First-class tool argument schemas (flat `id` addressing, text-first content).

use schemars::JsonSchema;
use serde::Deserialize;

use crate::nb::{LinePosition, LineRef, Occurrence, SearchMode, TaskStatus};

fn deserialize_plain_string_with_field<'de, D>(
    deserializer: D,
    field: &str,
) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(s),
        other => Err(serde::de::Error::custom(format!(
            "field `{field}` expects a plain UTF-8 string, got {}",
            match other {
                serde_json::Value::Null => "null",
                serde_json::Value::Bool(_) => "boolean",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::Array(_) => "array",
                serde_json::Value::Object(_) => "object",
                serde_json::Value::String(_) => "string",
            }
        ))),
    }
}

fn deserialize_plain_string_pattern<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_plain_string_with_field(d, "pattern")
}
fn deserialize_plain_string_replacement<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_plain_string_with_field(d, "replacement")
}
fn deserialize_plain_string_title<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_plain_string_with_field(d, "title")
}
fn deserialize_plain_string_new_body<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_plain_string_with_field(d, "new_body")
}
fn deserialize_plain_string_content<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_plain_string_with_field(d, "content")
}
fn deserialize_plain_string_search_pattern<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_plain_string_with_field(d, "pattern")
}
/// Parameters for the nb meta-tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NbCall {
    /// Subcommand to execute (e.g., "status", "add", "list").
    pub(crate) command: String,
    /// Arguments for the subcommand as a JSON object.
    #[schemars(with = "std::collections::BTreeMap<String, serde_json::Value>")]
    #[serde(default)]
    pub(crate) args: serde_json::Value,
}

/// Parameters for the help tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct HelpParams {
    /// Namespace or command to get help for (e.g., "nb" or "nb.add").
    pub(crate) query: String,
}

// Command-specific argument structs

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct StatusArgs {
    /// Notebook to check status for (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AddArgs {
    /// Title for the note.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) title: Option<String>,
    /// Content of the note. Markdown is supported.
    pub(crate) content: String,
    /// Tags to apply (without # prefix).
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    /// Folder path to create the note in; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) folder: Option<String>,
    /// Bare notebook name to add to; do not include folders or selectors.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ShowArgs {
    /// Notebook selector, note ID, filename, or title to show; not a filesystem path.
    #[serde(alias = "selector")]
    pub(crate) id: String,
    /// Bare notebook name to read from (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeleteArgs {
    /// Notebook selector, note ID, filename, or title to delete; not a filesystem path.
    #[serde(alias = "selector")]
    pub(crate) id: String,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MoveArgs {
    /// Notebook selector, note ID, filename, or title to move/rename; not a filesystem path.
    #[serde(alias = "selector")]
    pub(crate) id: String,
    /// Destination path or new name. Do not include notebook-qualified syntax.
    /// Can be a folder path (ending with /) or a new filename.
    /// Examples: "new-folder/" (move to folder), "new-name.md" (rename), "folder/new-name.md" (move and rename).
    pub(crate) destination: String,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListArgs {
    /// Folder path to list; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) folder: Option<String>,
    /// Filter by tags (without # prefix).
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    /// Maximum number of items to return.
    #[serde(default)]
    #[schemars(with = "u32")]
    pub(crate) limit: Option<u32>,
    /// Bare notebook name to list from (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchArgs {
    /// Search terms/patterns (supports regex). Provide one or more terms.
    pub(crate) queries: Vec<String>,
    /// Search mode: `any` (default, OR) or `all` (AND).
    #[serde(default)]
    pub(crate) mode: SearchMode,
    /// Filter by tags (without # prefix).
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    /// Folder path to search within; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) folder: Option<String>,
    /// Bare notebook name to search in (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TodoArgs {
    /// Short title for the todo item.
    pub(crate) title: String,
    /// Optional longer description/body for the todo item.
    #[serde(alias = "content", default)]
    #[schemars(with = "String")]
    pub(crate) description: Option<String>,
    /// Optional checklist task titles to add to the todo.
    #[serde(default)]
    pub(crate) tasks: Vec<String>,
    /// Tags to apply (without # prefix).
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    /// Folder path to create the todo in; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) folder: Option<String>,
    /// Bare notebook name to add todo to; do not include folders or selectors.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskIdArgs {
    /// Notebook selector, todo ID, filename, or title to mark as done/undone; not a filesystem path.
    #[serde(alias = "selector")]
    pub(crate) id: String,
    /// Optional task number within a todo item.
    #[serde(alias = "task", default)]
    #[schemars(with = "u32")]
    pub(crate) task_number: Option<u32>,
    /// Bare notebook name containing the todo (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TasksArgs {
    /// Folder path to list todos from; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) folder: Option<String>,
    /// Optional status filter (`open` or `closed`).
    #[serde(alias = "state", default)]
    #[schemars(with = "TaskStatus")]
    pub(crate) status: Option<TaskStatus>,
    /// Whether to include tasks from descendant folders.
    /// Defaults to true; set false for folder-only scope.
    #[serde(default = "default_true", alias = "recurse")]
    pub(crate) recursive: bool,
    /// Bare notebook name to list todos from (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

impl Default for TasksArgs {
    fn default() -> Self {
        Self {
            folder: None,
            status: None,
            recursive: true,
            notebook: None,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BookmarkArgs {
    /// URL to bookmark.
    pub(crate) url: String,
    /// Title for the bookmark.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) title: Option<String>,
    /// Tags to apply (without # prefix).
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    /// Comment or description.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) comment: Option<String>,
    /// Folder path to create the bookmark in; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) folder: Option<String>,
    /// Bare notebook name to add bookmark to; do not include folders or selectors.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct FoldersArgs {
    /// Parent folder path to list; do not include notebook-qualified syntax.
    #[serde(alias = "folder", default)]
    #[schemars(with = "String")]
    pub(crate) parent: Option<String>,
    /// Bare notebook name to list folders from (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MkdirArgs {
    /// Folder path to create; do not include notebook-qualified syntax.
    #[serde(alias = "folder")]
    pub(crate) path: String,
    /// Bare notebook name to create folder in (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImportArgs {
    /// File path or URL to import.
    pub(crate) source: String,
    /// Folder path to import into; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) folder: Option<String>,
    /// Filename to use in notebook (uses original name if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) filename: Option<String>,
    /// Convert HTML content to Markdown.
    #[serde(default)]
    pub(crate) convert: bool,
    /// Bare notebook name to import into; do not include folders or selectors.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

// Body-aware and line tool argument structs (direct-only, 0.3 surface).

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReplaceNoteBodyArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    pub(crate) id: String,
    /// New body content as plain UTF-8 text. Replaces the entire note body. No base64 opt-in.
    #[serde(deserialize_with = "deserialize_plain_string_new_body")]
    pub(crate) new_body: String,
    /// Body fingerprint from a preceding `show` (`b3:` + 64 lowercase hex). Required to prevent stale overwrites.
    pub(crate) fingerprint: String,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EditNoteSubstringArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    pub(crate) id: String,
    /// Byte pattern to find, as plain UTF-8 text.
    #[serde(deserialize_with = "deserialize_plain_string_pattern")]
    pub(crate) pattern: String,
    /// Replacement bytes, as plain UTF-8 text.
    #[serde(deserialize_with = "deserialize_plain_string_replacement")]
    pub(crate) replacement: String,
    /// Occurrence selector: `first`, `all`, or `nth` (with `n`).
    pub(crate) occurrence: Occurrence,
    /// Expected number of matches; mismatch rejects the edit.
    pub(crate) expected_count: u32,
    /// Optional body fingerprint from a preceding `show`; when present, mismatch rejects the edit.
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) fingerprint: Option<String>,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

/// MCP-facing line edit with plain UTF-8 content (no base64).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum McpLineEdit {
    Insert {
        at: LinePosition,
        #[serde(deserialize_with = "deserialize_plain_string_content")]
        content: String,
    },
    Delete {
        start: LineRef,
        end: LineRef,
    },
    Replace {
        start: LineRef,
        end: LineRef,
        #[serde(deserialize_with = "deserialize_plain_string_content")]
        content: String,
    },
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EditNoteLinesArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    pub(crate) id: String,
    /// Batch of disjoint line edits (insert/delete/replace) against anchors from one original snapshot. Content is plain UTF-8 text.
    pub(crate) edits: Vec<McpLineEdit>,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RetitleNoteArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    pub(crate) id: String,
    /// New title as plain UTF-8 text. Does not change the note path.
    #[serde(deserialize_with = "deserialize_plain_string_title")]
    pub(crate) title: String,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EditNoteTagsArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    pub(crate) id: String,
    /// Tags to add (with or without # prefix).
    #[serde(default)]
    pub(crate) add: Vec<String>,
    /// Tags to remove (with or without # prefix).
    #[serde(default)]
    pub(crate) remove: Vec<String>,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ShowNoteLinesArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    pub(crate) id: String,
    /// 1-based starting line; defaults to 1.
    #[serde(default)]
    #[schemars(with = "u32")]
    pub(crate) offset: Option<u32>,
    /// Maximum lines to return; defaults to 100.
    #[serde(default)]
    #[schemars(with = "u32")]
    pub(crate) limit: Option<u32>,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchNoteLinesArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    pub(crate) id: String,
    /// Byte pattern to search for within body lines, as plain UTF-8 text.
    #[serde(deserialize_with = "deserialize_plain_string_search_pattern")]
    pub(crate) pattern: String,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    pub(crate) notebook: Option<String>,
}
