use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use tokio::process::Command;

use crate::{Config, nb::NbClient};

pub async fn disable_commit_signing(config: &Config) -> Result<Option<PathBuf>> {
    let nb_client = NbClient::new(config.notebook.as_deref())
        .context("create nb client for commit signing update")?;
    let path = nb_client
        .notebook_path(config.notebook.as_deref())
        .await
        .context("fetch notebook path for commit signing update")?;
    disable_signing_for_path(&path).await.map(Some)
}

async fn disable_signing_for_path(path: &Path) -> Result<PathBuf> {
    let root = resolve_git_root(path).await?;
    apply_signing_config(&root).await?;
    Ok(root)
}

async fn resolve_git_root(path: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .await
        .context("run git rev-parse to resolve notebook repository root")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let message = select_output(&stderr, &stdout);
        return Err(anyhow!(
            "git rev-parse failed for notebook repository: {}",
            message.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let root = stdout.trim();
    if root.is_empty() {
        return Err(anyhow!(
            "git rev-parse returned an empty notebook repository path"
        ));
    }
    Ok(PathBuf::from(root))
}

async fn apply_signing_config(path: &Path) -> Result<()> {
    run_git_config(path, "commit.gpgsign", "false").await?;
    run_git_config(path, "tag.gpgsign", "false").await?;
    Ok(())
}

async fn run_git_config(path: &Path, key: &str, value: &str) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("config")
        .arg(key)
        .arg(value)
        .output()
        .await
        .with_context(|| format!("run git config {key} for notebook repository"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let message = select_output(&stderr, &stdout);
    Err(anyhow!(
        "git config failed for {key} in notebook repository: {}",
        message.trim()
    ))
}

fn select_output<'a>(stderr: &'a str, stdout: &'a str) -> &'a str {
    if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    }
}
