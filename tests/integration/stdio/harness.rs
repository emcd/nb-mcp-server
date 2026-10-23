//! Bash-shim stdio harness (fake `nb` CLI process, JSON-RPC helpers).

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};

pub(crate) const SHIM_DIR: &str = "tests/support";
pub(crate) const TEST_NOTEBOOK: &str = "mcp-stdio-testbook";
pub(crate) const TEMP_TEST_ROOT: &str = ".auxiliary/temporary/tests";
pub(crate) static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) struct ShimEnv {
    pub(crate) root: PathBuf,
    pub(crate) path: String,
}
impl Drop for ShimEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
pub(crate) struct McpProcess {
    pub(crate) child: Child,
    pub(crate) stdin: ChildStdin,
    pub(crate) stdout: BufReader<ChildStdout>,
    pub(crate) next_id: u64,
}
impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub(crate) fn unique_temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(TEMP_TEST_ROOT)
        .join(format!("{label}-{}-{nanos}-{sequence}", std::process::id()))
}

pub(crate) fn shim_env() -> ShimEnv {
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

pub(crate) fn start_server(shim: &ShimEnv) -> McpProcess {
    start_server_with_args(shim, &[])
}

pub(crate) fn start_server_with_args(
    shim: &ShimEnv,
    extra_args: &[&str],
) -> McpProcess {
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
    pub(crate) fn initialize(&mut self) {
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

    pub(crate) fn request(&mut self, method: &str, params: Value) -> Value {
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

    pub(crate) fn notification(&mut self, method: &str, params: Value) {
        self.write_message(json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }));
    }

    pub(crate) fn write_message(&mut self, message: Value) {
        writeln!(self.stdin, "{}", serde_json::to_string(&message).unwrap())
            .unwrap();
        self.stdin.flush().unwrap();
    }

    pub(crate) fn read_message(&mut self) -> Value {
        let mut line = String::new();
        let read = self.stdout.read_line(&mut line).unwrap();
        assert!(read > 0, "server stdout closed");
        serde_json::from_str(line.trim()).unwrap()
    }

    pub(crate) fn call_nb(&mut self, command: &str, args: Value) -> Value {
        self.request(
            "tools/call",
            json!({
                "name": "nb",
                "arguments": {"command": command, "args": args}
            }),
        )
    }

    pub(crate) fn call_help(&mut self, query: &str) -> Value {
        self.request(
            "tools/call",
            json!({
                "name": "help",
                "arguments": {"query": query}
            }),
        )
    }

    pub(crate) fn call_first_class(
        &mut self,
        tool: &str,
        args: Value,
    ) -> Value {
        self.request(
            "tools/call",
            json!({
                "name": tool,
                "arguments": args
            }),
        )
    }

    pub(crate) fn list_tools(&mut self) -> Value {
        self.request("tools/list", json!({}))
    }
}

pub(crate) fn tool_text(response: &Value) -> String {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

pub(crate) fn tool_json(response: &Value) -> Value {
    let content = &response["result"]["content"][0];
    if let Some(value) = content.get("json") {
        return value.clone();
    }
    let text = content["text"].as_str().unwrap();
    serde_json::from_str(text).unwrap()
}

pub(crate) fn is_tool_error(response: &Value) -> bool {
    response["result"]["isError"].as_bool().unwrap_or(false)
}

pub(crate) fn is_protocol_error(response: &Value) -> bool {
    response["error"].is_object()
}

pub(crate) fn is_rejection(response: &Value) -> bool {
    is_tool_error(response) || is_protocol_error(response)
}

// Dispatch and validation regressions (never reach a notebook).

pub(crate) fn assert_passthrough_exact(
    surface: &str,
    response: &Value,
    expected: &str,
) {
    assert!(
        !is_rejection(response),
        "[{surface}] should pass through shim output, got: {response}"
    );
    let output = tool_text(response);
    assert_eq!(
        output, expected,
        "[{surface}] should pass the exact shim output through (including trailing newline); got: {output:?}"
    );
}

pub(crate) fn help_description(
    server: &mut McpProcess,
    query: &str,
) -> String {
    let response = server.call_help(query);
    let help = tool_json(&response);
    help["description"].as_str().unwrap_or_default().to_string()
}

// Empty-list passthrough: the MCP layer must pass `list` and `folders`
// output through verbatim. Sanitization of `nb` native CLI hint blocks
// is the responsibility of `nb-api`, exercised at the API layer.

// `tests/support/nb` echoes `<verb> ${notebook}\n` (echo appends a
// trailing newline) for `list` and `folders`. The MCP layer must pass
// the bytes through unchanged on every surface. Asserts use the exact
// shim output, not `starts_with`, so appended, truncated, or
// reformatted output fails the regression.
// Empty-list passthrough: the MCP layer must pass `list` and `folders`
// output through verbatim. Sanitization of `nb` native CLI hint blocks
// is the responsibility of `nb-api`, exercised at the API layer.

// `tests/support/nb` echoes `<verb> ${notebook}\n` (echo appends a
// trailing newline) for `list` and `folders`. The MCP layer must pass
// the bytes through unchanged on every surface. Asserts use the exact
// shim output, not `starts_with`, so appended, truncated, or
// reformatted output fails the regression.
pub(crate) const EXPECTED_LIST: &str = "listed mcp-stdio-testbook\n";
pub(crate) const EXPECTED_FOLDERS: &str = "folders mcp-stdio-testbook\n";
