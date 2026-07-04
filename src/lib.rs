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

impl Config {
    /// Convert to an `nb_api::Config`, resolving `NB_MCP_NOTEBOOK` as fallback.
    ///
    /// Resolution order for notebook name:
    /// 1. `self.notebook` (CLI --notebook)
    /// 2. `NB_MCP_NOTEBOOK` env var
    /// 3. Git-derived fallback (handled by `nb_api::NbClient::new`)
    pub fn to_nb_api_config(&self) -> nb_api::Config {
        let notebook = self
            .notebook
            .clone()
            .or_else(|| std::env::var("NB_MCP_NOTEBOOK").ok());
        nb_api::Config {
            notebook,
            create_notebook: self.create_notebook,
            allow_top_level_notes: self.allow_top_level_notes,
            disable_git_signing: self.commit_signing_disabled,
        }
    }
}
