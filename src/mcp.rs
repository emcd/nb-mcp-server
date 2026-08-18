use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::Config;
use crate::git_signing;
use crate::nb::{
    ByteString, CommitOutcome, DocumentKind, Fingerprint, LineAnchor, LineEdit, LinePosition,
    LineRef, LineTerminator, NbClient, NbError, NoteTarget, Occurrence, SearchMode,
    SearchNoteLines, ShowNote, ShowNoteLines, TaskStatus, TodoState,
};

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

#[derive(Clone)]
struct McpServer {
    nb: NbClient,
}

/// Parameters for the nb meta-tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NbCall {
    /// Subcommand to execute (e.g., "status", "add", "list").
    command: String,
    /// Arguments for the subcommand as a JSON object.
    #[schemars(with = "std::collections::BTreeMap<String, serde_json::Value>")]
    #[serde(default)]
    args: serde_json::Value,
}

/// Parameters for the help tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct HelpParams {
    /// Namespace or command to get help for (e.g., "nb" or "nb.add").
    query: String,
}

// Command-specific argument structs

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct StatusArgs {
    /// Notebook to check status for (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AddArgs {
    /// Title for the note.
    #[serde(default)]
    #[schemars(with = "String")]
    title: Option<String>,
    /// Content of the note. Markdown is supported.
    content: String,
    /// Tags to apply (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Folder path to create the note in; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    folder: Option<String>,
    /// Bare notebook name to add to; do not include folders or selectors.
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ShowArgs {
    /// Notebook selector, note ID, filename, or title to show; not a filesystem path.
    #[serde(alias = "selector")]
    id: String,
    /// Bare notebook name to read from (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DeleteArgs {
    /// Notebook selector, note ID, filename, or title to delete; not a filesystem path.
    #[serde(alias = "selector")]
    id: String,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MoveArgs {
    /// Notebook selector, note ID, filename, or title to move/rename; not a filesystem path.
    #[serde(alias = "selector")]
    id: String,
    /// Destination path or new name. Do not include notebook-qualified syntax.
    /// Can be a folder path (ending with /) or a new filename.
    /// Examples: "new-folder/" (move to folder), "new-name.md" (rename), "folder/new-name.md" (move and rename).
    destination: String,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ListArgs {
    /// Folder path to list; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    folder: Option<String>,
    /// Filter by tags (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Maximum number of items to return.
    #[serde(default)]
    #[schemars(with = "u32")]
    limit: Option<u32>,
    /// Bare notebook name to list from (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SearchArgs {
    /// Search terms/patterns (supports regex). Provide one or more terms.
    queries: Vec<String>,
    /// Search mode: `any` (default, OR) or `all` (AND).
    #[serde(default)]
    mode: SearchMode,
    /// Filter by tags (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Folder path to search within; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    folder: Option<String>,
    /// Bare notebook name to search in (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TodoArgs {
    /// Short title for the todo item.
    title: String,
    /// Optional longer description/body for the todo item.
    #[serde(alias = "content", default)]
    #[schemars(with = "String")]
    description: Option<String>,
    /// Optional checklist task titles to add to the todo.
    #[serde(default)]
    tasks: Vec<String>,
    /// Tags to apply (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Folder path to create the todo in; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    folder: Option<String>,
    /// Bare notebook name to add todo to; do not include folders or selectors.
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TaskIdArgs {
    /// Notebook selector, todo ID, filename, or title to mark as done/undone; not a filesystem path.
    #[serde(alias = "selector")]
    id: String,
    /// Optional task number within a todo item.
    #[serde(alias = "task", default)]
    #[schemars(with = "u32")]
    task_number: Option<u32>,
    /// Bare notebook name containing the todo (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TasksArgs {
    /// Folder path to list todos from; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    folder: Option<String>,
    /// Optional status filter (`open` or `closed`).
    #[serde(alias = "state", default)]
    #[schemars(with = "TaskStatus")]
    status: Option<TaskStatus>,
    /// Whether to include tasks from descendant folders.
    /// Defaults to true; set false for folder-only scope.
    #[serde(default = "default_true", alias = "recurse")]
    recursive: bool,
    /// Bare notebook name to list todos from (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
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
struct BookmarkArgs {
    /// URL to bookmark.
    url: String,
    /// Title for the bookmark.
    #[serde(default)]
    #[schemars(with = "String")]
    title: Option<String>,
    /// Tags to apply (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Comment or description.
    #[serde(default)]
    #[schemars(with = "String")]
    comment: Option<String>,
    /// Folder path to create the bookmark in; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    folder: Option<String>,
    /// Bare notebook name to add bookmark to; do not include folders or selectors.
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FoldersArgs {
    /// Parent folder path to list; do not include notebook-qualified syntax.
    #[serde(alias = "folder", default)]
    #[schemars(with = "String")]
    parent: Option<String>,
    /// Bare notebook name to list folders from (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MkdirArgs {
    /// Folder path to create; do not include notebook-qualified syntax.
    #[serde(alias = "folder")]
    path: String,
    /// Bare notebook name to create folder in (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ImportArgs {
    /// File path or URL to import.
    source: String,
    /// Folder path to import into; do not include notebook-qualified syntax.
    #[serde(default)]
    #[schemars(with = "String")]
    folder: Option<String>,
    /// Filename to use in notebook (uses original name if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    filename: Option<String>,
    /// Convert HTML content to Markdown.
    #[serde(default)]
    convert: bool,
    /// Bare notebook name to import into; do not include folders or selectors.
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

// Body-aware and line tool argument structs (direct-only, 0.3 surface).

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ReplaceNoteBodyArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    id: String,
    /// New body content as plain UTF-8 text. Replaces the entire note body. No base64 opt-in.
    #[serde(deserialize_with = "deserialize_plain_string_new_body")]
    new_body: String,
    /// Body fingerprint from a preceding `show` (`b3:` + 64 lowercase hex). Required to prevent stale overwrites.
    fingerprint: String,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EditNoteSubstringArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    id: String,
    /// Byte pattern to find, as plain UTF-8 text.
    #[serde(deserialize_with = "deserialize_plain_string_pattern")]
    pattern: String,
    /// Replacement bytes, as plain UTF-8 text.
    #[serde(deserialize_with = "deserialize_plain_string_replacement")]
    replacement: String,
    /// Occurrence selector: `first`, `all`, or `nth` (with `n`).
    occurrence: Occurrence,
    /// Expected number of matches; mismatch rejects the edit.
    expected_count: u32,
    /// Optional body fingerprint from a preceding `show`; when present, mismatch rejects the edit.
    #[serde(default)]
    #[schemars(with = "String")]
    fingerprint: Option<String>,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

/// MCP-facing line edit with plain UTF-8 content (no base64).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
enum McpLineEdit {
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
struct EditNoteLinesArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    id: String,
    /// Batch of disjoint line edits (insert/delete/replace) against anchors from one original snapshot. Content is plain UTF-8 text.
    edits: Vec<McpLineEdit>,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RetitleNoteArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    id: String,
    /// New title as plain UTF-8 text. Does not change the note path.
    #[serde(deserialize_with = "deserialize_plain_string_title")]
    title: String,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EditNoteTagsArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    id: String,
    /// Tags to add (with or without # prefix).
    #[serde(default)]
    add: Vec<String>,
    /// Tags to remove (with or without # prefix).
    #[serde(default)]
    remove: Vec<String>,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ShowNoteLinesArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    id: String,
    /// 1-based starting line; defaults to 1.
    #[serde(default)]
    #[schemars(with = "u32")]
    offset: Option<u32>,
    /// Maximum lines to return; defaults to 100.
    #[serde(default)]
    #[schemars(with = "u32")]
    limit: Option<u32>,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SearchNoteLinesArgs {
    /// Notebook selector, note ID, filename, or title; not a filesystem path. Alias `selector`.
    #[serde(alias = "selector")]
    id: String,
    /// Byte pattern to search for within body lines, as plain UTF-8 text.
    #[serde(deserialize_with = "deserialize_plain_string_search_pattern")]
    pattern: String,
    /// Bare notebook name containing the note (uses default if not specified).
    #[serde(default)]
    #[schemars(with = "String")]
    notebook: Option<String>,
}

/// Text-first slim `show` envelope (no base64, Option<String> title, String body).
#[derive(Debug, Serialize)]
struct ShowEnvelope {
    selector: String,
    path: String,
    kind: DocumentKind,
    todo_state: Option<TodoState>,
    title: Option<String>,
    tags: Vec<String>,
    body: String,
    body_contiguous: bool,
    fingerprint: Fingerprint,
}

impl ShowEnvelope {
    fn from_show(shown: ShowNote) -> Result<Self, NbError> {
        // Strict UTF-8 validation: non-UTF-8 source yields a typed recovery error.
        let source_bytes = shown.source.as_bytes()?;
        if std::str::from_utf8(&source_bytes).is_err() {
            let mime_hint = mime_hint_for_bytes(&source_bytes);
            return Err(NbError::ValidationError {
                reason: format!(
                    "Note `{}` source is not valid UTF-8 (detected {mime_hint}); no base64 on this change's textual read/edit MCP tool surface. \
                     Hint: retrieve raw bytes outside MCP with `nb show <selector> --print --no-color` or `nb show <selector> --print --raw` and handle locally.",
                    shown.selector
                ),
                location: None,
            });
        }
        let body_bytes = shown.body.as_bytes()?;
        let body = String::from_utf8(body_bytes).map_err(|_| NbError::ValidationError {
            reason: format!(
                "Note `{}` body is not valid UTF-8; no base64 on this change's textual read/edit MCP tool surface. \
                 Hint: retrieve raw bytes outside MCP with `nb show <selector>` and handle locally.",
                shown.selector
            ),
            location: None,
        })?;
        let title = match shown.title {
            Some(bytes) => {
                let raw = bytes.as_bytes()?;
                let s = String::from_utf8(raw).map_err(|_| NbError::ValidationError {
                    reason: format!(
                        "Note `{}` title is not valid UTF-8; no base64 on this change's textual read/edit MCP tool surface.",
                        shown.selector
                    ),
                    location: None,
                })?;
                // Mirror nb-api's title_text trimming: strip leading '#', surrounding
                // whitespace, and trailing newline so the MCP surface exposes plain text.
                let trimmed = s
                    .trim_end_matches('\n')
                    .trim_end_matches('\r')
                    .trim_start_matches('#')
                    .trim()
                    .to_string();
                Some(trimmed)
            }
            None => None,
        };
        Ok(Self {
            selector: shown.selector,
            path: shown.path,
            kind: shown.kind,
            todo_state: shown.todo_state,
            title,
            tags: shown.tags,
            body,
            body_contiguous: shown.body_contiguous,
            fingerprint: shown.fingerprint,
        })
    }
}

fn mime_hint_for_bytes(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if bytes.starts_with(&[0x25, b'P', b'D', b'F']) {
        "application/pdf"
    } else if bytes.starts_with(&[0x1F, 0x8B]) {
        "application/gzip"
    } else {
        "application/octet-stream"
    }
}

/// Text-first line envelope.
#[derive(Debug, Serialize)]
struct ShowNoteLinesEnvelope {
    selector: String,
    path: String,
    kind: DocumentKind,
    total_lines: u32,
    offset: u32,
    limit: u32,
    next_offset: Option<u32>,
    lines: Vec<NoteLineEnvelope>,
    title: Option<String>,
    body_fingerprint: Fingerprint,
}

#[derive(Debug, Serialize)]
struct NoteLineEnvelope {
    number: u32,
    anchor: LineAnchor,
    text: String,
    terminator: LineTerminator,
}

#[derive(Debug, Serialize)]
struct SearchNoteLinesEnvelope {
    selector: String,
    path: String,
    kind: DocumentKind,
    hits: Vec<HitEnvelope>,
    body_fingerprint: Fingerprint,
}

#[derive(Debug, Serialize)]
struct HitEnvelope {
    number: u32,
    anchor: LineAnchor,
    start_byte: u32,
    end_byte: u32,
    text: String,
}

fn convert_mcp_line_edits(edits: Vec<McpLineEdit>) -> Result<Vec<LineEdit>, NbError> {
    edits
        .into_iter()
        .map(|e| match e {
            McpLineEdit::Insert { at, content } => Ok(LineEdit::Insert {
                at,
                content: ByteString::from_bytes(content.into_bytes()),
            }),
            McpLineEdit::Delete { start, end } => Ok(LineEdit::Delete { start, end }),
            McpLineEdit::Replace {
                start,
                end,
                content,
            } => Ok(LineEdit::Replace {
                start,
                end,
                content: ByteString::from_bytes(content.into_bytes()),
            }),
        })
        .collect()
}

fn show_note_lines_result(result: Result<ShowNoteLines, NbError>) -> CallToolResult {
    match result {
        Ok(shown) => match ShowNoteLinesEnvelope::from_wire(shown) {
            Ok(envelope) => json_success(&envelope),
            Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
        },
        Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
    }
}

fn search_note_lines_result(result: Result<SearchNoteLines, NbError>) -> CallToolResult {
    match result {
        Ok(shown) => match SearchNoteLinesEnvelope::from_wire(shown) {
            Ok(envelope) => json_success(&envelope),
            Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
        },
        Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
    }
}

impl ShowNoteLinesEnvelope {
    fn from_wire(shown: ShowNoteLines) -> Result<Self, NbError> {
        let title = match shown.title {
            Some(bytes) => {
                let raw = bytes.as_bytes()?;
                let s = String::from_utf8(raw).map_err(|_| NbError::ValidationError {
                    reason: "show_note_lines title is not valid UTF-8".to_string(),
                    location: None,
                })?;
                let trimmed = s
                    .trim_end_matches('\n')
                    .trim_end_matches('\r')
                    .trim_start_matches('#')
                    .trim()
                    .to_string();
                Some(trimmed)
            }
            None => None,
        };
        let lines = shown
            .lines
            .into_iter()
            .map(|l| {
                let raw = l.text.as_bytes()?;
                let text = String::from_utf8(raw).map_err(|_| NbError::ValidationError {
                    reason: format!("line {} text is not valid UTF-8", l.number),
                    location: None,
                })?;
                Ok(NoteLineEnvelope {
                    number: l.number,
                    anchor: l.anchor,
                    text,
                    terminator: l.terminator,
                })
            })
            .collect::<Result<Vec<_>, NbError>>()?;
        Ok(Self {
            selector: shown.selector,
            path: shown.path,
            kind: shown.kind,
            total_lines: shown.total_lines,
            offset: shown.offset,
            limit: shown.limit,
            next_offset: shown.next_offset,
            lines,
            title,
            body_fingerprint: shown.body_fingerprint,
        })
    }
}

impl SearchNoteLinesEnvelope {
    fn from_wire(shown: SearchNoteLines) -> Result<Self, NbError> {
        let hits = shown
            .hits
            .into_iter()
            .map(|h| {
                let bytes = h.text.ok_or_else(|| NbError::ValidationError {
                    reason: format!(
                        "hit line {} text is absent; expected plain string (upstream None)",
                        h.number
                    ),
                    location: None,
                })?;
                let raw = bytes.as_bytes()?;
                let text = String::from_utf8(raw).map_err(|_| NbError::ValidationError {
                    reason: format!("hit line {} text is not valid UTF-8", h.number),
                    location: None,
                })?;
                Ok(HitEnvelope {
                    number: h.number,
                    anchor: h.anchor,
                    start_byte: h.start_byte,
                    end_byte: h.end_byte,
                    text,
                })
            })
            .collect::<Result<Vec<_>, NbError>>()?;
        Ok(Self {
            selector: shown.selector,
            path: shown.path,
            kind: shown.kind,
            hits,
            body_fingerprint: shown.body_fingerprint,
        })
    }
}

#[tool_router]
impl McpServer {
    fn new(config: &Config) -> Result<Self> {
        let nb_config = config.to_nb_api_config();
        let nb = NbClient::new(&nb_config)?;
        Ok(Self { nb })
    }

    #[tool(
        description = r#"nb note-taking tool. Invoke with {"command":"nb.<subcommand>","args":{...}} and pass args as a JSON object (stringified JSON is not accepted). Unknown args are rejected. This is a curated wrapper (not a 1:1 map of nb CLI flags). New note commands require folder by default; use nb.mkdir to create folders and nb.folders to list them. notebook must be a bare notebook name; use folder for folders and id/selector for notes. Note-targeting commands accept id (alias: selector); returned identifiers are nb selectors, not repo filesystem paths. In nb.list output, todo state is [ ] / [x]; leading glyphs like ✔️ are item markers from nb, not completion status. nb.search uses queries[] with optional mode any|all. Commands: status, add, show, delete, move, list, search, todo, do, undo, tasks, bookmark, folders, mkdir, notebooks, import. The legacy edit command is removed; use the direct body-aware tools (replace_note_body, edit_note_substring, edit_note_lines, retitle_note, edit_note_tags) and line tools (show_note_lines, search_note_lines). Use `help` for exact schemas."#
    )]
    async fn nb(&self, Parameters(call): Parameters<NbCall>) -> Result<CallToolResult, McpError> {
        self.dispatch_nb(call).await
    }

    #[tool(
        description = "Return sub-command help and JSON schemas. Query 'nb' for command list or 'nb.<command>' for details."
    )]
    async fn help(
        &self,
        Parameters(params): Parameters<HelpParams>,
    ) -> Result<CallToolResult, McpError> {
        help_tool(params)
    }

    // First-class tools with typed schemas.
    // These bypass the multiplexed command dispatch and expose typed schemas
    // directly. Reuse existing arg structs and dispatch logic.

    #[tool(
        name = "add",
        description = "Create a new note. The folder field is required by default; use nb.mkdir to create folders and nb.folders to list them."
    )]
    async fn nb_add(
        &self,
        Parameters(args): Parameters<AddArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_add(args).await
    }

    #[tool(
        name = "search",
        description = "Full-text search notes. queries[] is required (non-empty). mode: any (default, OR) or all (AND)."
    )]
    async fn nb_search(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_search(args).await
    }

    #[tool(
        name = "todo",
        description = "Create a todo item. The folder field is required by default; title is required; optional description/content and tasks[] create checklist items."
    )]
    async fn nb_todo(
        &self,
        Parameters(args): Parameters<TodoArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_todo(args).await
    }

    #[tool(
        name = "list",
        description = "List notes with optional filtering. Todo state is [ ] / [x] in titles; leading glyphs are item markers from nb."
    )]
    async fn nb_list(
        &self,
        Parameters(args): Parameters<ListArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_list(args).await
    }

    #[tool(name = "status", description = "Show current notebook and stats.")]
    async fn nb_status(
        &self,
        Parameters(args): Parameters<StatusArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_status(args).await
    }

    #[tool(
        name = "notebooks",
        description = "List available notebooks (list-only; notebook creation/deletion is not exposed via MCP)."
    )]
    async fn nb_notebooks(&self) -> Result<CallToolResult, McpError> {
        self.dispatch_notebooks().await
    }

    #[tool(
        name = "show",
        description = "Read a note's content as a text-first structured envelope (selector, path, kind, todo_state, title, tags, body, body_contiguous, fingerprint). Non-UTF-8 or non-textual targets return a typed error with recovery guidance; use id (alias: selector) to identify the note."
    )]
    async fn nb_show(
        &self,
        Parameters(args): Parameters<ShowArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_show(args).await
    }

    #[tool(
        name = "replace_note_body",
        description = "Replace the entire body of a note. Requires the body fingerprint from a preceding `show` to prevent stale overwrites. Direct-only tool; no multiplexed alias."
    )]
    async fn nb_replace_note_body(
        &self,
        Parameters(args): Parameters<ReplaceNoteBodyArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_replace_note_body(args).await
    }

    #[tool(
        name = "edit_note_substring",
        description = "Edit a note body by replacing one or more occurrences of a byte pattern. `expected_count` must match the actual match count; an optional fingerprint guards against stale edits. Direct-only tool; no multiplexed alias."
    )]
    async fn nb_edit_note_substring(
        &self,
        Parameters(args): Parameters<EditNoteSubstringArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_edit_note_substring(args).await
    }

    #[tool(
        name = "edit_note_lines",
        description = "Apply a batch of disjoint line edits (insert/delete/replace) to a note body, verified against anchors from one original snapshot. Direct-only tool; no multiplexed alias."
    )]
    async fn nb_edit_note_lines(
        &self,
        Parameters(args): Parameters<EditNoteLinesArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_edit_note_lines(args).await
    }

    #[tool(
        name = "retitle_note",
        description = "Change a note's title without changing its path. Direct-only tool; no multiplexed alias."
    )]
    async fn nb_retitle_note(
        &self,
        Parameters(args): Parameters<RetitleNoteArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_retitle_note(args).await
    }

    #[tool(
        name = "edit_note_tags",
        description = "Add and/or remove tags on a note in one atomic operation. Direct-only tool; no multiplexed alias."
    )]
    async fn nb_edit_note_tags(
        &self,
        Parameters(args): Parameters<EditNoteTagsArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_edit_note_tags(args).await
    }

    #[tool(
        name = "show_note_lines",
        description = "List body lines of a contiguous-body note with anchors and window metadata. Direct-only tool; no multiplexed alias."
    )]
    async fn nb_show_note_lines(
        &self,
        Parameters(args): Parameters<ShowNoteLinesArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_show_note_lines(args).await
    }

    #[tool(
        name = "search_note_lines",
        description = "Search body line texts for a byte pattern and return anchored matching lines. Direct-only tool; no multiplexed alias."
    )]
    async fn nb_search_note_lines(
        &self,
        Parameters(args): Parameters<SearchNoteLinesArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_search_note_lines(args).await
    }

    #[tool(
        name = "delete",
        description = "Delete a note. Use id (alias: selector) to identify the note."
    )]
    async fn nb_delete(
        &self,
        Parameters(args): Parameters<DeleteArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_delete(args).await
    }

    #[tool(
        name = "move",
        description = "Move or rename a note. Use id (alias: selector) to identify the note. destination can be a folder path (ending with /) or a new filename."
    )]
    async fn nb_move(
        &self,
        Parameters(args): Parameters<MoveArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_move(args).await
    }

    #[tool(
        name = "do",
        description = "Mark a todo as complete. Use id (alias: selector) to identify the todo. Optional task_number for checklist items."
    )]
    async fn nb_do(
        &self,
        Parameters(args): Parameters<TaskIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_do(args).await
    }

    #[tool(
        name = "undo",
        description = "Reopen a completed todo. Use id (alias: selector) to identify the todo. Optional task_number for checklist items."
    )]
    async fn nb_undo(
        &self,
        Parameters(args): Parameters<TaskIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_undo(args).await
    }

    #[tool(
        name = "tasks",
        description = "List todo items. Optional status filter (open or closed). recursive defaults to true; set false for folder-only scope."
    )]
    async fn nb_tasks(
        &self,
        Parameters(args): Parameters<TasksArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_tasks(args).await
    }

    #[tool(
        name = "bookmark",
        description = "Save a URL as a bookmark. The folder field is required by default."
    )]
    async fn nb_bookmark(
        &self,
        Parameters(args): Parameters<BookmarkArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_bookmark(args).await
    }

    #[tool(
        name = "folders",
        description = "List folders in notebook. Optional parent to list subfolders."
    )]
    async fn nb_folders(
        &self,
        Parameters(args): Parameters<FoldersArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_folders(args).await
    }

    #[tool(name = "mkdir", description = "Create a folder in the notebook.")]
    async fn nb_mkdir(
        &self,
        Parameters(args): Parameters<MkdirArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_mkdir(args).await
    }

    #[tool(
        name = "import",
        description = "Import a file or URL into notebook. The folder field is required by default."
    )]
    async fn nb_import(
        &self,
        Parameters(args): Parameters<ImportArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_import(args).await
    }
}

