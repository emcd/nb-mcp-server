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

    #[error("nb not found in PATH; install via: brew install xwmx/taps/nb (macOS) or see https://github.com/xwmx/nb#installation")]
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
    ///
    /// CLI notebook argument takes precedence over NB_MCP_NOTEBOOK env var.
    pub fn new(cli_notebook: Option<&str>) -> anyhow::Result<Self> {
        let default_notebook = cli_notebook
            .map(String::from)
            .or_else(|| std::env::var("NB_MCP_NOTEBOOK").ok());
        Ok(Self { default_notebook })
    }

    /// Resolves the notebook to use for a command.
    fn resolve_notebook<'a>(&'a self, notebook: Option<&'a str>) -> Option<&'a str> {
        notebook.or(self.default_notebook.as_deref())
    }

    /// Executes an nb command and returns stdout.
    async fn exec(&self, args: &[&str]) -> Result<String, NbError> {
        tracing::debug!(?args, "executing nb command");
        let output = Command::new("nb")
            .args(args)
            .stdin(Stdio::null()) // Prevent TTY hangs
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    NbError::NotFound
                } else {
                    NbError::Io(e)
                }
            })?
            .wait_with_output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            // nb sometimes writes errors to stdout
            let msg = if stderr.is_empty() {
                stdout.to_string()
            } else {
                stderr.to_string()
            };
            Err(NbError::CommandFailed(msg))
        }
    }

    /// Executes an nb command with dynamic arguments.
    async fn exec_vec(&self, args: Vec<String>) -> Result<String, NbError> {
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_ref).await
    }

    /// Returns status information about the current/default notebook.
    pub async fn status(&self, notebook: Option<&str>) -> Result<String, NbError> {
        match self.resolve_notebook(notebook) {
            Some(nb) => self.exec(&[&format!("{}:", nb), "status"]).await,
            None => self.exec(&["status"]).await,
        }
    }

    /// Lists available notebooks.
    pub async fn notebooks(&self) -> Result<String, NbError> {
        // Use --no-color to avoid ANSI escape codes
        self.exec(&["notebooks", "--no-color"]).await
    }

    /// Creates a new note.
    pub async fn add(
        &self,
        title: Option<&str>,
        content: &str,
        tags: &[String],
        folder: Option<&str>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = Vec::new();

        // Notebook-qualified command
        let cmd = match self.resolve_notebook(notebook) {
            Some(nb) => format!("{}:add", nb),
            None => "add".to_string(),
        };
        args.push(cmd);

        // Title (if provided)
        if let Some(t) = title {
            args.push("--title".to_string());
            args.push(t.to_string());
        }

        // Content via --content flag (avoids shell escaping issues)
        args.push("--content".to_string());
        args.push(content.to_string());

        // Tags (nb expects #hashtag format)
        for tag in tags {
            args.push("--tags".to_string());
            let tag_str = if tag.starts_with('#') {
                tag.clone()
            } else {
                format!("#{}", tag)
            };
            args.push(tag_str);
        }

        // Folder
        if let Some(f) = folder {
            args.push("--folder".to_string());
            args.push(f.to_string());
        }

        self.exec_vec(args).await
    }

    /// Shows a note's content.
    pub async fn show(
        &self,
        id: &str,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let selector = match self.resolve_notebook(notebook) {
            Some(nb) => format!("{}:{}", nb, id),
            None => id.to_string(),
        };
        // --no-color avoids syntax highlighting ANSI codes
        self.exec(&["show", &selector, "--no-color"]).await
    }

    /// Lists notes in a notebook or folder.
    pub async fn list(
        &self,
        folder: Option<&str>,
        tags: &[String],
        limit: Option<u32>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = Vec::new();

        // Base command with optional notebook
        let cmd = match self.resolve_notebook(notebook) {
            Some(nb) => match folder {
                Some(f) => format!("{}:{}/", nb, f),
                None => format!("{}:", nb),
            },
            None => match folder {
                Some(f) => format!("{}/", f),
                None => String::new(),
            },
        };

        args.push("list".to_string());
        if !cmd.is_empty() {
            args.push(cmd);
        }

        // No color for parsing
        args.push("--no-color".to_string());

        // Limit
        if let Some(n) = limit {
            args.push("-n".to_string());
            args.push(n.to_string());
        }

        // Tags filter
        for tag in tags {
            args.push("--tags".to_string());
            let tag_str = if tag.starts_with('#') {
                tag.clone()
            } else {
                format!("#{}", tag)
            };
            args.push(tag_str);
        }

        self.exec_vec(args).await
    }

    /// Searches notes.
    pub async fn search(
        &self,
        query: &str,
        tags: &[String],
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = vec!["search".to_string()];

        // Notebook scope
        if let Some(nb) = self.resolve_notebook(notebook) {
            args.push(format!("{}:", nb));
        }

        // Query
        args.push(query.to_string());

        // Tags
        for tag in tags {
            args.push("--tag".to_string());
            let tag_str = if tag.starts_with('#') {
                tag.clone()
            } else {
                format!("#{}", tag)
            };
            args.push(tag_str);
        }

        // No color
        args.push("--no-color".to_string());

        self.exec_vec(args).await
    }

    /// Edits a note by replacing its content.
    pub async fn edit(
        &self,
        id: &str,
        content: &str,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let selector = match self.resolve_notebook(notebook) {
            Some(nb) => format!("{}:{}", nb, id),
            None => id.to_string(),
        };
        // --content replaces the note content
        self.exec(&["edit", &selector, "--content", content]).await
    }

    /// Deletes a note.
    pub async fn delete(
        &self,
        id: &str,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let selector = match self.resolve_notebook(notebook) {
            Some(nb) => format!("{}:{}", nb, id),
            None => id.to_string(),
        };
        // --force skips confirmation prompt
        self.exec(&["delete", &selector, "--force"]).await
    }

    /// Creates a todo item.
    pub async fn todo(
        &self,
        description: &str,
        tags: &[String],
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = Vec::new();

        let cmd = match self.resolve_notebook(notebook) {
            Some(nb) => format!("{}:todo", nb),
            None => "todo".to_string(),
        };
        args.push(cmd);
        args.push("add".to_string());
        args.push(description.to_string());

        for tag in tags {
            args.push("--tags".to_string());
            let tag_str = if tag.starts_with('#') {
                tag.clone()
            } else {
                format!("#{}", tag)
            };
            args.push(tag_str);
        }

        self.exec_vec(args).await
    }

    /// Marks a todo as done.
    pub async fn do_task(
        &self,
        id: &str,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let selector = match self.resolve_notebook(notebook) {
            Some(nb) => format!("{}:{}", nb, id),
            None => id.to_string(),
        };
        self.exec(&["do", &selector]).await
    }

    /// Marks a todo as not done.
    pub async fn undo_task(
        &self,
        id: &str,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let selector = match self.resolve_notebook(notebook) {
            Some(nb) => format!("{}:{}", nb, id),
            None => id.to_string(),
        };
        self.exec(&["undo", &selector]).await
    }

    /// Lists todos.
    pub async fn tasks(
        &self,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = vec!["tasks".to_string()];

        if let Some(nb) = self.resolve_notebook(notebook) {
            args.push(format!("{}:", nb));
        }

        args.push("--no-color".to_string());

        self.exec_vec(args).await
    }

    /// Creates a bookmark.
    pub async fn bookmark(
        &self,
        url: &str,
        title: Option<&str>,
        tags: &[String],
        comment: Option<&str>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = Vec::new();

        let cmd = match self.resolve_notebook(notebook) {
            Some(nb) => format!("{}:bookmark", nb),
            None => "bookmark".to_string(),
        };
        args.push(cmd);
        args.push(url.to_string());

        if let Some(t) = title {
            args.push("--title".to_string());
            args.push(t.to_string());
        }

        if let Some(c) = comment {
            args.push("--comment".to_string());
            args.push(c.to_string());
        }

        for tag in tags {
            args.push("--tags".to_string());
            let tag_str = if tag.starts_with('#') {
                tag.clone()
            } else {
                format!("#{}", tag)
            };
            args.push(tag_str);
        }

        self.exec_vec(args).await
    }

    /// Lists folders in a notebook.
    pub async fn folders(
        &self,
        parent: Option<&str>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = vec!["list".to_string()];

        let path = match self.resolve_notebook(notebook) {
            Some(nb) => match parent {
                Some(p) => format!("{}:{}/", nb, p),
                None => format!("{}:", nb),
            },
            None => match parent {
                Some(p) => format!("{}/", p),
                None => String::new(),
            },
        };

        if !path.is_empty() {
            args.push(path);
        }

        // Filter to only show folders
        args.push("--type".to_string());
        args.push("folder".to_string());
        args.push("--no-color".to_string());

        self.exec_vec(args).await
    }

    /// Creates a folder.
    pub async fn mkdir(
        &self,
        path: &str,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let folder_path = match self.resolve_notebook(notebook) {
            Some(nb) => format!("{}:{}/", nb, path),
            None => format!("{}/", path),
        };
        self.exec(&["add", "folder", &folder_path]).await
    }

    /// Imports a file or URL into the notebook.
    pub async fn import(
        &self,
        source: &str,
        folder: Option<&str>,
        filename: Option<&str>,
        convert: bool,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = Vec::new();

        // Notebook-qualified command
        let cmd = match self.resolve_notebook(notebook) {
            Some(nb) => format!("{}:import", nb),
            None => "import".to_string(),
        };
        args.push(cmd);

        // Source path or URL
        args.push(source.to_string());

        // Convert HTML to Markdown
        if convert {
            args.push("--convert".to_string());
        }

        // Destination: notebook:folder/filename or just folder/filename
        // nb import expects destination as a positional argument after source
        if folder.is_some() || filename.is_some() {
            let dest = match (folder, filename) {
                (Some(f), Some(n)) => format!("{}/{}", f, n),
                (Some(f), None) => format!("{}/", f),
                (None, Some(n)) => n.to_string(),
                (None, None) => unreachable!(),
            };
            args.push(dest);
        }

        self.exec_vec(args).await
    }
}
