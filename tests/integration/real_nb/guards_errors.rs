use serde_json::json;

use super::harness::*;

#[test]
fn retitle_note_changes_title_without_path() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response =
        add_note_via_mcp(&mut server, &env, "Old Title", "Body.\n");
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
    let add_response = add_note_via_mcp(
        &mut server,
        &env,
        "Tagged Note",
        "Body with a #kept tag.\n",
    );
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
        direct["result"]["isError"].as_bool() == Some(true)
            || direct["error"].is_object(),
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
    for (label, response) in
        [("direct", &direct), ("multiplexed", &multiplexed)]
    {
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

#[test]
fn tasks_empty_scope_returns_empty_signal_not_error() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    // nb-api 0.4.2 normalizes nb's empty-tasks exit into an empty signal:
    // an empty notebook must succeed with a "0 tasks." signal, not fail.
    for recursive in [true, false] {
        let response = server.call_first_class(
            "tasks",
            json!({"recursive": recursive, "notebook": env.notebook()}),
        );
        assert_eq!(
            response["result"]["isError"].as_bool(),
            Some(false),
            "empty tasks (recursive={recursive}) should succeed; got: {response}"
        );
        let text = RealNbServer::error_text(&response);
        assert!(
            text.contains("0 tasks"),
            "empty tasks should report the empty signal; got: {text:?}"
        );
    }
    // With a status filter, nb reports which bucket is empty; that rides
    // through verbatim (minus the `!` error marker).
    let filtered = server.call_first_class(
        "tasks",
        json!({"recursive": true, "status": "open", "notebook": env.notebook()}),
    );
    assert_eq!(
        filtered["result"]["isError"].as_bool(),
        Some(false),
        "empty filtered tasks should succeed; got: {filtered}"
    );
    assert!(
        RealNbServer::error_text(&filtered).contains("0 open tasks"),
        "empty open-tasks should name the bucket; got: {filtered}"
    );
}

#[test]
fn tasks_recursive_selects_scope() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let todo_response = server.call_first_class(
        "todo",
        json!({
            "folder": "deep",
            "title": "Scoped Todo",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        todo_response["result"]["isError"].as_bool(),
        Some(false),
        "todo create should succeed; got: {todo_response}"
    );

    let flat = server.call_first_class(
        "tasks",
        json!({"recursive": false, "notebook": env.notebook()}),
    );
    assert_eq!(
        flat["result"]["isError"].as_bool(),
        Some(false),
        "folder-only tasks should succeed; got: {flat}"
    );
    assert!(
        !RealNbServer::error_text(&flat).contains("Scoped Todo"),
        "folder-only scope must not enumerate subfolder todos; got: {flat}"
    );

    let recursive = server.call_first_class(
        "tasks",
        json!({"recursive": true, "notebook": env.notebook()}),
    );
    assert_eq!(
        recursive["result"]["isError"].as_bool(),
        Some(false),
        "recursive tasks should succeed; got: {recursive}"
    );
    assert!(
        RealNbServer::error_text(&recursive).contains("Scoped Todo"),
        "recursive scope must enumerate subfolder todos; got: {recursive}"
    );
}

#[test]
fn edit_note_tags_shrink_keeps_show_agreement() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response =
        add_note_via_mcp(&mut server, &env, "Shrink Tags", "Body.\n");
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    let tagged = server.call_first_class(
        "edit_note_tags",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "add": ["alpha", "beta"],
            "remove": [],
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        tagged["result"]["isError"].as_bool(),
        Some(false),
        "tag add should succeed; got: {tagged}"
    );

    // Shrink to a single surviving tag: show must agree and no orphan
    // hashtag lines may remain (nb-api 0.4.2 grouped-section fix).
    let shrunk = server.call_first_class(
        "edit_note_tags",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "add": [],
            "remove": ["beta"],
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        shrunk["result"]["isError"].as_bool(),
        Some(false),
        "tag shrink should succeed; got: {shrunk}"
    );

    let show = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let envelope = RealNbServer::result_json(&show);
    let tags = envelope["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        tags,
        vec!["alpha"],
        "show must report exactly the surviving tag; got: {tags:?}"
    );
    let body = envelope["body"].as_str().unwrap();
    assert!(
        !body.lines().any(|l| l.starts_with('#')),
        "no orphan hashtag lines may remain in the body; body: {body:?}"
    );
    assert!(
        !body.contains("beta"),
        "removed tag must be gone from the body; body: {body:?}"
    );
}

#[test]
fn reconciling_read_then_mutate_succeeds_with_clean_tree() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    // Reviewer wedge sequence: add, then a folder-scoped list through the
    // multiplexed nb tool (a CLI-backed read that reconciles .index
    // outside any transaction), then mutate. The absorb guard must let
    // the mutation through and the tree must end clean.
    let add_response =
        add_note_via_mcp(&mut server, &env, "Wedge Note", "Body.\n");
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();
    assert_eq!(worktree_status(&env), "", "tree should be clean after add");

    let mkdir = server.call_first_class(
        "mkdir",
        json!({"path": "sub", "notebook": env.notebook()}),
    );
    assert_eq!(
        mkdir["result"]["isError"].as_bool(),
        Some(false),
        "mkdir should succeed; got: {mkdir}"
    );

    let listed = server.call_multiplexed(
        "nb.list",
        json!({"folder": "sub", "notebook": env.notebook()}),
    );
    assert_eq!(
        listed["result"]["isError"].as_bool(),
        Some(false),
        "multiplexed list should succeed; got: {listed}"
    );

    let edited = server.call_first_class(
        "edit_note_substring",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "pattern": "Body",
            "replacement": "Body edited",
            "occurrence": {"type": "first"},
            "expected_count": 1,
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        edited["result"]["isError"].as_bool(),
        Some(false),
        "mutation after reconciling read must succeed (absorb guard); got: {edited}"
    );
    assert_eq!(
        worktree_status(&env),
        "",
        "tree should be clean after the absorbing commit"
    );

    let added = add_note_via_mcp(&mut server, &env, "After Wedge", "More.\n");
    assert_eq!(
        added["result"]["isError"].as_bool(),
        Some(false),
        "follow-up add must succeed; got: {added}"
    );
    assert_eq!(worktree_status(&env), "", "tree should be clean at the end");
}

