use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServiceExt,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::info;

use crate::Config;
use crate::nb::NbClient;

#[derive(Clone)]
struct McpServer {
    nb: NbClient,
    tool_router: ToolRouter<Self>,
}

/// Parameters for the nb meta-tool.
#[derive(Debug, Deserialize, JsonSchema)]
struct NbCall {
    /// Subcommand to execute (e.g., "status", "add", "list").
    command: String,
    /// Arguments for the subcommand as a JSON object.
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
    id: String,
    /// Notebook to read from (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct EditArgs {
    /// Note ID, filename, or title to edit.
    id: String,
    /// New content for the note (replaces existing content).
    content: String,
    /// Notebook containing the note (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct DeleteArgs {
    /// Note ID, filename, or title to delete.
    id: String,
    /// Must be true to confirm deletion.
    #[serde(default)]
    confirm: bool,
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
    /// Search query (supports regex).
    query: String,
    /// Filter by tags (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Notebook to search in (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct TodoArgs {
    /// Description of the todo item.
    description: String,
    /// Tags to apply (without # prefix).
    #[serde(default)]
    tags: Vec<String>,
    /// Notebook to add todo to (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct TaskIdArgs {
    /// Todo ID to mark as done/undone.
    id: String,
    /// Notebook containing the todo (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct TasksArgs {
    /// Notebook to list todos from (uses default if not specified).
    notebook: Option<String>,
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
    /// Notebook to add bookmark to (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct FoldersArgs {
    /// Parent folder to list (lists root if not specified).
    parent: Option<String>,
    /// Notebook to list folders from (uses default if not specified).
    notebook: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct MkdirArgs {
    /// Path of folder to create.
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
        let nb = NbClient::new(config.notebook.as_deref())?;
        Ok(Self {
            nb,
            tool_router: Self::tool_router(),
        })
    }

