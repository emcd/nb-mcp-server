use anyhow::Result;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServiceExt,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::info;

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

#[tool_router]
impl McpServer {
    fn new() -> Result<Self> {
        let nb = NbClient::new()?;
        Ok(Self {
            nb,
            tool_router: Self::tool_router(),
        })
    }

    #[tool(
        description = "nb note-taking tool. Commands: status, add, show, edit, delete, list, search, todo, do, undo, tasks, bookmark, folders, mkdir, notebooks. Use `help` for schemas."
    )]
    async fn nb(
        &self,
        Parameters(call): Parameters<NbCall>,
    ) -> Result<CallToolResult, McpError> {
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

pub async fn run() -> Result<()> {
    let server = McpServer::new()?;
    info!("starting nb-mcp server");
    let service = server.serve(stdio()).await?;
    info!("nb-mcp server ready");
    service.waiting().await?;
    Ok(())
}

impl McpServer {
    async fn dispatch_nb(&self, call: NbCall) -> Result<CallToolResult, McpError> {
        let command = call.command.trim();
        if command.is_empty() {
            return Err(McpError::invalid_params(
                "command must be non-empty",
                None,
            ));
        }

        // Strip "nb." prefix if present.
        let subcommand = command
            .strip_prefix("nb.")
            .unwrap_or(command);

        let result = match subcommand {
            "status" => self.nb.status().await,
            "notebooks" => self.nb.notebooks().await,
            // TODO: Implement remaining commands.
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
            ],
            "invoke": {
                "tool": "nb",
                "params": {"command": "nb.<subcommand>", "args": {}},
            },
        }),
        _ => {
            return Err(McpError::invalid_params(
                "unknown query; try 'nb' for command list",
                Some(serde_json::json!({"query": query})),
            ));
        }
    };

    Ok(CallToolResult::success(vec![Content::json(response)?]))
}
