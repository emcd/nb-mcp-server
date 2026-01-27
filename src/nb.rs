//! Client for invoking the `nb` CLI.
//!
//! Handles notebook qualification, escaping, and output parsing.

use std::process::Stdio;

use tokio::process::Command;

/// Errors from nb CLI invocation.
#[derive(Debug, thiserror::Error)]
pub enum NbError {
    #[error("nb command failed: {0}")]
    CommandFailed(String),

    #[error("nb not found in PATH")]
    NotFound,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Client for invoking nb commands.
#[derive(Clone)]
pub struct NbClient {
    /// Default notebook to use if not specified per-command.
    default_notebook: Option<String>,
}

impl NbClient {
    /// Creates a new nb client.
    pub fn new() -> anyhow::Result<Self> {
        // TODO: Read default notebook from environment or config.
        let default_notebook = std::env::var("NB_MCP_NOTEBOOK").ok();
        Ok(Self { default_notebook })
    }

    /// Returns the notebook prefix for commands (e.g., "mynotebook:").
    fn notebook_prefix(&self, notebook: Option<&str>) -> String {
        notebook
            .or(self.default_notebook.as_deref())
            .map(|n| format!("{}:", n))
            .unwrap_or_default()
    }

    /// Executes an nb command and returns stdout.
    async fn exec(&self, args: &[&str]) -> Result<String, NbError> {
        let output = Command::new("nb")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(NbError::CommandFailed(stderr.to_string()))
        }
    }

    /// Returns status information about the current/default notebook.
    pub async fn status(&self) -> Result<String, NbError> {
        // nb status shows current notebook info.
        self.exec(&["status"]).await
    }

    /// Lists available notebooks.
    pub async fn notebooks(&self) -> Result<String, NbError> {
        self.exec(&["notebooks"]).await
    }

    // TODO: Implement remaining commands:
    // - add(title, content, tags, folder, notebook)
    // - show(id, notebook)
    // - edit(id, content, notebook)
    // - delete(id, confirm, notebook)
    // - list(folder, tags, limit, verbosity, notebook)
    // - search(query, tags, notebook)
    // - todo(description, due, tags, notebook)
    // - do_task(id, notebook)
    // - undo_task(id, notebook)
    // - tasks(status, notebook)
    // - bookmark(url, title, tags, comment, notebook)
    // - folders(parent, notebook)
    // - mkdir(path, notebook)
}
