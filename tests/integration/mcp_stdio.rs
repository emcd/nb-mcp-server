#![cfg(unix)]

//! Stdio transport, schema, help, dispatch, and pure-CLI-read regressions.
//!
//! These tests run against the bash shim (`tests/support/nb`), which fakes
//! the `nb` CLI. They are valid only for operations that still invoke the
//! `nb` CLI (pure reads: `status`, `notebooks`, `list`, `search`, `folders`,
//! non-recursive `tasks`) and for schema/help/dispatch behavior that never
//! reaches a notebook. Mutations (`add`, `todo`, `bookmark`, `mkdir`,
//! `delete`, `move`, `do`, `undo`), `show`, `import`, and recursive `tasks`
//! run through nb-api 0.3's native `Transaction` engine / native file reads
//! and are covered by the real-`nb` integration suite
//! (`real_nb_typed_errors.rs`), not the shim.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};

const SHIM_DIR: &str = "tests/support";
const TEST_NOTEBOOK: &str = "mcp-stdio-testbook";
const TEMP_TEST_ROOT: &str = ".auxiliary/temporary/tests";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(TEMP_TEST_ROOT)
        .join(format!("{label}-{}-{nanos}-{sequence}", std::process::id()))
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

    fn call_first_class(&mut self, tool: &str, args: Value) -> Value {
        self.request(
            "tools/call",
            json!({
                "name": tool,
                "arguments": args
            }),
        )
    }

    fn list_tools(&mut self) -> Value {
        self.request("tools/list", json!({}))
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

fn is_protocol_error(response: &Value) -> bool {
    response["error"].is_object()
}

fn is_rejection(response: &Value) -> bool {
    is_tool_error(response) || is_protocol_error(response)
}

// Dispatch and validation regressions (never reach a notebook).

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
        assert!(tool_text(&response).contains("status"));
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
    let calls = fs::read_to_string(shim.root.join("calls.log")).unwrap_or_default();
    assert!(
        !calls.lines().any(|line| line.contains("add")),
        "folder-required must be reported before invoking nb; calls: {calls}"
    );
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

// First-class pure-CLI-read tool tests.

#[test]
fn first_class_search_accepts_array_queries() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class("search", json!({"queries": ["test", "search"]}));
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("searched"));
}

#[test]
fn first_class_search_requires_non_empty_queries() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class("search", json!({"queries": []}));
    assert!(is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("non-empty array"));
}

#[test]
fn first_class_list_accepts_array_tags() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "list",
        json!({
            "folder": "procedures",
            "tags": ["tag1"]
        }),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    // Should list notes (may be empty but no error).
    assert!(tool_text(&response).contains("listed"));
}

#[test]
fn first_class_search_rejects_unknown_args() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "search",
        json!({"queries": ["test"], "unknown_field": "value"}),
    );
    // MCP framework returns a JSON-RPC error for deserialization failures.
    assert!(response.get("error").is_some(), "response: {response}");
    let error_msg = response["error"]["message"].as_str().unwrap();
    assert!(
        error_msg.contains("unknown field") || error_msg.contains("failed to deserialize"),
        "unexpected error: {error_msg}"
    );
}

#[test]
fn first_class_add_requires_folder_by_default() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "add",
        json!({"title": "No folder", "content": "Should fail."}),
    );
    assert!(is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("require `folder`"));
}

#[test]
fn help_tool_describes_first_class_tools() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_help("nb");
    let help = tool_json(&response);
    let first_class = help["first_class_tools"].as_array().unwrap();
    assert_eq!(first_class.len(), 23);
    let tool_names: Vec<&str> = first_class
        .iter()
        .map(|t| t["tool"].as_str().unwrap())
        .collect();
    for tool in [
        "add",
        "search",
        "todo",
        "list",
        "status",
        "notebooks",
        "show",
        "delete",
        "move",
        "do",
        "undo",
        "tasks",
        "bookmark",
        "folders",
        "mkdir",
        "import",
        "replace_note_body",
        "edit_note_substring",
        "edit_note_lines",
        "retitle_note",
        "edit_note_tags",
        "show_note_lines",
        "search_note_lines",
    ] {
        assert!(tool_names.contains(&tool), "missing {tool}");
    }
    assert!(
        !tool_names.contains(&"edit"),
        "legacy edit tool must not be listed"
    );
}

