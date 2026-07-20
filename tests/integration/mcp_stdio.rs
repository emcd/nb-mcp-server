#![cfg(unix)]

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

fn rejection_text(response: &Value) -> Option<String> {
    if let Some(text) = response["result"]["content"][0]["text"].as_str() {
        return Some(text.to_string());
    }
    if let Some(message) = response["error"]["message"].as_str() {
        return Some(message.to_string());
    }
    None
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

// First-class tool tests.

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
fn first_class_add_accepts_array_tags() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "add",
        json!({
            "folder": "procedures",
            "title": "Test note with tags",
            "content": "Content here.",
            "tags": ["tag1", "tag2"]
        }),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("Added:"));
}

#[test]
fn first_class_todo_accepts_array_tasks_and_tags() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "todo",
        json!({
            "folder": "procedures",
            "title": "Test todo",
            "tasks": ["step1", "step2"],
            "tags": ["urgent"]
        }),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("Added:"));
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
    assert_eq!(first_class.len(), 17);
    let tool_names: Vec<&str> = first_class
        .iter()
        .map(|t| t["tool"].as_str().unwrap())
        .collect();
    assert!(tool_names.contains(&"add"));
    assert!(tool_names.contains(&"search"));
    assert!(tool_names.contains(&"todo"));
    assert!(tool_names.contains(&"list"));
    assert!(tool_names.contains(&"status"));
    assert!(tool_names.contains(&"notebooks"));
    assert!(tool_names.contains(&"show"));
    assert!(tool_names.contains(&"edit"));
    assert!(tool_names.contains(&"delete"));
    assert!(tool_names.contains(&"move"));
    assert!(tool_names.contains(&"do"));
    assert!(tool_names.contains(&"undo"));
    assert!(tool_names.contains(&"tasks"));
    assert!(tool_names.contains(&"bookmark"));
    assert!(tool_names.contains(&"folders"));
    assert!(tool_names.contains(&"mkdir"));
    assert!(tool_names.contains(&"import"));
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
        "edit",
        "delete",
        "move",
        "do",
        "undo",
        "tasks",
        "bookmark",
        "folders",
        "mkdir",
        "import",
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
        "edit",
        "delete",
        "move",
        "do",
        "undo",
        "tasks",
        "bookmark",
        "folders",
        "mkdir",
        "import",
    ] {
        assert!(
            tool_names.contains(&tool),
            "tool {tool} not found in {tool_names:?}"
        );
    }
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
    let desc_prop = &todo_schema["properties"]["description"];
    assert_eq!(
        desc_prop["type"].as_str().unwrap(),
        "string",
        "todo.description should be plain string"
    );
    assert!(
        desc_prop.get("anyOf").is_none() && desc_prop.get("oneOf").is_none(),
        "todo.description should not be a nullable union"
    );
}

#[test]
fn first_class_todo_accepts_content_alias_for_description() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "todo",
        json!({
            "folder": "session-notes",
            "title": "Alias test",
            "content": "This should map to description via alias."
        }),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("Added:"));
}

// Tests for new first-class tools (status, notebooks, show, edit, delete, move, do, undo, tasks, bookmark, folders, mkdir, import).

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
fn first_class_show_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "show",
        json!({"id": format!("{TEST_NOTEBOOK}:session-notes/test.md")}),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("shown"));
}

#[test]
fn first_class_edit_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "edit",
        json!({
            "id": format!("{TEST_NOTEBOOK}:session-notes/test.md"),
            "content": "Updated content.",
            "mode": "overwrite",
        }),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("edited"));
}

#[test]
fn first_class_delete_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "delete",
        json!({"id": format!("{TEST_NOTEBOOK}:session-notes/test.md")}),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("Deleted"));
}

#[test]
fn first_class_move_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "move",
        json!({
            "id": format!("{TEST_NOTEBOOK}:session-notes/test.md"),
            "destination": "archive/"
        }),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("Moved"));
}

#[test]
fn first_class_do_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "do",
        json!({"id": format!("{TEST_NOTEBOOK}:session-notes/test.md")}),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("Completed"));
}

#[test]
fn first_class_undo_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "undo",
        json!({"id": format!("{TEST_NOTEBOOK}:session-notes/test.md")}),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("Uncompleted"));
}

#[test]
fn first_class_tasks_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class("tasks", json!({}));
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("tasks"));
}

