//! Typed Rust interface to the `nb` note-taking CLI.
//!
//! Handles notebook qualification, escaping, and output parsing.
//! Wraps the `nb` CLI as a subprocess, providing async methods for
//! all note-taking operations.

mod git;

use std::{collections::VecDeque, path::PathBuf, process::Stdio, sync::LazyLock};

use regex::Regex;
use serde::Deserialize;
use tokio::process::Command;

pub use git::{derive_git_notebook_name, git_rev_parse};

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

/// Configuration for constructing an [`NbClient`].
///
/// Contains only nb-relevant fields. MCP-specific fields
/// (e.g., `show_paths`) remain in the server's config.
#[derive(Clone, Debug)]
pub struct Config {
    /// Default notebook name (overrides Git-derived fallback).
    pub notebook: Option<String>,
    /// Automatically create missing notebooks.
    pub create_notebook: bool,
    /// Allow new notes to be created at notebook root.
    pub allow_top_level_notes: bool,
    /// Disable Git commit and tag signing for `nb` subprocesses.
    pub disable_git_signing: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            notebook: None,
            create_notebook: true,
            allow_top_level_notes: false,
            disable_git_signing: false,
        }
    }
}

/// Client for invoking nb commands.
#[derive(Clone)]
pub struct NbClient {
    /// Default notebook to use if not specified per-command.
    default_notebook: Option<String>,
    /// Automatically create missing notebooks.
    create_notebook: bool,
    /// Disable Git commit and tag signing for `nb` subprocesses.
    disable_git_signing: bool,
    /// Allow new notes to be created at notebook root.
    allow_top_level_notes: bool,
}

const FOLDER_REQUIRED_MESSAGE: &str = "This server is configured to require `folder` for new notes. Use the `nb.mkdir` tool to create new folders and the `nb.folders` tool to list existing folders.";

const NOTEBOOK_FIELD_MESSAGE: &str = "Invalid `notebook`: use a bare notebook name only. Use `folder` for folder paths and `id`/`selector` for note selectors.";

const FOLDER_FIELD_MESSAGE: &str = "Invalid folder path: use `folder` for folder paths only, not notebook-qualified selectors. To choose a notebook, use the separate `notebook` field.";

/// Behavior mode for `nb edit` content updates.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EditMode {
    /// Replace note content using `nb edit --overwrite`.
    #[default]
    Replace,
    /// Append content using `nb edit --content` (nb default behavior).
    Append,
    /// Prepend content using `nb edit --prepend`.
    Prepend,
}

/// Matching mode for `nb search` query terms.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    /// Match any query term (`OR` semantics).
    #[default]
    Any,
    /// Require all query terms (`AND` semantics).
    All,
}

/// Status filter for `nb tasks`.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Return open tasks.
    Open,
    /// Return closed tasks.
    Closed,
}