#[test]
fn help_tool_provides_first_class_tool_schemas() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    for tool in [
        "add",
        "search",
        "todo",
        "list",
        "status",
        "notebooks",
        "show",
        "delete",
        "move",
        "do",
        "undo",
        "tasks",
        "bookmark",
        "folders",
        "mkdir",
        "import",
        "replace_note_body",
        "edit_note_substring",
        "edit_note_lines",
        "retitle_note",
        "edit_note_tags",
        "show_note_lines",
        "search_note_lines",
    ] {
        let response = server.call_help(tool);
        let help = tool_json(&response);
        assert!(help["args_schema"].is_object(), "tool: {tool}");
    }
}

#[test]
fn tools_list_exposes_first_class_tools() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.list_tools();
    let tools = response["result"]["tools"].as_array().unwrap();
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for tool in [
        "add",
        "search",
        "todo",
        "list",
        "status",
        "notebooks",
        "show",
        "delete",
        "move",
        "do",
        "undo",
        "tasks",
        "bookmark",
        "folders",
        "mkdir",
        "import",
        "replace_note_body",
        "edit_note_substring",
        "edit_note_lines",
        "retitle_note",
        "edit_note_tags",
        "show_note_lines",
        "search_note_lines",
    ] {
        assert!(
            tool_names.contains(&tool),
            "tool {tool} not found in {tool_names:?}"
        );
    }
    assert!(
        !tool_names.contains(&"edit"),
        "legacy edit tool must not be exposed"
    );
}

#[test]
fn tools_list_first_class_schemas_have_array_fields() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.list_tools();
    let tools = response["result"]["tools"].as_array().unwrap();

    let find_tool = |name: &str| -> Value {
        tools
            .iter()
            .find(|t| t["name"].as_str().unwrap() == name)
            .cloned()
            .unwrap_or_else(|| panic!("tool {name} not found"))
    };

    // search: queries must be array type
    let search_tool = find_tool("search");
    let search_schema = &search_tool["inputSchema"];
    let queries_type = search_schema["properties"]["queries"]["type"]
        .as_str()
        .unwrap();
    assert_eq!(queries_type, "array", "search.queries should be array");

    // add: tags must be array type
    let add_tool = find_tool("add");
    let add_schema = &add_tool["inputSchema"];
    let tags_type = add_schema["properties"]["tags"]["type"].as_str().unwrap();
    assert_eq!(tags_type, "array", "add.tags should be array");

    // todo: tasks and tags must be array type
    let todo_tool = find_tool("todo");
    let todo_schema = &todo_tool["inputSchema"];
    let tasks_type = todo_schema["properties"]["tasks"]["type"].as_str().unwrap();
    assert_eq!(tasks_type, "array", "todo.tasks should be array");
    let todo_tags_type = todo_schema["properties"]["tags"]["type"].as_str().unwrap();
    assert_eq!(todo_tags_type, "array", "todo.tags should be array");

    // list: tags must be array type
    let list_tool = find_tool("list");
    let list_schema = &list_tool["inputSchema"];
    let list_tags_type = list_schema["properties"]["tags"]["type"].as_str().unwrap();
    assert_eq!(list_tags_type, "array", "list.tags should be array");
}