    #[tool(
        description = "nb note-taking tool. Commands: status, add, show, edit, delete, list, search, todo, do, undo, tasks, bookmark, folders, mkdir, notebooks, import. Use `help` for schemas."
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
        ServerInfo {
            instructions: Some(
                "MCP server wrapping nb CLI for LLM-friendly note-taking. \
                 Handles markdown escaping and notebook qualification automatically."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

pub async fn run(config: Config) -> Result<()> {
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
        let command = call.command.trim();
        if command.is_empty() {
            return Err(McpError::invalid_params("command must be non-empty", None));
        }

        // Strip "nb." prefix if present.
        let subcommand = command.strip_prefix("nb.").unwrap_or(command);

        let result = match subcommand {
            "status" => {
                let args: StatusArgs = parse_args(call.args)?;
                self.nb.status(args.notebook.as_deref()).await
            }
            "notebooks" => self.nb.notebooks().await,
            "add" => {
                let args: AddArgs = parse_args(call.args)?;
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
                let args: ShowArgs = parse_args(call.args)?;
                self.nb.show(&args.id, args.notebook.as_deref()).await
            }
            "edit" => {
                let args: EditArgs = parse_args(call.args)?;
                self.nb
                    .edit(&args.id, &args.content, args.notebook.as_deref())
                    .await
            }
            "delete" => {
                let args: DeleteArgs = parse_args(call.args)?;
                if !args.confirm {
                    return Err(McpError::invalid_params(
                        "delete requires confirm: true",
                        Some(serde_json::json!({
                            "hint": "Set confirm: true to delete the note.",
                            "id": args.id,
                        })),
                    ));
                }
                self.nb.delete(&args.id, args.notebook.as_deref()).await
            }
            "list" => {
                let args: ListArgs = parse_args(call.args)?;
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
                let args: SearchArgs = parse_args(call.args)?;
                self.nb
                    .search(&args.query, &args.tags, args.notebook.as_deref())
                    .await
            }
            "todo" => {
                let args: TodoArgs = parse_args(call.args)?;
                self.nb
                    .todo(&args.description, &args.tags, args.notebook.as_deref())
                    .await
            }
            "do" => {
                let args: TaskIdArgs = parse_args(call.args)?;
                self.nb.do_task(&args.id, args.notebook.as_deref()).await
            }
            "undo" => {
                let args: TaskIdArgs = parse_args(call.args)?;
                self.nb.undo_task(&args.id, args.notebook.as_deref()).await
            }
            "tasks" => {
                let args: TasksArgs = parse_args(call.args)?;
                self.nb.tasks(args.notebook.as_deref()).await
            }
            "bookmark" => {
                let args: BookmarkArgs = parse_args(call.args)?;
                self.nb
                    .bookmark(
                        &args.url,
                        args.title.as_deref(),
                        &args.tags,
                        args.comment.as_deref(),
                        args.notebook.as_deref(),
                    )
                    .await
            }
            "folders" => {
                let args: FoldersArgs = parse_args(call.args)?;
                self.nb
                    .folders(args.parent.as_deref(), args.notebook.as_deref())
                    .await
            }
            "mkdir" => {
                let args: MkdirArgs = parse_args(call.args)?;
                self.nb.mkdir(&args.path, args.notebook.as_deref()).await
            }
            "import" => {
                let args: ImportArgs = parse_args(call.args)?;
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
                return Err(McpError::invalid_params(
                    "unknown subcommand",
                    Some(serde_json::json!({
                        "command": command,
                        "hint": "Call `help` with query 'nb' for available commands.",
                    })),
                ));
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
) -> Result<T, McpError> {
    // Handle empty/null args by using defaults
    if value.is_null() || (value.is_object() && value.as_object().unwrap().is_empty()) {
        return Ok(T::default());
    }

    // Handle string-encoded JSON (some clients send args as string)
    let value = match value {
        serde_json::Value::String(raw) => serde_json::from_str(&raw).map_err(|err| {
            McpError::invalid_params(
                "invalid args for command",
                Some(serde_json::json!({
                    "error": format!("args was a string but did not parse as JSON: {}", err),
                    "hint": "Pass args as a JSON object.",
                })),
            )
        })?,
        other => other,
    };

    serde_json::from_value::<T>(value).map_err(|err| {
        McpError::invalid_params(
            "invalid args for command",
            Some(serde_json::json!({
                "error": err.to_string(),
                "hint": "Check the required fields using the help tool.",
            })),
        )
    })
}

fn help_tool(params: HelpParams) -> Result<CallToolResult, McpError> {
    let query = params.query.trim();

    let response = match query {
        "nb" => serde_json::json!({
            "namespace": "nb",
            "commands": [
                {"command": "nb.status", "description": "Show current notebook and stats"},
                {"command": "nb.notebooks", "description": "List available notebooks"},
                {"command": "nb.add", "description": "Create a new note"},
                {"command": "nb.show", "description": "Read a note's content"},
                {"command": "nb.edit", "description": "Update a note's content"},
                {"command": "nb.delete", "description": "Delete a note (requires confirm: true)"},
                {"command": "nb.list", "description": "List notes with optional filtering"},
                {"command": "nb.search", "description": "Full-text search notes"},
                {"command": "nb.todo", "description": "Create a todo item"},
                {"command": "nb.do", "description": "Mark a todo as complete"},
                {"command": "nb.undo", "description": "Reopen a completed todo"},
                {"command": "nb.tasks", "description": "List todo items"},
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
            "Update a note's content",
            json_schema_for::<EditArgs>(),
        ),
        "nb.delete" => command_help(
            "nb.delete",
            "Delete a note (requires confirm: true)",
            json_schema_for::<DeleteArgs>(),
        ),
        "nb.list" => command_help(
            "nb.list",
            "List notes with optional filtering",
            json_schema_for::<ListArgs>(),
        ),
        "nb.search" => command_help(
            "nb.search",
            "Full-text search notes",
            json_schema_for::<SearchArgs>(),
        ),
        "nb.todo" => command_help(
            "nb.todo",
            "Create a todo item",
            json_schema_for::<TodoArgs>(),
        ),
        "nb.do" => command_help(
            "nb.do",
            "Mark a todo as complete",
            json_schema_for::<TaskIdArgs>(),
        ),
        "nb.undo" => command_help(
            "nb.undo",
            "Reopen a completed todo",
            json_schema_for::<TaskIdArgs>(),
        ),
        "nb.tasks" => command_help(
            "nb.tasks",
            "List todo items",
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
            "List available notebooks",
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