#[test]
fn first_class_bookmark_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "bookmark",
        json!({
            "folder": "session-notes",
            "url": "https://example.com",
            "title": "Test Bookmark"
        }),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("bookmarked"));
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
fn first_class_mkdir_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class("mkdir", json!({"path": "test-folder"}));
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("Created folder"));
}

#[test]
fn first_class_import_works() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "import",
        json!({
            "folder": "session-notes",
            "source": "https://example.com/test.md"
        }),
    );
    assert!(!is_tool_error(&response), "response: {response}");
    assert!(tool_text(&response).contains("imported"));
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

// Cross-surface equivalence tests: verify multiplexed nb and first-class tools
// produce equivalent behavior for the same parameters.

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

#[test]
fn cross_surface_add_equivalence() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    // Mutation: both should invoke the same nb command shape
    let args = json!({
        "folder": "session-notes",
        "title": "Cross-surface test",
        "content": "Testing equivalence."
    });
    let multiplexed = server.call_nb("nb.add", args.clone());
    let first_class = server.call_first_class("add", args);
    assert!(!is_tool_error(&multiplexed), "multiplexed: {multiplexed}");
    assert!(!is_tool_error(&first_class), "first_class: {first_class}");
    // Both should produce "Added:" output (same shim response)
    assert!(tool_text(&multiplexed).contains("Added:"));
    assert!(tool_text(&first_class).contains("Added:"));
}

#[test]
fn cross_surface_todo_equivalence() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    // Mutation: both should invoke the same nb command shape
    let args = json!({
        "folder": "session-notes",
        "title": "Cross-surface todo",
        "tasks": ["step1", "step2"]
    });
    let multiplexed = server.call_nb("nb.todo", args.clone());
    let first_class = server.call_first_class("todo", args);
    assert!(!is_tool_error(&multiplexed), "multiplexed: {multiplexed}");
    assert!(!is_tool_error(&first_class), "first_class: {first_class}");
    // Both should produce "Added:" output (same shim response)
    assert!(tool_text(&multiplexed).contains("Added:"));
    assert!(tool_text(&first_class).contains("Added:"));
}

// Edit-mode contract regressions: edit.mode is required; canonical
// overwrite is advertised; legacy replace remains compatible input.

#[test]
fn edit_mode_is_required_in_first_class_schema() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let tools = server.list_tools();
    let tools = tools["result"]["tools"].as_array().unwrap();
    let edit_tool = tools
        .iter()
        .find(|t| t["name"].as_str() == Some("edit"))
        .expect("edit tool should exist");
    let required = edit_tool["inputSchema"]["required"]
        .as_array()
        .expect("edit schema should expose required list");
    let required_fields: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        required_fields.contains(&"mode"),
        "edit.mode should be required, got required: {required_fields:?}"
    );

    let schema = &edit_tool["inputSchema"];
    let defs = &schema["$defs"];
    let mode_ref = schema["properties"]["mode"]["$ref"]
        .as_str()
        .expect("mode should $ref a $defs entry");
    let mode_def_name = mode_ref.trim_start_matches("#/$defs/");
    let mode_enum = &defs[mode_def_name]["oneOf"];
    let variants: Vec<String> = mode_enum
        .as_array()
        .expect("EditMode oneOf should be an array")
        .iter()
        .filter_map(|entry| entry["const"].as_str().map(str::to_string))
        .collect();
    for required_variant in ["overwrite", "append", "prepend"] {
        assert!(
            variants.iter().any(|v| v == required_variant),
            "edit.mode should advertise {required_variant:?}, got variants: {variants:?}"
        );
    }
    assert!(
        !variants.iter().any(|v| v == "replace"),
        "edit.mode must not advertise legacy replace, got variants: {variants:?}"
    );
}

#[test]
fn edit_mode_is_required_in_multiplexed_help() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let help = server.call_help("nb.edit");
    assert!(!is_tool_error(&help), "help: {help}");
    let help_text = tool_text(&help);
    assert!(
        help_text.contains("mode required"),
        "nb.edit help should describe mode as required, got: {help_text}"
    );
    for variant in ["overwrite", "append", "prepend"] {
        assert!(
            help_text.contains(variant),
            "nb.edit help should name {variant:?}, got: {help_text}"
        );
    }
}