#[test]
fn tools_list_optional_scalars_are_plain_types_not_nullable_unions() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.list_tools();
    let tools = response["result"]["tools"].as_array().unwrap();

    let find_tool = |name: &str| -> Value {
        tools
            .iter()
            .find(|t| t["name"].as_str().unwrap() == name)
            .cloned()
            .unwrap_or_else(|| panic!("tool {name} not found"))
    };

    // add: title, folder, notebook must be plain string (not anyOf/oneOf nullable union)
    let add_tool = find_tool("add");
    let add_schema = &add_tool["inputSchema"];
    let title_prop = &add_schema["properties"]["title"];
    assert_eq!(
        title_prop["type"].as_str().unwrap(),
        "string",
        "add.title should be plain string"
    );
    assert!(
        title_prop.get("anyOf").is_none() && title_prop.get("oneOf").is_none(),
        "add.title should not be a nullable union"
    );
    let folder_prop = &add_schema["properties"]["folder"];
    assert_eq!(
        folder_prop["type"].as_str().unwrap(),
        "string",
        "add.folder should be plain string"
    );
    assert!(
        folder_prop.get("anyOf").is_none() && folder_prop.get("oneOf").is_none(),
        "add.folder should not be a nullable union"
    );
    let notebook_prop = &add_schema["properties"]["notebook"];
    assert_eq!(
        notebook_prop["type"].as_str().unwrap(),
        "string",
        "add.notebook should be plain string"
    );
    assert!(
        notebook_prop.get("anyOf").is_none() && notebook_prop.get("oneOf").is_none(),
        "add.notebook should not be a nullable union"
    );

    // list: limit must be plain integer (not anyOf/oneOf nullable union)
    let list_tool = find_tool("list");
    let list_schema = &list_tool["inputSchema"];
    let limit_prop = &list_schema["properties"]["limit"];
    assert_eq!(
        limit_prop["type"].as_str().unwrap(),
        "integer",
        "list.limit should be plain integer"
    );
    assert!(
        limit_prop.get("anyOf").is_none() && limit_prop.get("oneOf").is_none(),
        "list.limit should not be a nullable union"
    );

    // todo: description must be plain string (not anyOf/oneOf nullable union)
    let todo_tool = find_tool("todo");
    let todo_schema = &todo_tool["inputSchema"];
    let description_prop = &todo_schema["properties"]["description"];
    assert_eq!(
        description_prop["type"].as_str().unwrap(),
        "string",
        "todo.description should be plain string"
    );
    assert!(
        description_prop.get("anyOf").is_none() && description_prop.get("oneOf").is_none(),
        "todo.description should not be a nullable union"
    );

    // replace_note_body: fingerprint must be plain string (no nullable union)
    let rnb = find_tool("replace_note_body");
    let rnb_schema = &rnb["inputSchema"];
    let fp_prop = &rnb_schema["properties"]["fingerprint"];
    assert_eq!(
        fp_prop["type"].as_str().unwrap(),
        "string",
        "replace_note_body.fingerprint should be plain string"
    );
    assert!(
        fp_prop.get("anyOf").is_none() && fp_prop.get("oneOf").is_none(),
        "replace_note_body.fingerprint should not be a nullable union"
    );
    // target must be present (NoteTarget tagged union, not nullable)
    assert!(
        rnb_schema["properties"]["target"].is_object(),
        "replace_note_body.target should be an object schema"
    );
    let required = rnb_schema["required"].as_array().unwrap();
    for field in ["target", "new_body", "fingerprint"] {
        assert!(
            required.iter().any(|r| r.as_str() == Some(field)),
            "replace_note_body should require {field}"
        );
    }
}

// Pure-CLI-read first-class tools.

#[test]
fn first_class_status_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class("status", json!({}));
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("status"));
}

#[test]
fn first_class_notebooks_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class("notebooks", json!({}));
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("notebooks"));
}

#[test]
fn first_class_tasks_non_recursive_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class("tasks", json!({"recursive": false}));
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("tasks"));
}

#[test]
fn first_class_folders_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class("folders", json!({}));
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("folders"));
}

#[test]
fn first_class_tasks_status_schema_is_plain_type() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.list_tools();
    let tools = response["result"]["tools"].as_array().unwrap();
    let tasks_tool = tools
        .iter()
        .find(|t| t["name"].as_str().unwrap() == "tasks")
        .unwrap();
    let schema = &tasks_tool["inputSchema"];
    let status_prop = &schema["properties"]["status"];
    // status should be a plain string enum or $ref, not a nullable union
    assert!(
        status_prop.get("anyOf").is_none() && status_prop.get("oneOf").is_none(),
        "tasks.status should not be a nullable union"
    );
    // status should be a string type or $ref to TaskStatus
    let is_string_type = status_prop["type"].as_str() == Some("string");
    let has_ref = status_prop["$ref"].is_string();
    assert!(
        is_string_type || has_ref,
        "tasks.status should be string type or $ref, got: {status_prop}"
    );
    // status should not be in required array
    let required = schema["required"].as_array();
    if let Some(req) = required {
        assert!(
            !req.iter().any(|r| r.as_str() == Some("status")),
            "tasks.status should not be required"
        );
    }
}

