use std::process::Command;

#[test]
fn version_flag_prints_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_nb-mcp"))
        .arg("--version")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        format!("nb-mcp {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_flag_prints_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_nb-mcp"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Usage: nb-mcp [OPTIONS]"));
    assert!(stderr.contains("--no-commit-signing"));
}
