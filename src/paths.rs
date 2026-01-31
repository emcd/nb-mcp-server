//! XDG-compliant path detection for logging.
//!
//! Log files are placed in `$XDG_STATE_HOME/nb-mcp/` (typically `~/.local/state/nb-mcp/`).
//! When running inside a Git repository, logs are named after the project and worktree
//! to avoid collisions between multiple MCP server instances.

use std::{path::PathBuf, process::Command, sync::OnceLock};

/// Cached log path (computed once per process).
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Get the log file path for this MCP server instance.
///
/// Format: `{XDG_STATE_HOME}/nb-mcp/{project}--{worktree}.log`
///
/// If not in a Git repository, uses a generic name.
pub fn get_log_path() -> PathBuf {
    LOG_PATH.get_or_init(compute_log_path).clone()
}

/// Compute the log path based on Git repository detection.
fn compute_log_path() -> PathBuf {
    let state_dir = xdg_state_home().join("nb-mcp");

    // Try to get a unique name from Git info
    let log_name = match detect_git_info() {
        Some((project, worktree)) => {
            if project == worktree {
                format!("{}.log", project)
            } else {
                format!("{}--{}.log", project, worktree)
            }
        }
        None => "nb-mcp.log".to_string(),
    };

    state_dir.join(log_name)
}

/// Detect Git project name and worktree basename.
///
/// Returns `(project_name, worktree_basename)` where:
/// - `project_name` is derived from the master repo directory name
/// - `worktree_basename` is the current worktree directory name
///
/// These are the same for non-worktree repos.
fn detect_git_info() -> Option<(String, String)> {
    // Get current worktree root
    let current_root = git_rev_parse(&["--show-toplevel"])?;

    // Get common git directory (may be relative)
    let git_common_dir = git_rev_parse(&["--git-common-dir"])?;

    // Resolve git_common_dir relative to current_root if it's relative
    let git_common_dir = if git_common_dir.is_relative() {
        current_root.join(&git_common_dir)
    } else {
        git_common_dir
    };

    // Canonicalize to resolve any .. or symlinks
    let git_common_dir = git_common_dir.canonicalize().ok()?;

    // Master root is parent of git_common_dir when it ends with .git
    let master_root = if git_common_dir.file_name().is_some_and(|n| n == ".git") {
        git_common_dir.parent()?.to_path_buf()
    } else {
        // Bare repo or unusual structure - fall back to current
        current_root.clone()
    };

    // Extract directory names
    let project_name = master_root
        .file_name()
        .and_then(|n| n.to_str())
        .map(sanitize_name)?;

    let worktree_name = current_root
        .file_name()
        .and_then(|n| n.to_str())
        .map(sanitize_name)?;

    Some((project_name, worktree_name))
}

/// Run `git rev-parse` with the given arguments and return the output as a path.
fn git_rev_parse(args: &[&str]) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse"])
        .args(args)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let path = stdout.trim();

    if path.is_empty() {
        return None;
    }

    Some(PathBuf::from(path))
}

/// Sanitize a name for use in a filename.
///
/// Replaces problematic characters with dashes.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Resolve the XDG state home directory.
///
/// Returns `$XDG_STATE_HOME` if set, otherwise `$HOME/.local/state`.
fn xdg_state_home() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_STATE_HOME") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".local/state")
}

/// Ensure a directory exists, creating it if necessary.
pub fn ensure_dir(path: &std::path::Path) -> std::io::Result<()> {
    if !path.is_dir() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("my-project"), "my-project");
        assert_eq!(sanitize_name("my_project"), "my_project");
        assert_eq!(sanitize_name("my project"), "my-project");
        assert_eq!(sanitize_name("my/project"), "my-project");
    }

    #[test]
    fn test_log_path_has_expected_structure() {
        let path = get_log_path();
        assert!(path.to_string_lossy().contains("nb-mcp"));
        assert!(path.extension().is_some_and(|e| e == "log"));
    }
}
