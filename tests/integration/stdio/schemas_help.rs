use serde_json::{Value, json};

use super::harness::*;

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
    let response = server
        .call_first_class("search", json!({"queries": ["test", "search"]}));
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
        error_msg.contains("unknown field")
            || error_msg.contains("failed to deserialize"),
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
    let tool_names: Vec<&str> =
        tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
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
    let tasks_type =
        todo_schema["properties"]["tasks"]["type"].as_str().unwrap();
    assert_eq!(tasks_type, "array", "todo.tasks should be array");
    let todo_tags_type =
        todo_schema["properties"]["tags"]["type"].as_str().unwrap();
    assert_eq!(todo_tags_type, "array", "todo.tags should be array");

    // list: tags must be array type
    let list_tool = find_tool("list");
    let list_schema = &list_tool["inputSchema"];
    let list_tags_type =
        list_schema["properties"]["tags"]["type"].as_str().unwrap();
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
        folder_prop.get("anyOf").is_none()
            && folder_prop.get("oneOf").is_none(),
        "add.folder should not be a nullable union"
    );
    let notebook_prop = &add_schema["properties"]["notebook"];
    assert_eq!(
        notebook_prop["type"].as_str().unwrap(),
        "string",
        "add.notebook should be plain string"
    );
    assert!(
        notebook_prop.get("anyOf").is_none()
            && notebook_prop.get("oneOf").is_none(),
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
        description_prop.get("anyOf").is_none()
            && description_prop.get("oneOf").is_none(),
        "todo.description should not be a nullable union"
    );

    // replace_note_body: fingerprint and new_body must be plain string (no nullable union), id flat
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
    // id must be plain string (flat id, alias selector)
    let id_prop = &rnb_schema["properties"]["id"];
    assert_eq!(
        id_prop["type"].as_str().unwrap(),
        "string",
        "replace_note_body.id should be plain string"
    );
    assert!(
        id_prop.get("anyOf").is_none() && id_prop.get("oneOf").is_none(),
        "replace_note_body.id should not be a nullable union"
    );
    let new_body_prop = &rnb_schema["properties"]["new_body"];
    assert_eq!(
        new_body_prop["type"].as_str().unwrap(),
        "string",
        "replace_note_body.new_body should be plain string"
    );
    let required = rnb_schema["required"].as_array().unwrap();
    for field in ["id", "new_body", "fingerprint"] {
        assert!(
            required.iter().any(|r| r.as_str() == Some(field)),
            "replace_note_body should require {field}"
        );
    }
    // ensure no ByteString shape appears
    assert!(
        rnb_schema["properties"]["new_body"]["properties"].is_null(),
        "replace_note_body.new_body should not be a ByteString object"
    );
}

// Pure-CLI-read first-class tools.