#[test]
fn first_class_edit_rejects_missing_mode_before_invoking_nb() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "edit",
        json!({
            "id": format!("{TEST_NOTEBOOK}:session-notes/test.md"),
            "content": "Updated content.",
        }),
    );
    assert!(
        is_rejection(&response),
        "expected rejection when mode is missing, got: {response}"
    );
    let calls = fs::read_to_string(shim.root.join("calls.log")).unwrap_or_default();
    assert!(
        !calls.lines().any(|line| line.starts_with("edit ")),
        "edit must not be invoked when mode is missing; calls: {calls}"
    );
}

#[test]
fn multiplexed_edit_rejects_missing_mode_before_invoking_nb() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_nb(
        "nb.edit",
        json!({
            "id": format!("{TEST_NOTEBOOK}:session-notes/test.md"),
            "content": "Updated content.",
        }),
    );
    assert!(
        is_rejection(&response),
        "expected rejection when mode is missing, got: {response}"
    );
    let calls = fs::read_to_string(shim.root.join("calls.log")).unwrap_or_default();
    assert!(
        !calls.lines().any(|line| line.starts_with("edit ")),
        "edit must not be invoked when mode is missing; calls: {calls}"
    );
}

#[test]
fn edit_accepts_legacy_replace_input() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_first_class(
        "edit",
        json!({
            "id": format!("{TEST_NOTEBOOK}:session-notes/test.md"),
            "content": "Updated content.",
            "mode": "replace",
        }),
    );
    assert!(
        !is_rejection(&response),
        "legacy mode:replace must remain compatible, got: {response}"
    );
    assert!(tool_text(&response).contains("edited"));
}

#[test]
fn multiplexed_edit_accepts_legacy_replace_input() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_nb(
        "nb.edit",
        json!({
            "id": format!("{TEST_NOTEBOOK}:session-notes/test.md"),
            "content": "Updated content.",
            "mode": "replace",
        }),
    );
    assert!(
        !is_rejection(&response),
        "legacy mode:replace must remain compatible, got: {response}"
    );
    assert!(tool_text(&response).contains("edited"));
}

// Cross-surface equivalence for edit behavior: every edit contract,
// observed through both surfaces, must produce identical wording.

#[test]
fn edit_mode_rejection_is_parity_across_surfaces() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let payload = json!({
        "id": format!("{TEST_NOTEBOOK}:session-notes/test.md"),
        "content": "Updated content.",
    });
    let first_class = server.call_first_class("edit", payload.clone());
    let multiplexed = server.call_nb("nb.edit", payload);
    assert!(
        is_rejection(&first_class),
        "first-class edit should reject missing mode, got: {first_class}"
    );
    assert!(
        is_rejection(&multiplexed),
        "multiplexed edit should reject missing mode, got: {multiplexed}"
    );
    let first_class_text = rejection_text(&first_class).unwrap_or_else(|| {
        panic!("direct edit rejection produced no diagnostic text: {first_class}")
    });
    let multiplexed_text = rejection_text(&multiplexed).unwrap_or_else(|| {
        panic!("multiplexed edit rejection produced no diagnostic text: {multiplexed}")
    });
    assert!(
        !first_class_text.is_empty() && !multiplexed_text.is_empty(),
        "both surfaces should produce a missing-mode diagnostic, got direct={first_class_text:?} multiplexed={multiplexed_text:?}"
    );
    assert_eq!(
        first_class_text, multiplexed_text,
        "direct and multiplexed missing-mode errors must produce identical wording"
    );
    for value in ["overwrite", "append", "prepend"] {
        assert!(
            first_class_text.contains(value),
            "missing-mode diagnostic should name {value}, got: {first_class_text}"
        );
    }
}

#[test]
fn edit_overwrite_success_is_parity_across_surfaces() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let payload = json!({
        "id": format!("{TEST_NOTEBOOK}:session-notes/test.md"),
        "content": "Updated content.",
        "mode": "overwrite",
    });
    let first_class = server.call_first_class("edit", payload.clone());
    let multiplexed = server.call_nb("nb.edit", payload);
    assert!(
        !is_rejection(&first_class),
        "first-class overwrite should succeed, got: {first_class}"
    );
    assert!(
        !is_rejection(&multiplexed),
        "multiplexed overwrite should succeed, got: {multiplexed}"
    );
    assert_eq!(
        tool_text(&first_class),
        tool_text(&multiplexed),
        "direct and multiplexed edit success responses must match"
    );
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