// Cross-surface equivalence for pure-CLI reads.

#[test]
fn cross_surface_status_equivalence() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    // Read-only: both should return status output
    let multiplexed = server.call_nb("nb.status", json!({}));
    let first_class = server.call_first_class("status", json!({}));
    assert!(!is_tool_error(&multiplexed), "multiplexed: {multiplexed}");
    assert!(!is_tool_error(&first_class), "first_class: {first_class}");
    assert_eq!(tool_text(&multiplexed), tool_text(&first_class));
}

#[test]
fn cross_surface_list_equivalence() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    // Read-only: both should return list output
    let multiplexed = server.call_nb("nb.list", json!({}));
    let first_class = server.call_first_class("list", json!({}));
    assert!(!is_tool_error(&multiplexed), "multiplexed: {multiplexed}");
    assert!(!is_tool_error(&first_class), "first_class: {first_class}");
    assert_eq!(tool_text(&multiplexed), tool_text(&first_class));
}

// Removed `edit` surface regressions.

#[test]
fn nb_edit_subcommand_is_rejected_with_recovery_guidance() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_nb(
        "nb.edit",
        json!({
            "id": format!("{TEST_NOTEBOOK}:session-notes/test.md"),
            "content": "Updated content.",
            "mode": "overwrite",
        }),
    );
    assert!(is_tool_error(&response), "response: {response}");
    let text = tool_text(&response);
    assert!(
        text.contains("removed"),
        "nb.edit rejection should mention removal, got: {text}"
    );
    for tool in [
        "replace_note_body",
        "edit_note_substring",
        "edit_note_lines",
        "retitle_note",
        "edit_note_tags",
    ] {
        assert!(
            text.contains(tool),
            "nb.edit rejection should name {tool}, got: {text}"
        );
    }
}

#[test]
fn direct_only_tools_have_no_multiplexed_alias() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    for tool in [
        "replace_note_body",
        "edit_note_substring",
        "edit_note_lines",
        "retitle_note",
        "edit_note_tags",
        "show_note_lines",
        "search_note_lines",
    ] {
        let response = server.call_nb(tool, json!({}));
        assert!(is_tool_error(&response), "{tool}: {response}");
        let text = tool_text(&response);
        assert!(
            text.contains("direct-only"),
            "{tool} rejection should say direct-only, got: {text}"
        );
        assert!(
            text.contains(tool),
            "{tool} rejection should name the direct tool, got: {text}"
        );
    }
}

// Empty-list passthrough: the MCP layer must pass `list` and `folders`
// output through verbatim. Sanitization of `nb` native CLI hint blocks
// is the responsibility of `nb-api`, exercised at the API layer.

// `tests/support/nb` echoes `<verb> ${notebook}\n` (echo appends a
// trailing newline) for `list` and `folders`. The MCP layer must pass
// the bytes through unchanged on every surface. Asserts use the exact
// shim output, not `starts_with`, so appended, truncated, or
// reformatted output fails the regression.
const EXPECTED_LIST: &str = "listed mcp-stdio-testbook\n";
const EXPECTED_FOLDERS: &str = "folders mcp-stdio-testbook\n";

fn assert_passthrough_exact(surface: &str, response: &Value, expected: &str) {
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

#[test]
fn first_class_list_passes_exact_shim_output() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class("list", json!({}));
    assert_passthrough_exact("first-class list", &response, EXPECTED_LIST);
}

#[test]
fn multiplexed_list_passes_exact_shim_output() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_nb("nb.list", json!({}));
    assert_passthrough_exact("multiplexed nb.list", &response, EXPECTED_LIST);
}

#[test]
fn first_class_folders_passes_exact_shim_output() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class("folders", json!({}));
    assert_passthrough_exact("first-class folders", &response, EXPECTED_FOLDERS);
}

#[test]
fn multiplexed_folders_passes_exact_shim_output() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_nb("nb.folders", json!({}));
    assert_passthrough_exact("multiplexed nb.folders", &response, EXPECTED_FOLDERS);
}