#[test]
fn delete_folder_refuses_non_empty_without_recursive() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let mkdir = server.call_first_class(
        "mkdir",
        json!({"path": "scratch", "notebook": env.notebook()}),
    );
    assert_eq!(
        mkdir["result"]["isError"].as_bool(),
        Some(false),
        "mkdir should succeed; got: {mkdir}"
    );
    add_note_via_mcp(&mut server, &env, "Scratch Note", "Body.\n");

    // Move the note into the folder so it is non-empty.
    let outcome = RealNbServer::result_json(&mkdir);
    let _ = outcome;
    let moved = server.call_first_class(
        "move",
        json!({
            "id": format!("{}:session-notes/scratch_note.md", env.notebook()),
            "destination": "scratch/",
            "notebook": env.notebook(),
        }),
    );
    // Fall back to an add-in-folder if the fixture name differs.
    if moved["result"]["isError"].as_bool() == Some(true) {
        let retry = server.call_first_class(
            "add",
            json!({
                "folder": "scratch",
                "title": "In Folder",
                "content": "Body.\n",
                "notebook": env.notebook(),
            }),
        );
        assert_eq!(
            retry["result"]["isError"].as_bool(),
            Some(false),
            "add in folder should succeed; got: {retry}"
        );
    }

    let refused = server.call_first_class(
        "delete",
        json!({"id": "scratch", "notebook": env.notebook()}),
    );
    assert_eq!(
        refused["result"]["isError"].as_bool(),
        Some(true),
        "non-recursive folder delete should be refused; got: {refused}"
    );
    assert!(
        RealNbServer::error_text(&refused).contains("recursive"),
        "refusal should name the recursive opt-in; got: {refused}"
    );
}

#[test]
fn delete_folder_recursive_removes_directory_and_markers() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let mkdir = server.call_first_class(
        "mkdir",
        json!({"path": "ephemeral", "notebook": env.notebook()}),
    );
    assert_eq!(
        mkdir["result"]["isError"].as_bool(),
        Some(false),
        "mkdir should succeed; got: {mkdir}"
    );
    let add_response = server.call_first_class(
        "add",
        json!({
            "folder": "ephemeral",
            "title": "Temp",
            "content": "Body.\n",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        add_response["result"]["isError"].as_bool(),
        Some(false),
        "add in folder should succeed; got: {add_response}"
    );

    let deleted = server.call_first_class(
        "delete",
        json!({"id": "ephemeral", "recursive": true, "notebook": env.notebook()}),
    );
    assert_eq!(
        deleted["result"]["isError"].as_bool(),
        Some(false),
        "recursive folder delete should succeed; got: {deleted}"
    );
    assert!(
        !env.nb_dir().join(env.notebook()).join("ephemeral").exists(),
        "folder directory must be gone from disk"
    );
    assert_eq!(
        worktree_status(&env),
        "",
        "tree should be clean after folder delete"
    );

    // The folder id must not resurrect with a new numeric id.
    let again = server.call_first_class(
        "delete",
        json!({"id": "ephemeral", "recursive": true, "notebook": env.notebook()}),
    );
    assert_eq!(
        again["result"]["isError"].as_bool(),
        Some(true),
        "second delete should be NotFound; got: {again}"
    );
}

#[test]
fn subcommand_named_folder_resolves_cleanly() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    // A folder whose name collides with nb subcommand vocabulary must
    // resolve through the filesystem fast path, never the nb CLI
    // dispatch that misroutes it (todos listing, foreign paths, ...).
    let mkdir = server.call_first_class(
        "mkdir",
        json!({"path": "todos", "notebook": env.notebook()}),
    );
    assert_eq!(
        mkdir["result"]["isError"].as_bool(),
        Some(false),
        "mkdir todos should succeed; got: {mkdir}"
    );

    let shown = server.call_first_class(
        "show",
        json!({"id": "todos", "notebook": env.notebook()}),
    );
    assert_eq!(
        shown["result"]["isError"].as_bool(),
        Some(true),
        "show on a folder must be rejected; got: {shown}"
    );
    let text = RealNbServer::error_text(&shown);
    assert!(
        text.contains("non-textual") && text.contains("folder"),
        "show must return the typed folder diagnostic, not nb CLI output; got: {text:?}"
    );
    assert!(
        !text.contains("No todos found"),
        "show must not dispatch to todos listing; got: {text:?}"
    );

    let deleted = server.call_first_class(
        "delete",
        json!({"id": "todos", "recursive": true, "notebook": env.notebook()}),
    );
    assert_eq!(
        deleted["result"]["isError"].as_bool(),
        Some(false),
        "recursive delete of subcommand-named folder should succeed; got: {deleted}"
    );
    assert!(
        !env.nb_dir().join(env.notebook()).join("todos").exists(),
        "folder must be gone from disk"
    );
    assert_eq!(
        worktree_status(&env),
        "",
        "tree should be clean after folder operations"
    );
}
