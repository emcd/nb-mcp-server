//! Client for invoking the `nb` CLI.
//!
//! Handles notebook qualification, escaping, and output parsing.

use std::{path::PathBuf, process::Stdio, sync::LazyLock};

use regex::Regex;
use tokio::process::Command;

/// Regex to match ANSI/ISO 2022 escape sequences.
///
/// Covers:
/// - Fe sequences: `ESC [@-Z\-_]` (single byte after ESC)
/// - CSI sequences: `ESC [ ... m` (SGR colors, cursor control, etc.)
/// - nF sequences: `ESC [ -/]* [0-~]` (character set designation like `ESC ( B`)
static ANSI_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|[ -/]*[0-~])").unwrap());

/// Strip ANSI escape sequences from text.
fn strip_ansi(text: &str) -> String {
    ANSI_REGEX.replace_all(text, "").into_owned()
}

/// Errors from nb CLI invocation.
#[derive(Debug, thiserror::Error)]
pub enum NbError {
    #[error("nb command failed: {0}")]
    CommandFailed(String),

    #[error(
        "nb not found in PATH; install via: brew install xwmx/taps/nb (macOS) or see https://github.com/xwmx/nb#installation"
    )]
    NotFound,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Client for invoking nb commands.
#[derive(Clone)]
pub struct NbClient {
    /// Default notebook to use if not specified per-command.
    default_notebook: Option<String>,
    /// Automatically create missing notebooks.
    create_notebook: bool,
}

impl NbClient {
    /// Creates a new nb client.
    ///
    /// CLI notebook argument takes precedence over NB_MCP_NOTEBOOK env var.
    /// Falls back to a Git-derived notebook name when available.
    pub fn new(cli_notebook: Option<&str>, create_notebook: bool) -> anyhow::Result<Self> {
        let default_notebook = cli_notebook
            .map(String::from)
            .or_else(|| std::env::var("NB_MCP_NOTEBOOK").ok())
            .or_else(derive_git_notebook_name);
        Ok(Self {
            default_notebook,
            create_notebook,
        })
    }

    /// Resolves the notebook to use for a command.
    fn resolve_notebook_name(&self, notebook: Option<&str>) -> Result<String, NbError> {
        if let Some(name) = notebook {
            return Ok(name.to_string());
        }
        if let Some(name) = self.default_notebook.as_deref() {
            return Ok(name.to_string());
        }
        Err(NbError::CommandFailed(
            "notebook not configured; set --notebook or NB_MCP_NOTEBOOK".to_string(),
        ))
    }

    async fn resolve_notebook(&self, notebook: Option<&str>) -> Result<String, NbError> {
        let name = self.resolve_notebook_name(notebook)?;
        self.ensure_notebook(&name).await?;
        Ok(name)
    }

