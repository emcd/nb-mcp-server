use anyhow::Result;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod git_signing;
mod mcp;
mod nb;
mod paths;

/// Command-line configuration for the MCP server.
pub struct Config {
    /// Default notebook (CLI --notebook overrides NB_MCP_NOTEBOOK env var).
    pub notebook: Option<String>,
    /// Disable commit and tag signing in the notebook repository.
    pub commit_signing_disabled: bool,
    /// Automatically create missing notebooks.
    pub create_notebook: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            notebook: None,
            commit_signing_disabled: false,
            create_notebook: true,
        }
    }
}

fn parse_args() -> Config {
    let mut config = Config::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--notebook" | "-n" => {
                config.notebook = args.next();
            }
            "--no-commit-signing" => {
                config.commit_signing_disabled = true;
            }
            "--no-create-notebook" => {
                config.create_notebook = false;
            }
            "--version" => {
                println!("nb-mcp {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--help" | "-h" => {
                eprintln!("nb-mcp: MCP server for nb note-taking");
                eprintln!();
                eprintln!("Usage: nb-mcp [OPTIONS]");
                eprintln!();
                eprintln!("Options:");
                eprintln!("  -n, --notebook <NAME>  Default notebook (overrides NB_MCP_NOTEBOOK)");
                eprintln!("      --no-commit-signing  Disable commit and tag signing");
                eprintln!("                            in notebook repo");
                eprintln!("      --no-create-notebook  Disable automatic notebook creation");
                eprintln!("      --version          Show version");
                eprintln!("  -h, --help             Show this help");
                std::process::exit(0);
            }
            _ => {
                // Ignore unknown args for forward compatibility
            }
        }
    }

    config
}

/// Set up logging to both stderr and a file.
///
/// - Stderr: For immediate feedback during development
/// - File: For persistent logs in `~/.local/state/nb-mcp/{project}--{worktree}.log`
fn setup_logging() {
    let env_filter = EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into());

    // Stderr layer (compact, for console)
    let stderr_layer = fmt::layer().with_writer(std::io::stderr).compact();

    // File layer (with timestamps, for debugging)
    let file_layer = match setup_file_logging() {
        Some((writer, guard)) => {
            // Keep the guard alive by leaking it (file logger lives for process lifetime)
            std::mem::forget(guard);
            Some(fmt::layer().with_writer(writer).with_ansi(false))
        }
        None => None,
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .init();
}

/// Set up file logging, returning the writer and guard.
///
/// Returns `None` if the log directory cannot be created.
fn setup_file_logging() -> Option<(
    tracing_appender::non_blocking::NonBlocking,
    tracing_appender::non_blocking::WorkerGuard,
)> {
    let log_path = paths::get_log_path();
    let log_dir = log_path.parent()?;
    let log_filename = log_path.file_name()?.to_str()?;

    // Ensure log directory exists
    paths::ensure_dir(log_dir).ok()?;

    // Create a non-blocking file appender
    let file_appender = tracing_appender::rolling::never(log_dir, log_filename);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    Some((non_blocking, guard))
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging();

    let log_path = paths::get_log_path();
    tracing::info!(log_file = %log_path.display(), "logging initialized");

    let config = parse_args();
    mcp::run(config).await
}
