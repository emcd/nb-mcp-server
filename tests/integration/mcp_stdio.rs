#![cfg(unix)]

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};

const SHIM_DIR: &str = "tests/support";
const TEST_NOTEBOOK: &str = "mcp-stdio-testbook";
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

struct McpProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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
    let root = unique_temp_root("mcp-stdio");
    let parent = root.parent().unwrap();
    fs::create_dir_all(parent).unwrap();
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(root.join("notebooks").join(TEST_NOTEBOOK)).unwrap();
    fs::create_dir_all(root.join("notebooks").join("other-team")).unwrap();
    let shim_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SHIM_DIR);
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}:{inherited_path}", shim_dir.display());
    ShimEnv { root, path }
}

fn start_server(shim: &ShimEnv) -> McpProcess {
    start_server_with_args(shim, &[])
}

fn start_server_with_args(shim: &ShimEnv, extra_args: &[&str]) -> McpProcess {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nb-mcp"))
        .arg("--notebook")
        .arg(TEST_NOTEBOOK)
        .arg("--no-commit-signing")
        .arg("--no-create-notebook")
        .args(extra_args)
        .env("NB_SHIM_ROOT", &shim.root)
        .env("PATH", &shim.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let mut process = McpProcess {
        child,
        stdin,
        stdout,
        next_id: 1,
    };
    process.initialize();
    process
}

impl McpProcess {
    fn initialize(&mut self) {
        let response = self.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "nb-mcp-test", "version": "0.0.0"}
            }),
        );
        assert!(response.get("result").is_some(), "response: {response}");
        self.notification("notifications/initialized", json!({}));
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        self.write_message(message);
        loop {
            let response = self.read_message();
            if response.get("id") == Some(&json!(id)) {
                return response;
            }
        }
    }

    fn notification(&mut self, method: &str, params: Value) {
        self.write_message(json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }));
    }

    fn write_message(&mut self, message: Value) {
        writeln!(self.stdin, "{}", serde_json::to_string(&message).unwrap()).unwrap();
        self.stdin.flush().unwrap();
    }

    fn read_message(&mut self) -> Value {
        let mut line = String::new();
        let read = self.stdout.read_line(&mut line).unwrap();
        assert!(read > 0, "server stdout closed");
        serde_json::from_str(line.trim()).unwrap()
    }

    fn call_nb(&mut self, command: &str, args: Value) -> Value {
        self.request(
            "tools/call",
            json!({
                "name": "nb",
                "arguments": {"command": command, "args": args}
            }),
        )
    }

    fn call_help(&mut self, query: &str) -> Value {
        self.request(
            "tools/call",
            json!({
                "name": "help",
                "arguments": {"query": query}
            }),
        )
    }
}

fn tool_text(response: &Value) -> String {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

fn tool_json(response: &Value) -> Value {
    let content = &response["result"]["content"][0];
    if let Some(value) = content.get("json") {
        return value.clone();
    }
    let text = content["text"].as_str().unwrap();
    serde_json::from_str(text).unwrap()
}

fn is_tool_error(response: &Value) -> bool {
    response["result"]["isError"].as_bool().unwrap_or(false)
}

#[test]
fn nb_tool_rejects_non_object_args_payloads() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    for args in [
        json!("{\"value\":\"ok\"}"),
        json!(["ok"]),
        json!(42),
        json!(true),
    ] {
        let response = server.call_nb("nb.status", args);
        assert!(is_tool_error(&response), "response: {response}");
        assert!(tool_text(&response).contains("args must be a JSON object"));
    }
}

#[test]
fn nb_tool_defaults_null_and_empty_args_objects() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    for args in [Value::Null, json!({})] {
        let response = server.call_nb("nb.status", args);
        assert!(tool_text(&response).contains("unsupported nb command"));
    }
}

#[test]
fn nb_tool_rejects_unknown_command_args() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let add_response = server.call_nb(
        "nb.add",
        json!({
            "selector": "coordination/general",
            "title": "Wrong field",
            "content": "Should fail."
        }),
    );
    assert!(is_tool_error(&add_response), "response: {add_response}");
    assert!(tool_text(&add_response).contains("unknown field `selector`"));
    let status_response = server.call_nb("nb.status", json!({"folder": "coordination"}));
    assert!(
        is_tool_error(&status_response),
        "response: {status_response}"
    );
    assert!(tool_text(&status_response).contains("unknown field `folder`"));
}

#[test]
fn nb_tool_reports_folder_required_before_running_nb() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_nb(
        "nb.add",
        json!({"title": "No Folder", "content": "Should fail."}),
    );
    assert!(is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("require `folder`"));
}

#[test]
fn nb_tool_allows_top_level_notes_when_configured() {
    let shim = shim_env();
    let mut server = start_server_with_args(&shim, &["--allow-top-level-notes"]);
    let response = server.call_nb(
        "nb.add",
        json!({"title": "No Folder", "content": "Allowed by config."}),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("Added:"));
}

#[test]
fn nb_tool_warns_after_non_default_notebook_writes() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_nb(
        "nb.add",
        json!({
            "notebook": "other-team",
            "folder": "coordination",
            "title": "Cross team note",
            "content": "Should warn."
        }),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("Warning: wrote to notebook `other-team`"));
}

#[test]
fn nb_tool_rejects_selector_like_routing_fields() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let invalid_notebook = server.call_nb(
        "nb.add",
        json!({
            "notebook": "agentmux:coordination",
            "folder": "coordination",
            "title": "Bad notebook",
            "content": "Should fail."
        }),
    );
    assert!(
        is_tool_error(&invalid_notebook),
        "response: {invalid_notebook}"
    );
    assert!(tool_text(&invalid_notebook).contains("Invalid `notebook`"));
    let invalid_folder = server.call_nb(
        "nb.add",
        json!({
            "notebook": "agentmux",
            "folder": "agentmux:coordination",
            "title": "Bad folder",
            "content": "Should fail."
        }),
    );
    assert!(is_tool_error(&invalid_folder), "response: {invalid_folder}");
    assert!(tool_text(&invalid_folder).contains("Invalid folder path"));
}

#[test]
fn nb_tool_honors_copied_selectors_and_rejects_conflicts() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_nb(
        "nb.show",
        json!({"id": format!("{TEST_NOTEBOOK}:todos/mcp/35")}),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains(&format!("shown {TEST_NOTEBOOK}:todos/mcp/35")));
    let conflict = server.call_nb(
        "nb.show",
        json!({"notebook": "other", "id": format!("{TEST_NOTEBOOK}:todos/mcp/35")}),
    );
    assert!(is_tool_error(&conflict), "response: {conflict}");
    assert!(tool_text(&conflict).contains("ambiguous selector"));
}

#[test]
fn help_tool_describes_routing_rules() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_help("nb");
    let help = tool_json(&response);
    let hints = help["shape_hints"].as_array().unwrap();
    assert!(
        hints
            .iter()
            .any(|hint| hint.as_str().unwrap().contains("bare notebook name"))
    );
    assert!(
        hints
            .iter()
            .any(|hint| hint.as_str().unwrap().contains("copied id/selector"))
    );
}
