use anyhow::Result;

mod mcp;
mod nb;

/// Command-line configuration for the MCP server.
#[derive(Default)]
pub struct Config {
    /// Default notebook (CLI --notebook overrides NB_MCP_NOTEBOOK env var).
    pub notebook: Option<String>,
}

fn parse_args() -> Config {
    let mut config = Config::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--notebook" | "-n" => {
                config.notebook = args.next();
            }
            "--help" | "-h" => {
                eprintln!("nb-mcp: MCP server for nb note-taking");
                eprintln!();
                eprintln!("Usage: nb-mcp [OPTIONS]");
                eprintln!();
                eprintln!("Options:");
                eprintln!("  -n, --notebook <NAME>  Default notebook (overrides NB_MCP_NOTEBOOK)");
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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let config = parse_args();
    mcp::run(config).await
}
