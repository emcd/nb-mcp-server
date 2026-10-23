use serde_json::json;

use super::harness::*;

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
    let response =
        server.call_first_class("tasks", json!({"recursive": false}));
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
        status_prop.get("anyOf").is_none()
            && status_prop.get("oneOf").is_none(),
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
    assert_passthrough_exact(
        "first-class folders",
        &response,
        EXPECTED_FOLDERS,
    );
}

#[test]
fn multiplexed_folders_passes_exact_shim_output() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    let response = server.call_nb("nb.folders", json!({}));
    assert_passthrough_exact(
        "multiplexed nb.folders",
        &response,
        EXPECTED_FOLDERS,
    );
}

// Help-text consistency: add/todo guidance must name the Note/Todo kinds
// everywhere help describes them (detailed entries and nb overview).

#[test]
fn help_add_todo_descriptions_name_note_todo_kinds() {
    let shim = shim_env();
    let mut server = start_server(&shim);
    for query in ["nb.add", "add"] {
        let description = help_description(&mut server, query);
        assert!(
            description.contains("DocumentKind::Note"),
            "{query} help should name DocumentKind::Note; got: {description}"
        );
        assert!(
            description.contains("`todo`"),
            "{query} help should point at todo; got: {description}"
        );
    }
    for query in ["nb.todo", "todo"] {
        let description = help_description(&mut server, query);
        assert!(
            description.contains("DocumentKind::Todo"),
            "{query} help should name DocumentKind::Todo; got: {description}"
        );
        assert!(
            description.contains("`add`"),
            "{query} help should contrast with add; got: {description}"
        );
    }
    // nb overview entries carry the short pointer form (both arrays).
    let response = server.call_help("nb");
    let help = tool_json(&response);
    let first_class = help["first_class_tools"].as_array().unwrap();
    let entry = |tool: &str| {
        first_class
            .iter()
            .find(|t| t["tool"].as_str() == Some(tool))
            .unwrap()["description"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert!(
        entry("add").contains("`todo`"),
        "nb overview add entry should point at todo"
    );
    assert!(
        entry("todo").contains("`add`"),
        "nb overview todo entry should contrast with add"
    );
    let commands = help["commands"].as_array().unwrap();
    let command_entry = |command: &str| {
        commands
            .iter()
            .find(|t| t["command"].as_str() == Some(command))
            .unwrap()["description"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert!(
        command_entry("nb.add").contains("DocumentKind::Note")
            && command_entry("nb.add").contains("`todo`"),
        "nb commands nb.add entry should name the Note kind and point at todo"
    );
    assert!(
        command_entry("nb.todo").contains("DocumentKind::Todo")
            && command_entry("nb.todo").contains("`add`"),
        "nb commands nb.todo entry should name the Todo kind and contrast with add"
    );
}