#[tool_handler]
impl rmcp::ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "MCP server wrapping nb CLI for LLM-friendly note-taking. \
             Handles markdown escaping and notebook qualification automatically.",
        )
    }
}

pub async fn run(config: Config) -> Result<()> {
    if config.commit_signing_disabled {
        match git_signing::disable_commit_signing(&config).await {
            Ok(Some(path)) => {
                info!(
                    repository = %path.display(),
                    "commit signing disabled for notebook repository"
                );
            }
            Ok(None) => {
                warn!("commit signing disable requested but notebook repository not found");
            }
            Err(err) => {
                warn!(
                    error = %err,
                    "commit signing disable requested but update failed"
                );
            }
        }
    }
    let server = McpServer::new(&config)?;
    info!("starting nb-mcp server");
    if let Some(ref nb) = config.notebook {
        info!(notebook = %nb, "using configured notebook");
    }
    let service = server.serve(stdio()).await?;
    info!("nb-mcp server ready");
    service.waiting().await?;
    Ok(())
}

impl McpServer {
    async fn dispatch_nb(&self, call: NbCall) -> Result<CallToolResult, McpError> {
        macro_rules! parse_or_return {
            ($ty:ty, $value:expr) => {
                match parse_args::<$ty>($value) {
                    Ok(args) => args,
                    Err(message) => return Ok(tool_error(message)),
                }
            };
        }

        let command = call.command.trim();
        if command.is_empty() {
            return Ok(tool_error(
                "Invalid command: command must be non-empty.\n\
                 Hint: call help with query 'nb' to list available commands.",
            ));
        }

        // Strip "nb." prefix if present.
        let subcommand = command.strip_prefix("nb.").unwrap_or(command);

        let result = match subcommand {
            "status" => {
                let args: StatusArgs = parse_or_return!(StatusArgs, call.args);
                text_result(self.nb.show_notebook_status(args.notebook.as_deref()).await)
            }
            "notebooks" => text_result(self.nb.list_notebooks().await),
            "add" => {
                let args: AddArgs = parse_or_return!(AddArgs, call.args);
                outcome_result(
                    self.nb
                        .add_note(
                            args.title.as_deref(),
                            &args.content,
                            &args.tags,
                            args.folder.as_deref(),
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            }
            "show" => {
                let args: ShowArgs = parse_or_return!(ShowArgs, call.args);
                show_result(self.nb.show_note(&args.id, args.notebook.as_deref()).await)
            }
            "edit" => {
                return Ok(tool_error(
                    "Invalid command: edit was removed in the nb-api 0.3 cutover.\n\
                     Hint: use the direct body-aware tools replace_note_body, \
                     edit_note_substring, edit_note_lines, retitle_note, or \
                     edit_note_tags.",
                ));
            }
            "delete" => {
                let args: DeleteArgs = parse_or_return!(DeleteArgs, call.args);
                outcome_result(
                    self.nb
                        .delete_note(&args.id, args.notebook.as_deref())
                        .await,
                )
            }
            "move" => {
                let args: MoveArgs = parse_or_return!(MoveArgs, call.args);
                outcome_result(
                    self.nb
                        .move_note(&args.id, &args.destination, args.notebook.as_deref())
                        .await,
                )
            }
            "list" => {
                let args: ListArgs = parse_or_return!(ListArgs, call.args);
                text_result(
                    self.nb
                        .list_notes(
                            args.folder.as_deref(),
                            &args.tags,
                            args.limit,
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            }
            "search" => {
                let args: SearchArgs = parse_or_return!(SearchArgs, call.args);
                if args.queries.is_empty() {
                    return Ok(tool_error(
                        "Invalid args for search.\n\
                         Reason: queries must be a non-empty array.\n\
                         Hint: pass queries as an array of one or more strings.",
                    ));
                }
                text_result(
                    self.nb
                        .search_notes(
                            &args.queries,
                            args.mode,
                            &args.tags,
                            args.folder.as_deref(),
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            }
            "todo" => {
                let args: TodoArgs = parse_or_return!(TodoArgs, call.args);
                outcome_result(
                    self.nb
                        .add_todo(
                            &args.title,
                            args.description.as_deref(),
                            &args.tasks,
                            &args.tags,
                            args.folder.as_deref(),
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            }
            "do" => {
                let args: TaskIdArgs = parse_or_return!(TaskIdArgs, call.args);
                outcome_result(
                    self.nb
                        .mark_task_done(&args.id, args.task_number, args.notebook.as_deref())
                        .await,
                )
            }
            "undo" => {
                let args: TaskIdArgs = parse_or_return!(TaskIdArgs, call.args);
                outcome_result(
                    self.nb
                        .unmark_task_done(&args.id, args.task_number, args.notebook.as_deref())
                        .await,
                )
            }
            "tasks" => {
                let args: TasksArgs = parse_or_return!(TasksArgs, call.args);
                text_result(
                    self.nb
                        .list_tasks(
                            args.folder.as_deref(),
                            args.status,
                            args.recursive,
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            }
            "bookmark" => {
                let args: BookmarkArgs = parse_or_return!(BookmarkArgs, call.args);
                outcome_result(
                    self.nb
                        .add_bookmark(
                            &args.url,
                            args.title.as_deref(),
                            &args.tags,
                            args.comment.as_deref(),
                            args.folder.as_deref(),
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            }
            "folders" => {
                let args: FoldersArgs = parse_or_return!(FoldersArgs, call.args);
                text_result(
                    self.nb
                        .list_folders(args.parent.as_deref(), args.notebook.as_deref())
                        .await,
                )
            }
            "mkdir" => {
                let args: MkdirArgs = parse_or_return!(MkdirArgs, call.args);
                outcome_result(
                    self.nb
                        .add_folder(&args.path, args.notebook.as_deref())
                        .await,
                )
            }
            "import" => {
                let args: ImportArgs = parse_or_return!(ImportArgs, call.args);
                text_result(
                    self.nb
                        .import_note(
                            &args.source,
                            args.folder.as_deref(),
                            args.filename.as_deref(),
                            args.convert,
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            }
            "replace_note_body"
            | "edit_note_substring"
            | "edit_note_lines"
            | "retitle_note"
            | "edit_note_tags"
            | "show_note_lines"
            | "search_note_lines" => {
                return Ok(tool_error(format!(
                    "Invalid command: {subcommand} is a direct-only tool with no multiplexed alias.\n\
                     Hint: invoke the direct `{subcommand}` tool instead of the multiplexed `nb` tool."
                )));
            }
            _ => {
                return Ok(tool_error(format!(
                    "Unknown subcommand: {command}.\n\
                     Hint: call help with query 'nb' for available commands."
                )));
            }
        };

        Ok(result)
    }

    // First-class dispatch methods.
    // These reuse the same NbClient methods as dispatch_nb.

    async fn dispatch_add(&self, args: AddArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .add_note(
                args.title.as_deref(),
                &args.content,
                &args.tags,
                args.folder.as_deref(),
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_search(&self, args: SearchArgs) -> Result<CallToolResult, McpError> {
        if args.queries.is_empty() {
            return Ok(tool_error(
                "Invalid args for search.\n\
                 Reason: queries must be a non-empty array.\n\
                 Hint: pass queries as an array of one or more strings.",
            ));
        }
        let result = self
            .nb
            .search_notes(
                &args.queries,
                args.mode,
                &args.tags,
                args.folder.as_deref(),
                args.notebook.as_deref(),
            )
            .await;
        Ok(text_result(result))
    }

    async fn dispatch_todo(&self, args: TodoArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .add_todo(
                &args.title,
                args.description.as_deref(),
                &args.tasks,
                &args.tags,
                args.folder.as_deref(),
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_list(&self, args: ListArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .list_notes(
                args.folder.as_deref(),
                &args.tags,
                args.limit,
                args.notebook.as_deref(),
            )
            .await;
        Ok(text_result(result))
    }

    async fn dispatch_status(&self, args: StatusArgs) -> Result<CallToolResult, McpError> {
        let result = self.nb.show_notebook_status(args.notebook.as_deref()).await;
        Ok(text_result(result))
    }

    async fn dispatch_notebooks(&self) -> Result<CallToolResult, McpError> {
        let result = self.nb.list_notebooks().await;
        Ok(text_result(result))
    }

    async fn dispatch_show(&self, args: ShowArgs) -> Result<CallToolResult, McpError> {
        let result = self.nb.show_note(&args.id, args.notebook.as_deref()).await;
        Ok(show_result(result))
    }

    async fn dispatch_replace_note_body(
        &self,
        args: ReplaceNoteBodyArgs,
    ) -> Result<CallToolResult, McpError> {
        let fingerprint = match parse_fingerprint(&args.fingerprint) {
            Ok(fp) => fp,
            Err(err) => return Ok(nb_error_result(err)),
        };
        let target = NoteTarget::selector(args.id);
        let new_body = args.new_body.into_bytes();
        let result = self
            .nb
            .replace_note_body(target, new_body, fingerprint, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_edit_note_substring(
        &self,
        args: EditNoteSubstringArgs,
    ) -> Result<CallToolResult, McpError> {
        let fingerprint = match args.fingerprint.as_deref() {
            Some(value) => match parse_fingerprint(value) {
                Ok(fp) => Some(fp),
                Err(err) => return Ok(nb_error_result(err)),
            },
            None => None,
        };
        let target = NoteTarget::selector(args.id);
        let pattern = args.pattern.into_bytes();
        let replacement = args.replacement.into_bytes();
        let result = self
            .nb
            .edit_note_substring(
                target,
                pattern,
                replacement,
                args.occurrence,
                args.expected_count,
                fingerprint,
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_edit_note_lines(
        &self,
        args: EditNoteLinesArgs,
    ) -> Result<CallToolResult, McpError> {
        let target = NoteTarget::selector(args.id);
        let edits = match convert_mcp_line_edits(args.edits) {
            Ok(edits) => edits,
            Err(err) => return Ok(nb_error_result(err)),
        };
        let result = self
            .nb
            .edit_note_lines(target, edits, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_retitle_note(
        &self,
        args: RetitleNoteArgs,
    ) -> Result<CallToolResult, McpError> {
        let target = NoteTarget::selector(args.id);
        let title = args.title.into_bytes();
        let result = self
            .nb
            .retitle_note(target, title, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_edit_note_tags(
        &self,
        args: EditNoteTagsArgs,
    ) -> Result<CallToolResult, McpError> {
        let target = NoteTarget::selector(args.id);
        let result = self
            .nb
            .edit_note_tags(target, &args.add, &args.remove, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_show_note_lines(
        &self,
        args: ShowNoteLinesArgs,
    ) -> Result<CallToolResult, McpError> {
        let target = NoteTarget::selector(args.id);
        let result = self
            .nb
            .show_note_lines(target, args.offset, args.limit, args.notebook.as_deref())
            .await;
        Ok(show_note_lines_result(result))
    }

    async fn dispatch_search_note_lines(
        &self,
        args: SearchNoteLinesArgs,
    ) -> Result<CallToolResult, McpError> {
        let target = NoteTarget::selector(args.id);
        let pattern = args.pattern.into_bytes();
        let result = self
            .nb
            .search_note_lines(target, &pattern, args.notebook.as_deref())
            .await;
        Ok(search_note_lines_result(result))
    }

    async fn dispatch_delete(&self, args: DeleteArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .delete_note(&args.id, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_move(&self, args: MoveArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .move_note(&args.id, &args.destination, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_do(&self, args: TaskIdArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .mark_task_done(&args.id, args.task_number, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_undo(&self, args: TaskIdArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .unmark_task_done(&args.id, args.task_number, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_tasks(&self, args: TasksArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .list_tasks(
                args.folder.as_deref(),
                args.status,
                args.recursive,
                args.notebook.as_deref(),
            )
            .await;
        Ok(text_result(result))
    }

    async fn dispatch_bookmark(&self, args: BookmarkArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .add_bookmark(
                &args.url,
                args.title.as_deref(),
                &args.tags,
                args.comment.as_deref(),
                args.folder.as_deref(),
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_folders(&self, args: FoldersArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .list_folders(args.parent.as_deref(), args.notebook.as_deref())
            .await;
        Ok(text_result(result))
    }

    async fn dispatch_mkdir(&self, args: MkdirArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .add_folder(&args.path, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    async fn dispatch_import(&self, args: ImportArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .import_note(
                &args.source,
                args.folder.as_deref(),
                args.filename.as_deref(),
                args.convert,
                args.notebook.as_deref(),
            )
            .await;
        Ok(text_result(result))
    }
}

fn parse_args<T: serde::de::DeserializeOwned + Default>(
    value: serde_json::Value,
) -> Result<T, String> {
    if value.is_null() {
        return Ok(T::default());
    }
    let value = match value {
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return Ok(T::default());
            }
            serde_json::Value::Object(map)
        }
        other => {
            return Err(format!(
                "Invalid args for command.\n\
                 Reason: args must be a JSON object, got {}.\n\
                 Hint: pass args as a JSON object (not a stringified JSON payload).",
                json_type_name(&other)
            ));
        }
    };

    serde_json::from_value::<T>(value).map_err(|err| {
        format!(
            "Invalid args for command.\n\
             Reason: {}.\n\
             Hint: check required fields with help query 'nb.<command>'.",
            err
        )
    })
}

fn parse_fingerprint(value: &str) -> Result<Fingerprint, NbError> {
    value.parse()
}

fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(message.into())])
}

/// Present a `Result<String, NbError>` (reads) as a text tool result.
fn text_result(result: Result<String, NbError>) -> CallToolResult {
    match result {
        Ok(output) => CallToolResult::success(vec![Content::text(output)]),
        Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
    }
}

/// Present a `Result<CommitOutcome, NbError>` as a structured JSON tool result.
fn outcome_result(result: Result<CommitOutcome, NbError>) -> CallToolResult {
    match result {
        Ok(outcome) => json_success(&outcome),
        Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
    }
}

/// Present a `Result<ShowNote, NbError>` as the structured envelope.
fn show_result(result: Result<ShowNote, NbError>) -> CallToolResult {
    match result {
        Ok(shown) => match ShowEnvelope::from_show(shown) {
            Ok(envelope) => json_success(&envelope),
            Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
        },
        Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
    }
}

/// Present a serializable typed result (lines/search lines) as JSON.
#[allow(dead_code)]
fn typed_result<T: Serialize>(result: Result<T, NbError>) -> CallToolResult {
    match result {
        Ok(value) => json_success(&value),
        Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
    }
}

/// Present an `Err(NbError)` as a structured error result.
fn nb_error_result(err: NbError) -> CallToolResult {
    CallToolResult::error(vec![Content::text(present_nb_error(&err))])
}

fn json_success(value: &impl Serialize) -> CallToolResult {
    match serde_json::to_value(value) {
        Ok(json) => CallToolResult::success(vec![
            Content::json(json)
                .unwrap_or_else(|_| Content::text("failed to serialize result".to_string())),
        ]),
        Err(_) => CallToolResult::success(vec![Content::text(
            "failed to serialize result".to_string(),
        )]),
    }
}

fn present_nb_error(err: &NbError) -> String {
    match err {
        NbError::UnsupportedShowTarget {
            selector,
            actual_type,
        } => format!(
            "Invalid args for show.\n\
             Reason: selector `{selector}` resolved to a non-textual type \
             ({actual_type}); `show` reads text notes only.\n\
             Hint: for folders use `folders` or `list`; for other non-text \
             types there is no MCP read operation in this release."
        ),
        NbError::DuplicateTitleHeading { title, heading } => format!(
            "Invalid args for add.\n\
             Reason: title `{title}` duplicates the first H1 heading in content \
             (`{heading}`); both produce a top-level title and double-render.\n\
             Hint: remove the duplicate H1 from content, or omit the separate \
             `title` field."
        ),
        NbError::FingerprintMismatch { target, .. } => format!(
            "Stale edit rejected: body fingerprint does not match the current \
             note `{}`.\n\
             Hint: re-read the note with `show` and retry with the fresh \
             fingerprint.",
            target.value()
        ),
        NbError::AnchorMismatch {
            target,
            number,
            guidance,
        } => format!(
            "Stale edit rejected for `{target}` line {number}: {guidance}.\n\
             Hint: re-read the note lines with `show_note_lines` and retry with \
             fresh anchors."
        ),
        NbError::OccurrenceMismatch { expected, actual } => format!(
            "Substring edit rejected: expected {expected} match(es) but found \
             {actual}.\n\
             Hint: re-read the note and correct expected_count."
        ),
        NbError::OverlappingEdits { indices } => format!(
            "Line edit batch rejected: edits at indices {indices:?} overlap.\n\
             Hint: provide disjoint edits against one original snapshot."
        ),
        NbError::InvalidLineWindow {
            offset,
            limit,
            total_lines,
        } => format!(
            "Invalid line window: offset={offset} limit={limit} but note has \
             {total_lines} line(s).\n\
             Hint: re-read with `show_note_lines` and use valid bounds."
        ),
        NbError::EmptySubstringPattern => {
            "Substring edit rejected: the pattern must be non-empty.\n\
             Hint: provide a non-empty pattern."
                .to_string()
        }
        NbError::FragmentedBody {
            fragment_count,
            guidance,
        } => format!(
            "Body operation rejected: the note has {fragment_count} body \
             fragment(s), so line/substring/body-replace operations are not \
             available. {guidance}\n\
             Hint: metadata operations (`retitle_note`, `edit_note_tags`) still \
             apply."
        ),
        NbError::DirtyBaseline { guidance } => format!(
            "Mutation rejected because the notebook worktree/index is dirty.\n\
             Hint: commit or clean the notebook worktree/index before retrying. \
             ({guidance})"
        ),
        NbError::IndeterminateCommit {
            pre_revision,
            post_revision_observed,
            guidance,
        } => format!(
            "Commit completion is unknown (pre={pre_revision}, \
             post={post_revision_observed:?}).\n\
             Hint: do not auto-retry the same plan blindly; inspect HEAD and \
             notebook status first. ({guidance})"
        ),
        NbError::RecoveryRequired {
            pre_revision,
            post_revision_observed,
            status_observed,
            preserved_paths,
            guidance,
        } => format!(
            "Recovery required after a failed commit (pre={pre_revision}, \
             post={post_revision_observed:?}, preserved={preserved_paths:?}).\n\
             Hint: inspect HEAD and status before any retry. \
             status={status_observed:?} ({guidance})"
        ),
        NbError::GateTimeout { gate, timeout_ms } => format!(
            "The notebook `{gate}` is busy; the operation timed out after \
             {timeout_ms}ms.\n\
             Hint: retry later; concurrent notebook operations are serialized."
        ),
        NbError::PathCollision { path, plan_index } => format!(
            "Path collision at `{path}` (plan op {plan_index:?}).\n\
             Hint: choose a different path or resolve the existing item first."
        ),
        NbError::PathIgnored {
            path,
            guidance,
            plan_index,
        } => format!(
            "Path `{path}` is Git-ignored (plan op {plan_index:?}); the \
             transaction cannot represent it.\n\
             Hint: {guidance}"
        ),
        NbError::PlanValidation {
            kind,
            message,
            plan_index,
        } => format!("Plan validation ({kind}) failed at op {plan_index:?}: {message}"),
        NbError::UnsupportedStructure { reason } => {
            format!("Unsupported notebook structure: {reason}")
        }
        NbError::InvalidFingerprint { reason } => format!(
            "Invalid fingerprint: {reason}.\n\
             Hint: a fingerprint is `b3:` followed by 64 lowercase hex digits, \
             obtained from `show`."
        ),
        other => other.to_string(),
    }
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn help_tool(params: HelpParams) -> Result<CallToolResult, McpError> {
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
                {"command": "nb.add", "description": "Create a new note (folder required by default)"},
                {"command": "nb.show", "description": "Read a note's content (structured envelope)"},
                {"command": "nb.delete", "description": "Delete a note"},
                {"command": "nb.move", "description": "Move or rename a note"},
                {"command": "nb.list", "description": "List notes with optional filtering (todo state is [ ] / [x], not leading glyph icons)"},
                {"command": "nb.search", "description": "Full-text search notes (queries[] + mode any|all)"},
                {"command": "nb.todo", "description": "Create a todo item (folder required by default; title required; optional description/content and tasks[] checklist)"},
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
                {"tool": "add", "description": "Create a new note (folder required by default)"},
                {"tool": "show", "description": "Read a note's content (structured envelope)"},
                {"tool": "delete", "description": "Delete a note"},
                {"tool": "move", "description": "Move or rename a note"},
                {"tool": "list", "description": "List notes with optional filtering"},
                {"tool": "search", "description": "Full-text search notes (queries[] + mode any|all)"},
                {"tool": "todo", "description": "Create a todo item (folder required by default)"},
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
            "Create a new note. The folder field is required by default; use nb.mkdir to create folders and nb.folders to list them.",
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
            "Create a todo item. The folder field is required by default; title is required; optional description/content and tasks[] create checklist items.",
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
            "Create a new note. The folder field is required by default; use nb.mkdir to create folders and nb.folders to list them.",
            json_schema_for::<AddArgs>(),
        ),
        "show" => first_class_help(
            "show",
            "Read a note's content as a text-first structured envelope (selector, path, kind, todo_state, title, tags, body, body_contiguous, fingerprint).",
            json_schema_for::<ShowArgs>(),
        ),
        "delete" => first_class_help("delete", "Delete a note.", json_schema_for::<DeleteArgs>()),
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
            "Create a todo item. The folder field is required by default; title is required.",
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
        "mkdir" => first_class_help("mkdir", "Create a folder.", json_schema_for::<MkdirArgs>()),
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
        }
    };

    Ok(CallToolResult::success(vec![Content::json(response)?]))
}

fn command_help(command: &str, description: &str, schema: serde_json::Value) -> serde_json::Value {
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

fn first_class_help(tool: &str, description: &str, schema: serde_json::Value) -> serde_json::Value {
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
    serde_json::to_value(schemars::schema_for!(T)).unwrap_or(serde_json::Value::Null)
}
