//! Git repository detection helpers.
//!
//! Shared between nb-api and nb-mcp-server (paths.rs).

use std::path::PathBuf;

/// Derive the notebook name from the current Git repository.
///
/// Returns the directory name of the master repository (not the worktree).
/// Used as a fallback when no explicit notebook name is configured.
pub fn derive_git_notebook_name() -> Option<String> {
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

/// Run `git rev-parse` with the given arguments and return the output as a path.
///
/// Returns `None` if git is not available, the command fails, or the output is empty.
pub fn git_rev_parse(args: &[&str]) -> Option<PathBuf> {
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