    async fn ensure_notebook(&self, notebook: &str) -> Result<(), NbError> {
        let show_result = self
            .exec_vec(vec![
                "notebooks".to_string(),
                "show".to_string(),
                notebook.to_string(),
                "--path".to_string(),
            ])
            .await;
        match show_result {
            Ok(output) => {
                if output.trim().is_empty() {
                    return Err(NbError::CommandFailed(
                        "nb notebooks path output was empty".to_string(),
                    ));
                }
                Ok(())
            }
            Err(_) => {
                if !self.create_notebook {
                    return Err(NbError::CommandFailed(format!(
                        "notebook not found; run `nb notebooks add {}` or remove \
                         --no-create-notebook",
                        notebook
                    )));
                }
                self.exec_vec(vec![
                    "notebooks".to_string(),
                    "add".to_string(),
                    notebook.to_string(),
                ])
                .await?;
                Ok(())
            }
        }
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
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(strip_ansi(&stdout))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            // nb sometimes writes errors to stdout
            let msg = if stderr.is_empty() {
                strip_ansi(&stdout)
            } else {
                strip_ansi(&stderr)
            };
            Err(NbError::CommandFailed(msg))
        }
    }

    /// Executes an nb command with dynamic arguments.
    async fn exec_vec(&self, args: Vec<String>) -> Result<String, NbError> {
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_ref).await
    }

    /// Returns status information about the resolved notebook.
    pub async fn status(&self, notebook: Option<&str>) -> Result<String, NbError> {
        let notebook = self.resolve_notebook(notebook).await?;
        self.exec_vec(vec![format!("{}:", notebook), "status".to_string()])
            .await
    }

    /// Lists available notebooks.
    pub async fn notebooks(&self) -> Result<String, NbError> {
        // Use --no-color to avoid ANSI escape codes
        self.exec(&["notebooks", "--no-color"]).await
    }

    /// Returns the path for a notebook.
    pub async fn notebook_path(&self, notebook: Option<&str>) -> Result<PathBuf, NbError> {
        let notebook = self.resolve_notebook(notebook).await?;
        let output = self
            .exec_vec(vec![
                "notebooks".to_string(),
                "show".to_string(),
                notebook,
                "--path".to_string(),
            ])
            .await?;
        let path = output.trim();
        if path.is_empty() {
            return Err(NbError::CommandFailed(
                "nb notebooks path output was empty".to_string(),
            ));
        }
        Ok(PathBuf::from(path))
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

        let notebook = self.resolve_notebook(notebook).await?;
        let cmd = format!("{}:add", notebook);
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
    pub async fn show(&self, id: &str, notebook: Option<&str>) -> Result<String, NbError> {
        let notebook = self.resolve_notebook(notebook).await?;
        let selector = format!("{}:{}", notebook, id);
        self.exec_vec(vec!["show".to_string(), selector, "--no-color".to_string()])
            .await
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

        let notebook = self.resolve_notebook(notebook).await?;
        let cmd = match folder {
            Some(f) => format!("{}:{}/", notebook, f),
            None => format!("{}:", notebook),
        };

        args.push("list".to_string());
        args.push(cmd);

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
        folder: Option<&str>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = vec!["search".to_string()];

        let notebook = self.resolve_notebook(notebook).await?;
        let scope = match folder {
            Some(f) => format!("{}:{}/", notebook, f),
            None => format!("{}:", notebook),
        };
        args.push(scope);

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
        let notebook = self.resolve_notebook(notebook).await?;
        let selector = format!("{}:{}", notebook, id);
        self.exec_vec(vec![
            "edit".to_string(),
            selector,
            "--content".to_string(),
            content.to_string(),
        ])
        .await
    }

    /// Deletes a note.
    pub async fn delete(&self, id: &str, notebook: Option<&str>) -> Result<String, NbError> {
        let notebook = self.resolve_notebook(notebook).await?;
        let selector = format!("{}:{}", notebook, id);
        self.exec_vec(vec!["delete".to_string(), selector, "--force".to_string()])
            .await
    }

    /// Moves or renames a note.
    pub async fn move_note(
        &self,
        id: &str,
        destination: &str,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let notebook = self.resolve_notebook(notebook).await?;
        let selector = format!("{}:{}", notebook, id);
        self.exec_vec(vec![
            "move".to_string(),
            selector,
            destination.to_string(),
            "--force".to_string(),
        ])
        .await
    }

    /// Creates a todo item.
    pub async fn todo(
        &self,
        description: &str,
        tags: &[String],
        folder: Option<&str>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = Vec::new();

        let notebook = self.resolve_notebook(notebook).await?;
        let cmd = format!("{}:todo", notebook);
        args.push(cmd);
        args.push("add".to_string());

        // Folder path comes as a positional argument before the description
        if let Some(f) = folder {
            let path = if f.ends_with('/') {
                f.to_string()
            } else {
                format!("{}/", f)
            };
            args.push(path);
        }

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
    pub async fn do_task(&self, id: &str, notebook: Option<&str>) -> Result<String, NbError> {
        let notebook = self.resolve_notebook(notebook).await?;
        let selector = format!("{}:{}", notebook, id);
        self.exec_vec(vec!["do".to_string(), selector]).await
    }

    /// Marks a todo as not done.
    pub async fn undo_task(&self, id: &str, notebook: Option<&str>) -> Result<String, NbError> {
        let notebook = self.resolve_notebook(notebook).await?;
        let selector = format!("{}:{}", notebook, id);
        self.exec_vec(vec!["undo".to_string(), selector]).await
    }

    /// Lists todos.
    pub async fn tasks(
        &self,
        folder: Option<&str>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = vec!["tasks".to_string()];

        let notebook = self.resolve_notebook(notebook).await?;
        let scope = match folder {
            Some(f) => format!("{}:{}/", notebook, f),
            None => format!("{}:", notebook),
        };
        args.push(scope);

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
        folder: Option<&str>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = Vec::new();

        // Build the destination path with optional folder
        let notebook = self.resolve_notebook(notebook).await?;
        let dest = match folder {
            Some(f) => format!("{}:{}/", notebook, f),
            None => format!("{}:", notebook),
        };

        let cmd = format!("{}bookmark", dest);
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

        let notebook = self.resolve_notebook(notebook).await?;
        let path = match parent {
            Some(p) => format!("{}:{}/", notebook, p),
            None => format!("{}:", notebook),
        };
        args.push(path);

        // Filter to only show folders
        args.push("--type".to_string());
        args.push("folder".to_string());
        args.push("--no-color".to_string());

        self.exec_vec(args).await
    }

    /// Creates a folder.
    pub async fn mkdir(&self, path: &str, notebook: Option<&str>) -> Result<String, NbError> {
        let notebook = self.resolve_notebook(notebook).await?;
        let folder_path = format!("{}:{}/", notebook, path);
        self.exec_vec(vec!["add".to_string(), "folder".to_string(), folder_path])
            .await
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

        let notebook = self.resolve_notebook(notebook).await?;
        let cmd = format!("{}:import", notebook);
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

fn derive_git_notebook_name() -> Option<String> {
    let current_root = git_rev_parse(&["--show-toplevel"])?;
    let git_common_dir = git_rev_parse(&["--git-common-dir"])?;
    let git_common_dir = if git_common_dir.is_relative() {
        current_root.join(&git_common_dir)
    } else {
        git_common_dir
    };
    let git_common_dir = git_common_dir.canonicalize().ok()?;
    let master_root = if git_common_dir.file_name().is_some_and(|n| n == ".git") {
        git_common_dir.parent()?.to_path_buf()
    } else {
        return None;
    };
    master_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
}

fn git_rev_parse(args: &[&str]) -> Option<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse"])
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let value = stdout.trim();
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}
