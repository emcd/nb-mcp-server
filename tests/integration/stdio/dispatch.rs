use std::fs;

use serde_json::{Value, json};

use super::harness::*;

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
    let status_response =
        server.call_nb("nb.status", json!({"folder": "coordination"}));
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
    let calls =
        fs::read_to_string(shim.root.join("calls.log")).unwrap_or_default();
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
