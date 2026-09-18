//! First-class tool registrations, multiplexed routing entry, and startup.

use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use tracing::{info, warn};

use crate::Config;
use crate::git_signing;
use crate::nb::NbClient;

use super::McpServer;
use super::help::help_tool;
use super::schemas::*;

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
        description = "Create a new note (DocumentKind::Note). Do NOT use for todos or checklists — use the `todo` tool instead; todo-shaped content (checkboxes, Tasks heading) is rejected. The folder field is required by default; use nb.mkdir to create folders and nb.folders to list them."
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
        description = "Create a todo item (DocumentKind::Todo) with checkbox state and optional checklist tasks. Use this — not `add` — for anything with checkboxes or a Tasks section. The folder field is required by default; title is required; optional description/content and tasks[] create checklist items."
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