impl NbClient {
    /// Creates a new nb client.
    ///
    /// Uses the notebook from config if set, otherwise falls back to a
    /// Git-derived notebook name. Does NOT read `NB_MCP_NOTEBOOK` —
    /// that is an MCP-server-specific env var resolved by the server.
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let default_notebook = config
            .notebook
            .as_deref()
            .map(String::from)
            .or_else(derive_git_notebook_name);
        Ok(Self {
            default_notebook,
            create_notebook: config.create_notebook,
            disable_git_signing: config.disable_git_signing,
            allow_top_level_notes: config.allow_top_level_notes,
        })
    }

    fn require_folder_for_new_note(&self, folder: Option<&str>) -> Result<(), NbError> {
        if self.allow_top_level_notes || folder.is_some_and(|value| !value.trim().is_empty()) {
            return Ok(());
        }
        Err(NbError::CommandFailed(FOLDER_REQUIRED_MESSAGE.to_string()))
    }

    async fn resolve_target_selector(
        &self,
        id: &str,
        notebook: Option<&str>,
    ) -> Result<(String, String), NbError> {
        if let Some((embedded_notebook, path)) = parse_qualified_selector(id)? {
            let notebook = match notebook {
                Some(value) => {
                    validate_notebook_name(value)?;
                    if value != embedded_notebook {
                        return Err(NbError::CommandFailed(format!(
                            "ambiguous selector: id targets notebook `{embedded_notebook}`, but notebook field is `{value}`"
                        )));
                    }
                    embedded_notebook.to_string()
                }
                _ => embedded_notebook.to_string(),
            };
            self.ensure_existing_notebook(&notebook).await?;
            return Ok((notebook, format!("{}:{}", embedded_notebook, path)));
        }
        let notebook = self.resolve_notebook(notebook).await?;
        Ok((notebook.clone(), format!("{}:{}", notebook, id)))
    }

    fn append_notebook_warning(&self, output: String, notebook: &str) -> String {
        let Some(default_notebook) = self.default_notebook.as_deref() else {
            return output;
        };
        if default_notebook == notebook {
            return output;
        }
        append_warning(
            output,
            format!(
                "Warning: wrote to notebook `{notebook}`, not the project default notebook `{default_notebook}`. If this was unintended, move or delete the note and retry with the correct notebook/folder."
            ),
        )
    }

    /// Resolves the notebook to use for a command.
    fn resolve_notebook_name(&self, notebook: Option<&str>) -> Result<String, NbError> {
        if let Some(name) = notebook {
            validate_notebook_name(name)?;
            return Ok(name.to_string());
        }
        if let Some(name) = self.default_notebook.as_deref() {
            validate_notebook_name(name)?;
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
        match self.check_notebook(notebook).await {
            Ok(()) => Ok(()),
            Err(_) => {
                if !self.create_notebook {
                    return Err(NbError::CommandFailed(format!(
                        "notebook not found; create it with the nb CLI (`nb notebooks add {}`) \
                         or remove --no-create-notebook",
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

    async fn ensure_existing_notebook(&self, notebook: &str) -> Result<(), NbError> {
        self.check_notebook(notebook).await.map_err(|_| {
            NbError::CommandFailed(format!(
                "notebook not found: `{notebook}`. Use a copied selector only for an existing notebook."
            ))
        })
    }

    async fn check_notebook(&self, notebook: &str) -> Result<(), NbError> {
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
            Err(_) => Err(NbError::CommandFailed(format!(
                "notebook not found: `{notebook}`"
            ))),
        }
    }

    /// Executes an nb command and returns stdout.
    async fn exec(&self, args: &[&str]) -> Result<String, NbError> {
        tracing::debug!(?args, "executing nb command");
        let mut command = Command::new("nb");
        command
            .args(args)
            .stdin(Stdio::null()) // Prevent TTY hangs
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if self.disable_git_signing {
            apply_git_signing_env(&mut command);
        }
        let output = command
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
        self.require_folder_for_new_note(folder)?;
        validate_folder_option(folder)?;

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

        self.exec_vec(args)
            .await
            .map(|output| self.append_notebook_warning(output, &notebook))
    }

    /// Shows a note's content.
    pub async fn show(&self, id: &str, notebook: Option<&str>) -> Result<String, NbError> {
        let (_, selector) = self.resolve_target_selector(id, notebook).await?;
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
        validate_folder_option(folder)?;

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
        queries: &[String],
        mode: SearchMode,
        tags: &[String],
        folder: Option<&str>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        validate_folder_option(folder)?;
        if queries.is_empty() {
            return Err(NbError::CommandFailed(
                "at least one search query is required".to_string(),
            ));
        }

        let notebook = self.resolve_notebook(notebook).await?;
        let scope = match folder {
            Some(f) => format!("{}:{}/", notebook, f),
            None => format!("{}:", notebook),
        };
        let args = search_command_args(scope, queries, mode, tags);
        self.exec_vec(args).await
    }

    /// Edits a note using the provided content mode.
    pub async fn edit(
        &self,
        id: &str,
        content: &str,
        mode: EditMode,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let (notebook, selector) = self.resolve_target_selector(id, notebook).await?;
        let output = self.exec_vec(edit_args(selector, content, mode)).await?;
        Ok(self.append_notebook_warning(output, &notebook))
    }

    /// Deletes a note.
    pub async fn delete(&self, id: &str, notebook: Option<&str>) -> Result<String, NbError> {
        let (notebook, selector) = self.resolve_target_selector(id, notebook).await?;
        let output = self
            .exec_vec(vec!["delete".to_string(), selector, "--force".to_string()])
            .await?;
        Ok(self.append_notebook_warning(output, &notebook))
    }

    /// Moves or renames a note.
    pub async fn move_note(
        &self,
        id: &str,
        destination: &str,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        validate_destination(destination)?;
        let (notebook, selector) = self.resolve_target_selector(id, notebook).await?;
        let output = self
            .exec_vec(vec![
                "move".to_string(),
                selector,
                destination.to_string(),
                "--force".to_string(),
            ])
            .await?;
        Ok(self.append_notebook_warning(output, &notebook))
    }

    /// Creates a todo item.
    pub async fn todo(
        &self,
        title: &str,
        description: Option<&str>,
        tasks: &[String],
        tags: &[String],
        folder: Option<&str>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        self.require_folder_for_new_note(folder)?;
        validate_folder_option(folder)?;
        let notebook = self.resolve_notebook(notebook).await?;
        let output = self
            .exec_vec(todo_command_args(
                &notebook,
                title,
                description,
                tasks,
                tags,
                folder,
            ))
            .await?;
        Ok(self.append_notebook_warning(output, &notebook))
    }

    /// Marks a todo as done.
    pub async fn do_task(
        &self,
        id: &str,
        task_number: Option<u32>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let (notebook, selector) = self.resolve_target_selector(id, notebook).await?;
        let output = self
            .exec_vec(task_command_args("do", selector, task_number))
            .await?;
        Ok(self.append_notebook_warning(output, &notebook))
    }

    /// Marks a todo as not done.
    pub async fn undo_task(
        &self,
        id: &str,
        task_number: Option<u32>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let (notebook, selector) = self.resolve_target_selector(id, notebook).await?;
        let output = self
            .exec_vec(task_command_args("undo", selector, task_number))
            .await?;
        Ok(self.append_notebook_warning(output, &notebook))
    }

    /// Lists todos.
    pub async fn tasks(
        &self,
        folder: Option<&str>,
        status: Option<TaskStatus>,
        recursive: bool,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        validate_folder_option(folder)?;
        let notebook = self.resolve_notebook(notebook).await?;
        let folder = folder.map(normalize_folder);
        let scopes = if recursive {
            self.tasks_scopes_recursive(&notebook, folder.as_deref())
                .await?
        } else {
            vec![tasks_scope(&notebook, folder.as_deref())]
        };

        let mut outputs: Vec<String> = Vec::new();
        let mut saw_empty = false;
        for scope in scopes {
            match self.exec_vec(tasks_command_args(scope, status)).await {
                Ok(output) => {
                    let output = output.trim();
                    if !output.is_empty() {
                        outputs.push(output.to_string());
                    }
                }
                Err(NbError::CommandFailed(message)) if is_empty_tasks_error(&message) => {
                    saw_empty = true;
                }
                Err(err) => return Err(err),
            }
        }
        if outputs.is_empty() && saw_empty {
            return Err(NbError::CommandFailed(empty_tasks_message(status)));
        }
        Ok(outputs.join("\n"))
    }

    async fn tasks_scopes_recursive(
        &self,
        notebook: &str,
        folder: Option<&str>,
    ) -> Result<Vec<String>, NbError> {
        let notebook_root = self.notebook_path(Some(notebook)).await?;
        let start = folder.unwrap_or_default().to_string();
        let mut queue = VecDeque::new();
        queue.push_back(start.clone());

        let mut scopes = vec![tasks_scope(notebook, folder)];
        while let Some(current) = queue.pop_front() {
            let base = if current.is_empty() {
                notebook_root.clone()
            } else {
                notebook_root.join(&current)
            };
            let children = child_folder_names(&base)?;
            for child in children {
                let next = if current.is_empty() {
                    child
                } else {
                    format!("{}/{}", current, child)
                };
                scopes.push(tasks_scope(notebook, Some(&next)));
                queue.push_back(next);
            }
        }
        Ok(scopes)
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
        self.require_folder_for_new_note(folder)?;
        validate_folder_option(folder)?;

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

        self.exec_vec(args)
            .await
            .map(|output| self.append_notebook_warning(output, &notebook))
    }

    /// Lists folders in a notebook.
    pub async fn folders(
        &self,
        parent: Option<&str>,
        notebook: Option<&str>,
    ) -> Result<String, NbError> {
        let mut args = vec!["list".to_string()];
        validate_folder_option(parent)?;

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
        validate_folder_path(path)?;
        let notebook = self.resolve_notebook(notebook).await?;
        let folder_path = mkdir_selector(&notebook, path);
        let output = self
            .exec_vec(vec!["add".to_string(), "folder".to_string(), folder_path])
            .await?;
        Ok(self.append_notebook_warning(output, &notebook))
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
        self.require_folder_for_new_note(folder)?;
        validate_folder_option(folder)?;

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

        self.exec_vec(args)
            .await
            .map(|output| self.append_notebook_warning(output, &notebook))
    }
}

fn append_warning(mut output: String, warning: String) -> String {
    if !output.trim().is_empty() {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push('\n');
    }
    output.push_str(&warning);
    output
}

fn validate_notebook_name(name: &str) -> Result<(), NbError> {
    if name.trim().is_empty() || name.contains(':') || name.contains('/') || name.contains('\\') {
        return Err(NbError::CommandFailed(NOTEBOOK_FIELD_MESSAGE.to_string()));
    }
    Ok(())
}

fn validate_folder_option(folder: Option<&str>) -> Result<(), NbError> {
    if let Some(path) = folder {
        validate_folder_path(path)?;
    }
    Ok(())
}

fn validate_folder_path(path: &str) -> Result<(), NbError> {
    if path.trim().is_empty() || path.contains(':') {
        return Err(NbError::CommandFailed(FOLDER_FIELD_MESSAGE.to_string()));
    }
    Ok(())
}

fn validate_destination(destination: &str) -> Result<(), NbError> {
    if destination.trim().is_empty() || destination.contains(':') {
        return Err(NbError::CommandFailed(
            "Invalid destination: use a folder path or filename only, not a notebook-qualified selector."
                .to_string(),
        ));
    }
    Ok(())
}

fn parse_qualified_selector(selector: &str) -> Result<Option<(&str, &str)>, NbError> {
    let Some((notebook, path)) = selector.split_once(':') else {
        return Ok(None);
    };
    validate_notebook_name(notebook)?;
    if path.trim().is_empty() || path.contains(':') {
        return Err(NbError::CommandFailed(
            "Invalid selector: use at most one notebook qualifier, as `<notebook>:<folder>/<id>`."
                .to_string(),
        ));
    }
    Ok(Some((notebook, path)))
}

fn edit_args(selector: String, content: &str, mode: EditMode) -> Vec<String> {
    let mut args = vec!["edit".to_string(), selector];
    match mode {
        EditMode::Replace => args.push("--overwrite".to_string()),
        EditMode::Append => {}
        EditMode::Prepend => args.push("--prepend".to_string()),
    }
    args.push("--content".to_string());
    args.push(content.to_string());
    args
}

fn task_command_args(action: &str, selector: String, task_number: Option<u32>) -> Vec<String> {
    let mut args = vec![action.to_string(), selector];
    if let Some(number) = task_number {
        args.push(number.to_string());
    }
    args
}

fn todo_command_args(
    notebook: &str,
    title: &str,
    description: Option<&str>,
    tasks: &[String],
    tags: &[String],
    folder: Option<&str>,
) -> Vec<String> {
    let mut args = vec![format!("{notebook}:todo"), "add".to_string()];

    // Folder path comes as a positional argument before the title.
    if let Some(folder) = folder {
        args.push(folder_scope(folder));
    }

    args.push(title.to_string());

    if let Some(description) = description {
        args.push("--description".to_string());
        args.push(description.to_string());
    }

    for task in tasks {
        args.push("--task".to_string());
        args.push(task.to_string());
    }

    for tag in tags {
        args.push("--tags".to_string());
        args.push(normalize_tag(tag));
    }

    args
}

fn folder_scope(folder: &str) -> String {
    if folder.ends_with('/') {
        folder.to_string()
    } else {
        format!("{folder}/")
    }
}

fn normalize_tag(tag: &str) -> String {
    if tag.starts_with('#') {
        tag.to_string()
    } else {
        format!("#{tag}")
    }
}

fn normalize_folder(folder: &str) -> String {
    folder.trim_matches('/').to_string()
}

fn mkdir_selector(notebook: &str, path: &str) -> String {
    let normalized = normalize_folder(path);
    format!("{}:{}", notebook, normalized)
}

fn tasks_scope(notebook: &str, folder: Option<&str>) -> String {
    match folder {
        Some(path) if !path.is_empty() => format!("{}:{}/", notebook, path),
        _ => format!("{}:", notebook),
    }
}

fn tasks_command_args(scope: String, status: Option<TaskStatus>) -> Vec<String> {
    let mut args = vec!["tasks".to_string(), scope];
    if let Some(filter) = status {
        let status = match filter {
            TaskStatus::Open => "open",
            TaskStatus::Closed => "closed",
        };
        args.push(status.to_string());
    }
    args.push("--no-color".to_string());
    args
}

fn search_command_args(
    scope: String,
    queries: &[String],
    mode: SearchMode,
    tags: &[String],
) -> Vec<String> {
    let mut args = vec!["search".to_string(), scope];
    let mut terms = queries.iter();
    if let Some(first) = terms.next() {
        args.push(first.to_string());
    }
    match mode {
        SearchMode::Any => {
            for query in terms {
                args.push("--or".to_string());
                args.push(query.to_string());
            }
        }
        SearchMode::All => {
            for query in terms {
                args.push(query.to_string());
            }
        }
    }
    for tag in tags {
        args.push("--tag".to_string());
        args.push(normalize_tag(tag));
    }
    args.push("--no-color".to_string());
    args
}

fn is_empty_tasks_error(message: &str) -> bool {
    message.trim_start().starts_with("! 0 ") && message.contains(" tasks.")
}

fn empty_tasks_message(status: Option<TaskStatus>) -> String {
    match status {
        Some(TaskStatus::Open) => "! 0 open tasks.".to_string(),
        Some(TaskStatus::Closed) => "! 0 closed tasks.".to_string(),
        None => "! 0 tasks.".to_string(),
    }
}

fn child_folder_names(path: &std::path::Path) -> Result<Vec<String>, NbError> {
    let read_dir = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(NbError::Io(err)),
    };

    let mut names = Vec::new();
    for entry in read_dir {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(|value| value.to_string()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(meta) => meta,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(NbError::Io(err)),
        };
        if meta.is_dir() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

const GIT_SIGNING_OVERRIDES: [(&str, &str); 2] =
    [("commit.gpgsign", "false"), ("tag.gpgsign", "false")];

fn git_config_count(raw: Option<&str>) -> usize {
    raw.and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
}

fn git_signing_env_vars(start_index: usize) -> Vec<(String, String)> {
    let total = start_index.saturating_add(GIT_SIGNING_OVERRIDES.len());
    let mut env_vars = Vec::with_capacity(1 + GIT_SIGNING_OVERRIDES.len() * 2);
    env_vars.push(("GIT_CONFIG_COUNT".to_string(), total.to_string()));
    for (offset, (key, value)) in GIT_SIGNING_OVERRIDES.iter().enumerate() {
        let index = start_index + offset;
        env_vars.push((format!("GIT_CONFIG_KEY_{index}"), (*key).to_string()));
        env_vars.push((format!("GIT_CONFIG_VALUE_{index}"), (*value).to_string()));
    }
    env_vars
}

fn apply_git_signing_env(command: &mut Command) {
    let start_index = git_config_count(std::env::var("GIT_CONFIG_COUNT").ok().as_deref());
    for (name, value) in git_signing_env_vars(start_index) {
        command.env(name, value);
    }
}
