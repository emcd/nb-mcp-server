use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::{info, warn};

use crate::Config;
use crate::git_signing;
use crate::nb::{EditMode, NbClient, SearchMode, TaskStatus};

#[derive(Clone)]
struct McpServer {
    nb: NbClient,
}

/// Parameters for the nb meta-tool.
#[derive(Debug, Deserialize, JsonSchema)]
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
struct HelpParams {
    /// Namespace or command to get help for (e.g., "nb" or "nb.add").
    query: String,
}

// Command-specific argument structs

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct StatusArgs {
    /// Notebook to check status for (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct AddArgs {
    /// Title for the note.
    title: Option<String>,
    /// Content of the note. Markdown is supported.
    content: String,
    /// Tags to apply (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Folder to create the note in.
    folder: Option<String>,
    /// Notebook to add to (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct ShowArgs {
    /// Note ID, filename, or title to show.
    #[serde(alias = "selector")]
    id: String,
    /// Notebook to read from (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct EditArgs {
    /// Note ID, filename, or title to edit.
    #[serde(alias = "selector")]
    id: String,
    /// New content for the note.
    content: String,
    /// Edit mode: `replace` (default), `append`, or `prepend`.
    #[serde(default)]
    mode: EditMode,
    /// Notebook containing the note (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct DeleteArgs {
    /// Note ID, filename, or title to delete.
    #[serde(alias = "selector")]
    id: String,
    /// Notebook containing the note (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct MoveArgs {
    /// Note ID, filename, or title to move/rename.
    #[serde(alias = "selector")]
    id: String,
    /// Destination path or new name. Can be a folder path (ending with /) or a new filename.
    /// Examples: "new-folder/" (move to folder), "new-name.md" (rename), "folder/new-name.md" (move and rename).
    destination: String,
    /// Notebook containing the note (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct ListArgs {
    /// Folder to list (lists root if not specified).
    folder: Option<String>,
    /// Filter by tags (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Maximum number of items to return.
    limit: Option<u32>,
    /// Notebook to list from (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct SearchArgs {
    /// Search terms/patterns (supports regex). Provide one or more terms.
    queries: Vec<String>,
    /// Search mode: `any` (default, OR) or `all` (AND).
    #[serde(default)]
    mode: SearchMode,
    /// Filter by tags (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Folder to search within (searches all if not specified).
    folder: Option<String>,
    /// Notebook to search in (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct TodoArgs {
    /// Description of the todo item.
    #[serde(alias = "title")]
    description: String,
    /// Optional checklist task titles to add to the todo.
    #[serde(default)]
    tasks: Vec<String>,
    /// Tags to apply (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Folder to create the todo in.
    folder: Option<String>,
    /// Notebook to add todo to (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct TaskIdArgs {
    /// Todo ID to mark as done/undone.
    #[serde(alias = "selector")]
    id: String,
    /// Optional task number within a todo item.
    #[serde(alias = "task")]
    task_number: Option<u32>,
    /// Notebook containing the todo (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TasksArgs {
    /// Folder to list todos from (lists all if not specified).
    folder: Option<String>,
    /// Optional status filter (`open` or `closed`).
    #[serde(alias = "state")]
    status: Option<TaskStatus>,
    /// Whether to include tasks from descendant folders.
    /// Defaults to true; set false for folder-only scope.
    #[serde(default = "default_true", alias = "recurse")]
    recursive: bool,
    /// Notebook to list todos from (uses default if not specified).
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
struct BookmarkArgs {
    /// URL to bookmark.
    url: String,
    /// Title for the bookmark.
    title: Option<String>,
    /// Tags to apply (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Comment or description.
    comment: Option<String>,
    /// Folder to create the bookmark in.
    folder: Option<String>,
    /// Notebook to add bookmark to (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct FoldersArgs {
    /// Parent folder to list (lists root if not specified).
    #[serde(alias = "folder")]
    parent: Option<String>,
    /// Notebook to list folders from (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct MkdirArgs {
    /// Path of folder to create.
    #[serde(alias = "folder")]
    path: String,
    /// Notebook to create folder in (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct ImportArgs {
    /// File path or URL to import.
    source: String,
    /// Folder to import into (imports to root if not specified).
    folder: Option<String>,
    /// Filename to use in notebook (uses original name if not specified).
    filename: Option<String>,
    /// Convert HTML content to Markdown.
    #[serde(default)]
    convert: bool,
    /// Notebook to import into (uses default if not specified).
    notebook: Option<String>,
}

#[tool_router]
impl McpServer {
    fn new(config: &Config) -> Result<Self> {
        let nb = NbClient::new(
            config.notebook.as_deref(),
            config.create_notebook,
            config.commit_signing_disabled,
        )?;
        Ok(Self { nb })
    }

    #[tool(
        description = "nb note-taking tool. Invoke with {\"command\":\"nb.<subcommand>\",\"args\":{...}} and pass args as a JSON object (stringified JSON is not accepted). This is a curated wrapper (not a 1:1 map of nb CLI flags). Note-targeting commands accept id (alias: selector). In nb.list output, todo state is [ ] / [x]; leading glyphs like ✔️ are item markers from nb, not completion status. nb.search uses queries[] with optional mode any|all. Commands: status, add, show, edit, delete, list, search, todo, do, undo, tasks, bookmark, folders, mkdir, notebooks, import. Use `help` for exact schemas."
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
                self.nb.status(args.notebook.as_deref()).await
            }
            "notebooks" => self.nb.notebooks().await,
            "add" => {
                let args: AddArgs = parse_or_return!(AddArgs, call.args);
                self.nb
                    .add(
                        args.title.as_deref(),
                        &args.content,
                        &args.tags,
                        args.folder.as_deref(),
                        args.notebook.as_deref(),
                    )
                    .await
            }
            "show" => {
                let args: ShowArgs = parse_or_return!(ShowArgs, call.args);
                self.nb.show(&args.id, args.notebook.as_deref()).await
            }
            "edit" => {
                let args: EditArgs = parse_or_return!(EditArgs, call.args);
                self.nb
                    .edit(&args.id, &args.content, args.mode, args.notebook.as_deref())
                    .await
            }
            "delete" => {
                let args: DeleteArgs = parse_or_return!(DeleteArgs, call.args);
                self.nb.delete(&args.id, args.notebook.as_deref()).await
            }
            "move" => {
                let args: MoveArgs = parse_or_return!(MoveArgs, call.args);
                self.nb
                    .move_note(&args.id, &args.destination, args.notebook.as_deref())
                    .await
            }
            "list" => {
                let args: ListArgs = parse_or_return!(ListArgs, call.args);
                self.nb
                    .list(
                        args.folder.as_deref(),
                        &args.tags,
                        args.limit,
                        args.notebook.as_deref(),
                    )
                    .await
            }
            "search" => {
                let args: SearchArgs = parse_or_return!(SearchArgs, call.args);
                if args.queries.is_empty() {
                    return Ok(tool_error(
                        "Invalid args for nb.search.\n\
                         Reason: queries must be a non-empty array.\n\
                         Hint: pass args.queries as an array of one or more strings.",
                    ));
                }
                self.nb
                    .search(
                        &args.queries,
                        args.mode,
                        &args.tags,
                        args.folder.as_deref(),
                        args.notebook.as_deref(),
                    )
                    .await
            }
            "todo" => {
                let args: TodoArgs = parse_or_return!(TodoArgs, call.args);
                self.nb
                    .todo(
                        &args.description,
                        &args.tasks,
                        &args.tags,
                        args.folder.as_deref(),
                        args.notebook.as_deref(),
                    )
                    .await
            }
            "do" => {
                let args: TaskIdArgs = parse_or_return!(TaskIdArgs, call.args);
                self.nb
                    .do_task(&args.id, args.task_number, args.notebook.as_deref())
                    .await
            }
            "undo" => {
                let args: TaskIdArgs = parse_or_return!(TaskIdArgs, call.args);
                self.nb
                    .undo_task(&args.id, args.task_number, args.notebook.as_deref())
                    .await
            }
            "tasks" => {
                let args: TasksArgs = parse_or_return!(TasksArgs, call.args);
                self.nb
                    .tasks(
                        args.folder.as_deref(),
                        args.status,
                        args.recursive,
                        args.notebook.as_deref(),
                    )
                    .await
            }
            "bookmark" => {
                let args: BookmarkArgs = parse_or_return!(BookmarkArgs, call.args);
                self.nb
                    .bookmark(
                        &args.url,
                        args.title.as_deref(),
                        &args.tags,
                        args.comment.as_deref(),
                        args.folder.as_deref(),
                        args.notebook.as_deref(),
                    )
                    .await
            }
            "folders" => {
                let args: FoldersArgs = parse_or_return!(FoldersArgs, call.args);
                self.nb
                    .folders(args.parent.as_deref(), args.notebook.as_deref())
                    .await
            }
            "mkdir" => {
                let args: MkdirArgs = parse_or_return!(MkdirArgs, call.args);
                self.nb.mkdir(&args.path, args.notebook.as_deref()).await
            }
            "import" => {
                let args: ImportArgs = parse_or_return!(ImportArgs, call.args);
                self.nb
                    .import(
                        &args.source,
                        args.folder.as_deref(),
                        args.filename.as_deref(),
                        args.convert,
                        args.notebook.as_deref(),
                    )
                    .await
            }
            _ => {
                return Ok(tool_error(format!(
                    "Unknown subcommand: {command}.\n\
                     Hint: call help with query 'nb' for available commands."
                )));
            }
        };

        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
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

fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(message.into())])
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
                "Common fields: id (alias selector), folder path, tags array, optional notebook override, plus task_number/status for todo commands.",
                "nb.search takes queries[] (required, non-empty), plus mode: any (default OR) or all (AND).",
                "nb.todo supports optional tasks[] to create checklist items via repeated --task flags.",
                "Compatibility aliases: note commands selector->id, nb.todo title->description, nb.folders folder->parent, nb.mkdir folder->path.",
                "nb.tasks is recursive by default; pass recursive:false to limit to the selected folder.",
                "In nb.list output, todo state comes from [ ] / [x] in titles; leading glyphs like ✔️ are item-type markers from nb.",
                "Call help with query 'nb.<command>' for exact command schemas."
            ],
            "commands": [
                {"command": "nb.status", "description": "Show current notebook and stats"},
                {"command": "nb.notebooks", "description": "List available notebooks (list-only; no add/delete in MCP)"},
                {"command": "nb.add", "description": "Create a new note"},
                {"command": "nb.show", "description": "Read a note's content"},
                {"command": "nb.edit", "description": "Update a note's content (replace by default)"},
                {"command": "nb.delete", "description": "Delete a note"},
                {"command": "nb.move", "description": "Move or rename a note"},
                {"command": "nb.list", "description": "List notes with optional filtering (todo state is [ ] / [x], not leading glyph icons)"},
                {"command": "nb.search", "description": "Full-text search notes (queries[] + mode any|all)"},
                {"command": "nb.todo", "description": "Create a todo item (optional tasks[] checklist)"},
                {"command": "nb.do", "description": "Mark a todo as complete (optional task_number)"},
                {"command": "nb.undo", "description": "Reopen a completed todo (optional task_number)"},
                {"command": "nb.tasks", "description": "List todo items recursively (optional status: open|closed)"},
                {"command": "nb.bookmark", "description": "Save a URL as a bookmark"},
                {"command": "nb.folders", "description": "List folders in notebook"},
                {"command": "nb.mkdir", "description": "Create a folder"},
                {"command": "nb.import", "description": "Import a file or URL into notebook"},
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
        "nb.add" => command_help("nb.add", "Create a new note", json_schema_for::<AddArgs>()),
        "nb.show" => command_help(
            "nb.show",
            "Read a note's content",
            json_schema_for::<ShowArgs>(),
        ),
        "nb.edit" => command_help(
            "nb.edit",
            "Update a note's content (replace by default)",
            json_schema_for::<EditArgs>(),
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
            "Create a todo item (optional tasks[] checklist)",
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
            "Save a URL as a bookmark",
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
            "Import a file or URL into notebook",
            json_schema_for::<ImportArgs>(),
        ),
        "nb.notebooks" => command_help(
            "nb.notebooks",
            "List available notebooks (list-only; notebook creation/deletion is not exposed via MCP)",
            serde_json::json!({"type": "object", "properties": {}}),
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

fn json_schema_for<T: schemars::JsonSchema>() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(T)).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::parse_args;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Debug, Default, Deserialize, PartialEq, Eq)]
    struct ExampleArgs {
        #[serde(default)]
        value: String,
    }

    #[test]
    fn parse_args_accepts_object_payloads() {
        let parsed = parse_args::<ExampleArgs>(json!({"value": "ok"})).unwrap();
        assert_eq!(
            parsed,
            ExampleArgs {
                value: "ok".to_string()
            }
        );
    }

    #[test]
    fn parse_args_defaults_from_null_or_empty_object() {
        assert_eq!(
            parse_args::<ExampleArgs>(serde_json::Value::Null).unwrap(),
            ExampleArgs::default()
        );
        assert_eq!(
            parse_args::<ExampleArgs>(json!({})).unwrap(),
            ExampleArgs::default()
        );
    }

    #[test]
    fn parse_args_rejects_non_object_payloads() {
        assert!(parse_args::<ExampleArgs>(json!("{\"value\":\"ok\"}")).is_err());
        assert!(parse_args::<ExampleArgs>(json!(["ok"])).is_err());
        assert!(parse_args::<ExampleArgs>(json!(42)).is_err());
        assert!(parse_args::<ExampleArgs>(json!(true)).is_err());
    }
}
