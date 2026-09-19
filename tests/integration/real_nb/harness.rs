//! Shared real-`nb` fixture harness (server process, asserts, builders).

use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use nb_api::testing::NbTestEnv;
use serde_json::{Value, json};

pub(crate) struct RealNbServer {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Drop for RealNbServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl RealNbServer {
    pub(crate) fn spawn(env: &NbTestEnv) -> Self {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_nb-mcp"));
        cmd.arg("--notebook")
            .arg(env.notebook())
            .arg("--no-commit-signing")
            .arg("--no-create-notebook")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        env.configure_std(&mut cmd);
        let mut child = cmd.spawn().expect("spawn nb-mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut server = RealNbServer {
            child,
            stdin,
            stdout,
            next_id: 1,
        };
        server.initialize();
        server
    }

    fn initialize(&mut self) {
        let response = self.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "real-nb-test", "version": "0.0.0"}
            }),
        );
        assert!(response.get("result").is_some(), "initialize: {response}");
        self.notification("notifications/initialized", json!({}));
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        writeln!(self.stdin, "{}", serde_json::to_string(&message).unwrap()).unwrap();
        self.stdin.flush().unwrap();
        loop {
            let mut line = String::new();
            let read = self.stdout.read_line(&mut line).unwrap();
            assert!(read > 0, "server stdout closed");
            let response: Value = serde_json::from_str(line.trim()).unwrap();
            if response.get("id") == Some(&json!(id)) {
                return response;
            }
        }
    }

    fn notification(&mut self, method: &str, params: Value) {
        let message = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        writeln!(self.stdin, "{}", serde_json::to_string(&message).unwrap()).unwrap();
        self.stdin.flush().unwrap();
    }

    pub(crate) fn call_first_class(&mut self, tool: &str, args: Value) -> Value {
        self.request("tools/call", json!({"name": tool, "arguments": args}))
    }

    pub(crate) fn call_multiplexed(&mut self, command: &str, args: Value) -> Value {
        self.request(
            "tools/call",
            json!({
                "name": "nb",
                "arguments": {"command": command, "args": args},
            }),
        )
    }

    pub(crate) fn error_text(response: &Value) -> String {
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }

    pub(crate) fn result_json(response: &Value) -> Value {
        let content = &response["result"]["content"][0];
        if let Some(value) = content.get("json") {
            return value.clone();
        }
        let text = content["text"].as_str().unwrap();
        serde_json::from_str(text).unwrap()
    }
}

/// Path of the regression folder inside each fixture. `nb` 7.24.0
/// reserves `archive` as a top-level command (`nb archive
/// <notebook>`). With a fixture whose current notebook is
/// `scratch`, the qualified-folder argv `nb add folder
/// scratch:archive` is misparsed — the resolved
/// `nb-api::add_folder("archive", ...)` invocation produces the
/// `nb command failed: ! Notebook not found: folder` error and the
/// folder is never created. `subfolder` is unambiguous and
/// matches the upstream `show_rejects_folder_selector` regression.
pub(crate) const FOLDER: &str = "subfolder";

pub(crate) fn fresh_env() -> NbTestEnv {
    NbTestEnv::new().expect("real-nb fixture initialization")
}

pub(crate) fn assert_typed_show_error(response: &Value, label: &str) {
    assert_eq!(
        response["result"]["isError"].as_bool(),
        Some(true),
        "[{label}] expected isError, got: {response}"
    );
    let error_text = RealNbServer::error_text(response);
    assert!(
        error_text.contains("subfolder"),
        "[{label}] selector name `subfolder` missing: {error_text}"
    );
    assert!(
        error_text.contains("folder"),
        "[{label}] actual_type `folder` missing: {error_text}"
    );
    assert!(
        error_text.contains("non-textual") || error_text.contains("non-text"),
        "[{label}] non-text classification missing: {error_text}"
    );
    assert!(
        error_text.contains("`show` reads text notes only"),
        "[{label}] show restriction missing: {error_text}"
    );
    assert!(
        error_text.contains("folders") && error_text.contains("list"),
        "[{label}] recovery toward folders/list missing: {error_text}"
    );
}

pub(crate) fn assert_duplicate_h1_error(error_text: &str, label: &str) {
    assert!(
        error_text.contains("`Routing Notes`"),
        "[{label}] title missing: {error_text}"
    );
    assert!(
        error_text.contains("# Routing Notes"),
        "[{label}] duplicate heading missing: {error_text}"
    );
    assert!(
        error_text.contains("duplicate")
            && (error_text.contains("remove the duplicate")
                || error_text.contains("omit the separate")),
        "[{label}] recovery guidance missing: {error_text}"
    );
}

/// Address a note by its fixture-relative path (e.g. `folder/name.md`).
#[allow(dead_code)]
pub(crate) fn target_path(path: &str) -> Value {
    json!({"type": "path", "value": path})
}

/// Create a note through the MCP `add` tool and return the created note's
/// path (from the structured CommitOutcome) plus the response.
pub(crate) fn add_note_via_mcp(
    server: &mut RealNbServer,
    env: &NbTestEnv,
    title: &str,
    content: &str,
) -> Value {
    let response = server.call_first_class(
        "add",
        json!({
            "folder": "session-notes",
            "title": title,
            "content": content,
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        response["result"]["isError"].as_bool(),
        Some(false),
        "add should succeed; got: {response}"
    );
    response
}

/// Commit one new file in the fixture notebook so the worktree returns
/// clean (the native Transaction engine refuses a dirty baseline).
pub(crate) fn commit_file(env: &NbTestEnv, notebook_root: &std::path::Path, rel_path: &str) {
    let mut git = Command::new("git");
    env.configure_std(&mut git);
    git.current_dir(notebook_root);
    git.args(["add", rel_path]);
    let add = git.output().expect("git add");
    assert!(add.status.success(), "git add failed: {:?}", add);

    let mut git = Command::new("git");
    env.configure_std(&mut git);
    git.current_dir(notebook_root);
    git.args(["commit", "-m", "add fragmented fixture"]);
    let commit = git.output().expect("git commit");
    assert!(
        commit.status.success(),
        "git commit failed: {:?} {}",
        commit,
        String::from_utf8_lossy(&commit.stderr)
    );
}

// ---- nb-api 0.4.1 adaptation regressions ----
