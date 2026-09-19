use serde_json::json;

use super::harness::*;

#[test]
fn retitle_note_changes_title_without_path() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response = add_note_via_mcp(&mut server, &env, "Old Title", "Body.\n");
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    let response = server.call_first_class(
        "retitle_note",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "title": "New Title",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        response["result"]["isError"].as_bool(),
        Some(false),
        "retitle should succeed; got: {response}"
    );

    let show_response = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let envelope = RealNbServer::result_json(&show_response);
    let title_text = envelope["title"].as_str().unwrap();
    assert_eq!(
        title_text, "New Title",
        "retitle should change the title text; got: {title_text}"
    );
}

#[test]
fn edit_note_tags_adds_and_removes_atomically() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response =
        add_note_via_mcp(&mut server, &env, "Tagged Note", "Body with a #kept tag.\n");
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    let response = server.call_first_class(
        "edit_note_tags",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "add": ["alpha", "beta"],
            "remove": [],
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        response["result"]["isError"].as_bool(),
        Some(false),
        "tag add should succeed; got: {response}"
    );

    let show_response = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let envelope = RealNbServer::result_json(&show_response);
    let tags = envelope["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(
        tags.contains(&"alpha") && tags.contains(&"beta"),
        "tags should include alpha and beta; got: {tags:?}"
    );
}

