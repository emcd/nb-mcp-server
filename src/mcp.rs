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

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EditArgs {
    /// Notebook selector, note ID, filename, or title to edit; not a filesystem path.
    #[serde(alias = "selector")]
    id: String,
    /// New content for the note.
    content: String,
    /// Edit mode: `overwrite` (replaces every byte of the note body), `append`, or `prepend`.
    mode: EditMode,
    /// Bare notebook name containing the note (uses default if not specified).
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

#[tool_router]
impl McpServer {
    fn new(config: &Config) -> Result<Self> {
        let nb_config = config.to_nb_api_config();
        let nb = NbClient::new(&nb_config)?;
        Ok(Self { nb })
    }

    #[tool(
        description = r#"nb note-taking tool. Invoke with {"command":"nb.<subcommand>","args":{...}} and pass args as a JSON object (stringified JSON is not accepted). Unknown args are rejected. This is a curated wrapper (not a 1:1 map of nb CLI flags). New note commands require folder by default; use nb.mkdir to create folders and nb.folders to list them. notebook must be a bare notebook name; use folder for folders and id/selector for notes. Note-targeting commands accept id (alias: selector); returned identifiers are nb selectors, not repo filesystem paths. In nb.list output, todo state is [ ] / [x]; leading glyphs like ✔️ are item markers from nb, not completion status. nb.search uses queries[] with optional mode any|all. Commands: status, add, show, edit, delete, move, list, search, todo, do, undo, tasks, bookmark, folders, mkdir, notebooks, import. Use `help` for exact schemas."#
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
        description = "Read a note's content. Use id (alias: selector) to identify the note."
    )]
    async fn nb_show(
        &self,
        Parameters(args): Parameters<ShowArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.dispatch_show(args).await
    }

    #[tool(
        name = "edit",
        description = "Update a note's content. Use id (alias: selector) to identify the note. mode is required: overwrite (replaces every byte of the note body), append, or prepend.",
        input_schema = ::std::sync::Arc::new(
            json_schema_for::<EditArgs>()
                .as_object()
                .cloned()
                .unwrap_or_default(),
        ),
    )]
    async fn nb_edit(
        &self,
        Parameters(args): Parameters<serde_json::Value>,
    ) -> Result<CallToolResult, McpError> {
        let args = match parse_edit_args(args) {
            Ok(args) => args,
            Err(message) => return Ok(tool_error(message)),
        };
        self.dispatch_edit(args).await
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
                self.nb.show_notebook_status(args.notebook.as_deref()).await
            }
            "notebooks" => self.nb.list_notebooks().await,
            "add" => {
                let args: AddArgs = parse_or_return!(AddArgs, call.args);
                self.nb
                    .add_note(
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
                self.nb.show_note(&args.id, args.notebook.as_deref()).await
            }
            "edit" => {
                let args: EditArgs = match parse_edit_args(call.args) {
                    Ok(args) => args,
                    Err(message) => return Ok(tool_error(message)),
                };
                self.nb
                    .edit_note(&args.id, &args.content, args.mode, args.notebook.as_deref())
                    .await
            }
            "delete" => {
                let args: DeleteArgs = parse_or_return!(DeleteArgs, call.args);
                self.nb
                    .delete_note(&args.id, args.notebook.as_deref())
                    .await
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
                    .list_notes(
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
                        "Invalid args for search.\n\
                         Reason: queries must be a non-empty array.\n\
                         Hint: pass queries as an array of one or more strings.",
                    ));
                }
                self.nb
                    .search_notes(
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
                    .add_todo(
                        &args.title,
                        args.description.as_deref(),
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
                    .mark_task_done(&args.id, args.task_number, args.notebook.as_deref())
                    .await
            }
            "undo" => {
                let args: TaskIdArgs = parse_or_return!(TaskIdArgs, call.args);
                self.nb
                    .unmark_task_done(&args.id, args.task_number, args.notebook.as_deref())
                    .await
            }
            "tasks" => {
                let args: TasksArgs = parse_or_return!(TasksArgs, call.args);
                self.nb
                    .list_tasks(
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
                    .add_bookmark(
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
                    .list_folders(args.parent.as_deref(), args.notebook.as_deref())
                    .await
            }
            "mkdir" => {
                let args: MkdirArgs = parse_or_return!(MkdirArgs, call.args);
                self.nb
                    .add_folder(&args.path, args.notebook.as_deref())
                    .await
            }
            "import" => {
                let args: ImportArgs = parse_or_return!(ImportArgs, call.args);
                self.nb
                    .import_note(
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
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
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
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
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
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
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
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }

    async fn dispatch_status(&self, args: StatusArgs) -> Result<CallToolResult, McpError> {
        let result = self.nb.show_notebook_status(args.notebook.as_deref()).await;
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }

    async fn dispatch_notebooks(&self) -> Result<CallToolResult, McpError> {
        let result = self.nb.list_notebooks().await;
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }

    async fn dispatch_show(&self, args: ShowArgs) -> Result<CallToolResult, McpError> {
        let result = self.nb.show_note(&args.id, args.notebook.as_deref()).await;
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }

    async fn dispatch_edit(&self, args: EditArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .edit_note(&args.id, &args.content, args.mode, args.notebook.as_deref())
            .await;
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }

    async fn dispatch_delete(&self, args: DeleteArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .delete_note(&args.id, args.notebook.as_deref())
            .await;
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }

    async fn dispatch_move(&self, args: MoveArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .move_note(&args.id, &args.destination, args.notebook.as_deref())
            .await;
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }

    async fn dispatch_do(&self, args: TaskIdArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .mark_task_done(&args.id, args.task_number, args.notebook.as_deref())
            .await;
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }

    async fn dispatch_undo(&self, args: TaskIdArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .unmark_task_done(&args.id, args.task_number, args.notebook.as_deref())
            .await;
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
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
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
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
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }

    async fn dispatch_folders(&self, args: FoldersArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .list_folders(args.parent.as_deref(), args.notebook.as_deref())
            .await;
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }

    async fn dispatch_mkdir(&self, args: MkdirArgs) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .add_folder(&args.path, args.notebook.as_deref())
            .await;
        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
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

fn parse_edit_args(value: serde_json::Value) -> Result<EditArgs, String> {
    if value.is_null() {
        return Err("Invalid args for edit.\n\
             Reason: args is null.\n\
             Hint: choose overwrite, append, or prepend for mode; id and content are required."
            .to_string());
    }
    let value = match value {
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return Err("Invalid args for edit.\n\
                     Reason: mode is required.\n\
                     Hint: choose overwrite, append, or prepend for mode."
                    .to_string());
            }
            serde_json::Value::Object(map)
        }
        other => {
            return Err(format!(
                "Invalid args for edit.\n\
                 Reason: args must be a JSON object, got {}.\n\
                 Hint: pass args as a JSON object (not a stringified JSON payload).",
                json_type_name(&other)
            ));
        }
    };

    serde_json::from_value::<EditArgs>(value).map_err(|err| {
        if err.to_string().contains("missing field `mode`") {
            "Invalid args for edit.\n\
             Reason: mode is required.\n\
             Hint: choose overwrite, append, or prepend for mode."
                .to_string()
        } else {
            format!(
                "Invalid args for edit.\n\
                 Reason: {}.\n\
                 Hint: choose overwrite, append, or prepend for mode.",
                err
            )
        }
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
                "Call help with query 'nb.<command>' for exact command schemas.",
                "First-class tools: add, show, edit, delete, move, list, search, todo, do, undo, tasks, bookmark, folders, mkdir, import, status, notebooks. These expose typed schemas directly and bypass the multiplexed command dispatch. Some clients may display these with server-prefixed names (e.g., nb_add)."
            ],
            "commands": [
                {"command": "nb.status", "description": "Show current notebook and stats"},
                {"command": "nb.notebooks", "description": "List available notebooks (list-only; no add/delete in MCP)"},
                {"command": "nb.add", "description": "Create a new note (folder required by default)"},
                {"command": "nb.show", "description": "Read a note's content"},
                {"command": "nb.edit", "description": "Update a note's content (mode required: overwrite, append, or prepend)"},
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
                {"tool": "show", "description": "Read a note's content"},
                {"tool": "edit", "description": "Update a note's content (mode required: overwrite, append, or prepend)"},
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
            "Read a note's content",
            json_schema_for::<ShowArgs>(),
        ),
        "nb.edit" => command_help(
            "nb.edit",
            "Update a note's content (mode required: overwrite, append, or prepend).",
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
            "Read a note's content.",
            json_schema_for::<ShowArgs>(),
        ),
        "edit" => first_class_help(
            "edit",
            "Update a note's content (mode required: overwrite, append, or prepend).",
            json_schema_for::<EditArgs>(),
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
