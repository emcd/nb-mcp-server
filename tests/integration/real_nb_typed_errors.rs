#![cfg(unix)]

use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use nb_api::testing::NbTestEnv;
use serde_json::{Value, json};

struct RealNbServer {
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
    fn spawn(env: &NbTestEnv) -> Self {
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

    fn call_first_class(&mut self, tool: &str, args: Value) -> Value {
        self.request("tools/call", json!({"name": tool, "arguments": args}))
    }

    fn call_multiplexed(&mut self, command: &str, args: Value) -> Value {
        self.request(
            "tools/call",
            json!({
                "name": "nb",
                "arguments": {"command": command, "args": args},
            }),
        )
    }

    fn error_text(response: &Value) -> String {
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string()
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
const FOLDER: &str = "subfolder";

fn fresh_env() -> NbTestEnv {
    NbTestEnv::new().expect("real-nb fixture initialization")
}

fn assert_typed_show_error(response: &Value, label: &str) {
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

fn assert_duplicate_h1_error(error_text: &str, label: &str) {
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

#[test]
fn direct_show_on_folder_selector_returns_actionable_typed_error() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    let mkdir_response =
        server.call_first_class("mkdir", json!({"path": FOLDER, "notebook": env.notebook()}));
    assert_eq!(
        mkdir_response["result"]["isError"].as_bool(),
        Some(false),
        "mkdir should succeed against a fresh fixture; got: {mkdir_response}"
    );

    let response =
        server.call_first_class("show", json!({"id": FOLDER, "notebook": env.notebook()}));
    assert_typed_show_error(&response, "direct");
}

#[test]
fn multiplexed_show_on_folder_selector_returns_actionable_typed_error() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    let mkdir_response =
        server.call_first_class("mkdir", json!({"path": FOLDER, "notebook": env.notebook()}));
    assert_eq!(
        mkdir_response["result"]["isError"].as_bool(),
        Some(false),
        "mkdir should succeed; got: {mkdir_response}"
    );

    let response =
        server.call_multiplexed("nb.show", json!({"id": FOLDER, "notebook": env.notebook()}));
    assert_typed_show_error(&response, "multiplexed");
}

#[test]
fn show_folder_typed_error_is_parity_across_surfaces() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    let mkdir_response =
        server.call_first_class("mkdir", json!({"path": FOLDER, "notebook": env.notebook()}));
    assert_eq!(
        mkdir_response["result"]["isError"].as_bool(),
        Some(false),
        "mkdir should succeed; got: {mkdir_response}"
    );

    let payload = json!({"id": FOLDER, "notebook": env.notebook()});
    let direct = server.call_first_class("show", payload.clone());
    let multiplexed = server.call_multiplexed("nb.show", payload);
    let direct_text = RealNbServer::error_text(&direct);
    let multiplexed_text = RealNbServer::error_text(&multiplexed);
    assert_eq!(
        direct_text, multiplexed_text,
        "direct and multiplexed folder-show errors must produce identical wording"
    );
}

#[test]
fn add_duplicate_h1_typed_error_is_parity_across_surfaces() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    let payload = json!({
        "title": "Routing Notes",
        "content": "# Routing Notes\n\nDetails here.",
    });
    let direct = server.call_first_class("add", payload.clone());
    let multiplexed = server.call_multiplexed("nb.add", payload);
    let direct_text = RealNbServer::error_text(&direct);
    let multiplexed_text = RealNbServer::error_text(&multiplexed);
    assert_eq!(
        direct_text, multiplexed_text,
        "direct and multiplexed duplicate-h1 errors must produce identical wording"
    );
    assert_duplicate_h1_error(&direct_text, "direct");
    assert_duplicate_h1_error(&multiplexed_text, "multiplexed");
}