#[test]
fn edit_subcommand_rejection_is_parity_across_surfaces() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let payload = json!({
        "id": format!("{}:session-notes/test.md", env.notebook()),
        "content": "Should be rejected.",
        "mode": "overwrite",
    });
    let direct = server.call_first_class("edit", payload.clone());
    let multiplexed = server.call_multiplexed("nb.edit", payload);
    assert!(
        direct["result"]["isError"].as_bool() == Some(true) || direct["error"].is_object(),
        "direct edit should be rejected; got: {direct}"
    );
    assert_eq!(
        multiplexed["result"]["isError"].as_bool(),
        Some(true),
        "multiplexed edit should be rejected; got: {multiplexed}"
    );
    let direct_text = if direct["result"]["content"][0]["text"].is_string() {
        RealNbServer::error_text(&direct)
    } else {
        direct["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    };
    let multiplexed_text = RealNbServer::error_text(&multiplexed);
    assert!(
        direct_text.contains("removed") || direct_text.contains("not found"),
        "direct edit rejection should mention removal or absence; got: {direct_text}"
    );
    assert!(
        multiplexed_text.contains("removed"),
        "multiplexed edit rejection should mention removal; got: {multiplexed_text}"
    );
    for tool in [
        "replace_note_body",
        "edit_note_substring",
        "edit_note_lines",
        "retitle_note",
        "edit_note_tags",
    ] {
        assert!(
            multiplexed_text.contains(tool),
            "multiplexed edit rejection should name {tool}; got: {multiplexed_text}"
        );
    }
}

#[test]
fn dirty_baseline_mutation_is_rejected_with_recovery_guidance() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    // Create a note so there is a committed baseline.
    add_note_via_mcp(&mut server, &env, "Baseline Note", "Clean baseline.\n");

    // Dirty the worktree with an uncommitted file directly on disk.
    let notebook_root = env.nb_dir().join(env.notebook());
    let dirty_file = notebook_root.join("dirty-untracked.txt");
    std::fs::write(&dirty_file, b"untracked").unwrap();

    let response = server.call_first_class(
        "add",
        json!({
            "folder": "session-notes",
            "title": "Should fail on dirty baseline",
            "content": "Body.",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        response["result"]["isError"].as_bool(),
        Some(true),
        "mutation against a dirty worktree should be rejected; got: {response}"
    );
    let text = RealNbServer::error_text(&response);
    assert!(
        text.contains("dirty"),
        "dirty-baseline rejection should mention dirty; got: {text}"
    );
    assert!(
        text.contains("commit") || text.contains("clean"),
        "dirty-baseline rejection should give recovery guidance; got: {text}"
    );
}

// ---- Additional 0.3 direct-tool execution / error regressions (P2-2) ----

#[test]
fn add_rejects_todo_shaped_content() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    // Unchecked checkbox rejected on both surfaces with identical wording.
    let payload = json!({
        "folder": "session-notes",
        "title": "Unchecked",
        "content": "Notes here.\n\n- [ ] buy milk\n",
        "notebook": env.notebook(),
    });
    let direct = server.call_first_class("add", payload.clone());
    let multiplexed = server.call_multiplexed("nb.add", payload);
    for (label, response) in [("direct", &direct), ("multiplexed", &multiplexed)] {
        assert_eq!(
            response["result"]["isError"].as_bool(),
            Some(true),
            "[{label}] unchecked checkbox add should be rejected; got: {response}"
        );
        let text = RealNbServer::error_text(response);
        assert!(
            text.contains("`todo`"),
            "[{label}] rejection should name todo; got: {text}"
        );
    }
    assert_eq!(
        RealNbServer::error_text(&direct),
        RealNbServer::error_text(&multiplexed),
        "guard wording must be parity across surfaces"
    );

    // Checked-only checkbox rejected.
    let checked = server.call_first_class(
        "add",
        json!({
            "folder": "session-notes",
            "title": "Checked",
            "content": "Done list.\n\n- [x] done\n",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        checked["result"]["isError"].as_bool(),
        Some(true),
        "checked-only checkbox add should be rejected; got: {checked}"
    );

    // Tasks heading rejected.
    let heading = server.call_first_class(
        "add",
        json!({
            "folder": "session-notes",
            "title": "Heading",
            "content": "Plan.\n\n## Tasks\n\nstuff\n",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        heading["result"]["isError"].as_bool(),
        Some(true),
        "Tasks-heading add should be rejected; got: {heading}"
    );

    // Fenced checkbox allowed.
    let fenced_box = server.call_first_class(
        "add",
        json!({
            "folder": "session-notes",
            "title": "Fenced Box",
            "content": "Example:\n\n```\n- [ ] not a task\n```\n",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        fenced_box["result"]["isError"].as_bool(),
        Some(false),
        "fenced checkbox add should be accepted; got: {fenced_box}"
    );

    // Fenced Tasks heading allowed.
    let fenced_tasks = server.call_first_class(
        "add",
        json!({
            "folder": "session-notes",
            "title": "Fenced Tasks",
            "content": "Example:\n\n```\n## Tasks\n```\n",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        fenced_tasks["result"]["isError"].as_bool(),
        Some(false),
        "fenced Tasks-heading add should be accepted; got: {fenced_tasks}"
    );
}

#[test]
fn add_fence_tracking_follows_commonmark_shape() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    // Nested triple-backtick sample inside a four-backtick fence stays
    // content: the inner ```rust opener must not close the guard early.
    let nested = server.call_first_class(
        "add",
        json!({
            "folder": "session-notes",
            "title": "Nested Fence",
            "content": "Doc sample:\n\n````\n```rust\n- [ ] not a task\n```\n````\n",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        nested["result"]["isError"].as_bool(),
        Some(false),
        "checkbox inside a longer fence should be accepted; got: {nested}"
    );

    // A four-space-indented marker never opens a fence, so the following
    // real checkbox is still detected and rejected.
    let indented = server.call_first_class(
        "add",
        json!({
            "folder": "session-notes",
            "title": "Indented Fence",
            "content": "    ```\n- [ ] real task\n    ```\n",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        indented["result"]["isError"].as_bool(),
        Some(true),
        "indented marker must not suppress a real checklist; got: {indented}"
    );
    assert!(
        RealNbServer::error_text(&indented).contains("`todo`"),
        "indented-fence rejection should name todo; got: {}",
        RealNbServer::error_text(&indented)
    );

    // Opposite-marker lines inside a fence stay content.
    let mixed = server.call_first_class(
        "add",
        json!({
            "folder": "session-notes",
            "title": "Mixed Fence",
            "content": "Doc:\n\n~~~\n```\n- [ ] not a task\n~~~\n",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        mixed["result"]["isError"].as_bool(),
        Some(false),
        "opposite-marker lines inside a fence are content; got: {mixed}"
    );

    // An invalid backtick opener (backtick in info string) opens nothing,
    // so the following real checkbox is still detected and rejected.
    let bad_info = server.call_first_class(
        "add",
        json!({
            "folder": "session-notes",
            "title": "Bad Info",
            "content": "Doc:\n\n```lang`tick\n- [ ] real task\n```\n",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        bad_info["result"]["isError"].as_bool(),
        Some(true),
        "invalid backtick-info opener must not hide a checklist; got: {bad_info}"
    );
}
