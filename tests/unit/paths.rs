use std::path::PathBuf;

use nb_mcp_server::paths::{ensure_dir, get_log_path};

#[test]
fn log_path_has_expected_structure() {
    let path = get_log_path();
    assert!(path.to_string_lossy().contains("nb-mcp"));
    assert!(path.extension().is_some_and(|ext| ext == "log"));
}

#[test]
fn ensure_dir_creates_missing_directories() {
    let test_dir = PathBuf::from(format!(
        "/tmp/nb-mcp-tests-{}-{}",
        std::process::id(),
        "ensure-dir"
    ));
    if test_dir.exists() {
        std::fs::remove_dir_all(&test_dir).unwrap();
    }
    ensure_dir(&test_dir).unwrap();
    assert!(test_dir.is_dir());
    std::fs::remove_dir_all(&test_dir).unwrap();
}
