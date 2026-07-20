use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use tokio::process::Command;

use crate::Config;

pub async fn disable_commit_signing(config: &Config) -> Result<Option<PathBuf>> {
    let nb_config = config.to_nb_api_config();
    let nb_client =
        nb_api::NbClient::new(&nb_config).context("create nb client for commit signing update")?;
    let path = nb_client
        .show_notebook_path(nb_config.notebook.as_deref())
        .await
        .context("fetch notebook path for commit signing update")?;
    disable_signing_for_path(&path).await.map(Some)
}

async fn disable_signing_for_path(path: &Path) -> Result<PathBuf> {
    let root = resolve_git_root(path).await?;
    ensure_notebook_git_root(path, &root)?;
    apply_signing_config(&root).await?;
    Ok(root)
}

async fn resolve_git_root(path: &Path) -> Result<PathBuf> {
    let mut command = Command::new("git");
    scrub_git_env(&mut command);
    let output = command
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

fn ensure_notebook_git_root(path: &Path, root: &Path) -> Result<()> {
    let path = path.canonicalize().with_context(|| {
        format!(
            "canonicalize notebook path before applying signing config: {}",
            path.display()
        )
    })?;
    let root = root.canonicalize().with_context(|| {
        format!(
            "canonicalize notebook git root before applying signing config: {}",
            root.display()
        )
    })?;
    if path == root {
        return Ok(());
    }
    Err(anyhow!(
        "refusing to disable signing outside notebook repository: notebook path {} resolves to ancestor git repository {}",
        path.display(),
        root.display()
    ))
}

async fn apply_signing_config(path: &Path) -> Result<()> {
    run_git_config(path, "commit.gpgsign", "false").await?;
    run_git_config(path, "tag.gpgsign", "false").await?;
    Ok(())
}

async fn run_git_config(path: &Path, key: &str, value: &str) -> Result<()> {
    let mut command = Command::new("git");
    scrub_git_env(&mut command);
    let output = command
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

fn scrub_git_env(command: &mut Command) {
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
}

fn select_output<'a>(stderr: &'a str, stdout: &'a str) -> &'a str {
    if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    }
}
