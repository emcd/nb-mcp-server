#![cfg(unix)]

use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const SHIM_DIR: &str = "tests/support";
const TEST_NOTEBOOK: &str = "shim-signing-testbook";
const TEMP_TEST_ROOT: &str = ".auxiliary/temporary/tests";

struct ShimEnv {
    root: PathBuf,
    path: String,
}

impl Drop for ShimEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn unique_temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(TEMP_TEST_ROOT)
        .join(format!("{label}-{}-{nanos}", std::process::id()))
}

fn shim_env() -> ShimEnv {
    let root = unique_temp_root("nb-shim");
    let parent = root.parent().unwrap();
    fs::create_dir_all(parent).unwrap();
    let shim_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SHIM_DIR);
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}:{inherited_path}", shim_dir.display());
    ShimEnv { root, path }
}

#[test]
fn show_paths_with_no_commit_signing_initializes_notebook_without_signing() {
    let shim = shim_env();

    let output = Command::new(env!("CARGO_BIN_EXE_nb-mcp"))
        .arg("--show-paths")
        .arg("--notebook")
        .arg(TEST_NOTEBOOK)
        .arg("--no-commit-signing")
        .env("NB_SHIM_ROOT", &shim.root)
        .env("PATH", &shim.path)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "stderr: {stderr}");

    let notebook_path = shim.root.join("notebooks").join(TEST_NOTEBOOK);
    assert!(notebook_path.is_dir());
    assert!(stdout.contains(&format!("notebook_path: {}", notebook_path.display())));
    assert!(shim.root.join("signing-overrides-detected").is_file());

    let calls = fs::read_to_string(shim.root.join("calls.log")).unwrap();
    assert!(calls.contains(&format!("notebooks add {TEST_NOTEBOOK}")));
}

#[test]
fn show_paths_without_no_commit_signing_fails_on_initialization() {
    let shim = shim_env();

    let output = Command::new(env!("CARGO_BIN_EXE_nb-mcp"))
        .arg("--show-paths")
        .arg("--notebook")
        .arg(TEST_NOTEBOOK)
        .env("NB_SHIM_ROOT", &shim.root)
        .env("PATH", &shim.path)
        .output()
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !output.status.success(),
        "expected failure when signing overrides are absent"
    );
    assert!(
        stderr.contains("signing required"),
        "unexpected stderr: {stderr}"
    );
    assert!(!shim.root.join("notebooks").join(TEST_NOTEBOOK).is_dir());
    assert!(!shim.root.join("signing-overrides-detected").exists());

    let calls = fs::read_to_string(shim.root.join("calls.log")).unwrap();
    assert!(calls.contains(&format!("notebooks add {TEST_NOTEBOOK}")));
}
