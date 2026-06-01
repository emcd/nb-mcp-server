pub mod git_signing;
pub mod mcp;
pub mod nb;
pub mod paths;

/// Command-line configuration for the MCP server.
#[derive(Clone)]
pub struct Config {
    /// Default notebook (CLI --notebook overrides NB_MCP_NOTEBOOK env var).
    pub notebook: Option<String>,
    /// Disable commit and tag signing in the notebook repository.
    pub commit_signing_disabled: bool,
    /// Automatically create missing notebooks.
    pub create_notebook: bool,
    /// Allow new notes to be created at notebook root.
    pub allow_top_level_notes: bool,
    /// Show notebook and state paths, then exit.
    pub show_paths: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            notebook: None,
            commit_signing_disabled: false,
            create_notebook: true,
            allow_top_level_notes: false,
            show_paths: false,
        }
    }
}
