use serde_json::json;

use super::harness::*;

#[test]
fn replace_note_body_requires_fresh_fingerprint() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response =
        add_note_via_mcp(&mut server, &env, "Replace Me", "Original body.\n");
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    // Read the note to obtain the fresh fingerprint.
    let show_response = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let envelope = RealNbServer::result_json(&show_response);
    let fingerprint = envelope["fingerprint"].as_str().unwrap().to_string();

    // Correct fingerprint: replace succeeds.
    let ok = server.call_first_class(
        "replace_note_body",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "new_body": "Replaced body.\n",
            "fingerprint": fingerprint,
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        ok["result"]["isError"].as_bool(),
        Some(false),
        "replace with fresh fingerprint should succeed; got: {ok}"
    );

    // Stale fingerprint: replace is rejected with mismatch guidance.
    let stale =
        "b3:0000000000000000000000000000000000000000000000000000000000000000";
    let rejected = server.call_first_class(
        "replace_note_body",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "new_body": "Should be rejected.\n",
            "fingerprint": stale,
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        rejected["result"]["isError"].as_bool(),
        Some(true),
        "replace with stale fingerprint should be rejected; got: {rejected}"
    );
    let text = RealNbServer::error_text(&rejected);
    assert!(
        text.contains("fingerprint"),
        "stale-fingerprint rejection should mention fingerprint; got: {text}"
    );
    assert!(
        text.contains("re-read"),
        "stale-fingerprint rejection should tell caller to re-read; got: {text}"
    );
}

#[test]
fn edit_note_substring_replaces_occurrences_and_rejects_mismatches() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response = add_note_via_mcp(
        &mut server,
        &env,
        "Substring Note",
        "foo bar foo baz\n",
    );
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    // Success: replace all occurrences, expected_count matches.
    let ok = server.call_first_class(
        "edit_note_substring",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "pattern": "foo",
            "replacement": "qux",
            "occurrence": {"type": "all"},
            "expected_count": 2,
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        ok["result"]["isError"].as_bool(),
        Some(false),
        "substring replace should succeed; got: {ok}"
    );

    // Occurrence mismatch: expected_count disagrees with actual.
    let mismatch = server.call_first_class(
        "edit_note_substring",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "pattern": "qux",
            "replacement": "zed",
            "occurrence": {"type": "all"},
            "expected_count": 5,
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        mismatch["result"]["isError"].as_bool(),
        Some(true),
        "occurrence mismatch should be rejected; got: {mismatch}"
    );
    let mismatch_text = RealNbServer::error_text(&mismatch);
    assert!(
        mismatch_text.contains("expected") && mismatch_text.contains("found"),
        "occurrence mismatch should report expected/found; got: {mismatch_text}"
    );

    // Empty pattern rejected.
    let empty = server.call_first_class(
        "edit_note_substring",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "pattern": "",
            "replacement": "x",
            "occurrence": {"type": "first"},
            "expected_count": 0,
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        empty["result"]["isError"].as_bool(),
        Some(true),
        "empty pattern should be rejected; got: {empty}"
    );
    let empty_text = RealNbServer::error_text(&empty);
    assert!(
        empty_text.contains("non-empty"),
        "empty-pattern rejection should say non-empty; got: {empty_text}"
    );
}

#[test]
fn edit_note_lines_applies_anchored_edits_and_rejects_stale_anchors() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let body = "keep a\nreplace me\nkeep b\n";
    let add_response = add_note_via_mcp(&mut server, &env, "Line Edit", body);
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    // Read anchors from a preceding show_note_lines.
    let read = server.call_first_class(
        "show_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "notebook": env.notebook(),
        }),
    );
    let read_json = RealNbServer::result_json(&read);
    let lines = read_json["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 3);
    let line2 = &lines[1];
    let anchor2 = line2["anchor"].as_str().unwrap();

    // Success: replace line 2 with new content.
    let ok = server.call_first_class(
        "edit_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "edits": [
                {
                    "type": "replace",
                    "start": {"number": 2, "anchor": anchor2},
                    "end": {"number": 2, "anchor": anchor2},
                    "content": "replaced!",
                }
            ],
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        ok["result"]["isError"].as_bool(),
        Some(false),
        "anchored line edit should succeed; got: {ok}"
    );

    // Stale anchor: reuse the pre-edit anchor against the new body.
    let stale = server.call_first_class(
        "edit_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "edits": [
                {
                    "type": "replace",
                    "start": {"number": 2, "anchor": anchor2},
                    "end": {"number": 2, "anchor": anchor2},
                    "content": "stale",
                }
            ],
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        stale["result"]["isError"].as_bool(),
        Some(true),
        "stale anchor should be rejected; got: {stale}"
    );
    let stale_text = RealNbServer::error_text(&stale);
    assert!(
        stale_text.contains("anchor"),
        "stale-anchor rejection should mention anchor; got: {stale_text}"
    );
    assert!(
        stale_text.contains("re-read"),
        "stale-anchor rejection should say re-read; got: {stale_text}"
    );

    // Overlapping edits rejected: re-read fresh anchors first, then submit
    // two edits whose spans overlap on the current body.
    let fresh_read = server.call_first_class(
        "show_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "notebook": env.notebook(),
        }),
    );
    let fresh_json = RealNbServer::result_json(&fresh_read);
    let fresh_lines = fresh_json["lines"].as_array().unwrap();
    let fresh_a1 = fresh_lines[0]["anchor"].as_str().unwrap();
    let fresh_a2 = fresh_lines[1]["anchor"].as_str().unwrap();
    let overlap = server.call_first_class(
        "edit_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "edits": [
                {
                    "type": "replace",
                    "start": {"number": 1, "anchor": fresh_a1},
                    "end": {"number": 2, "anchor": fresh_a2},
                    "content": "x",
                },
                {
                    "type": "replace",
                    "start": {"number": 2, "anchor": fresh_a2},
                    "end": {"number": 2, "anchor": fresh_a2},
                    "content": "y",
                },
            ],
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        overlap["result"]["isError"].as_bool(),
        Some(true),
        "overlapping line edits should be rejected; got: {overlap}"
    );
    let overlap_text = RealNbServer::error_text(&overlap);
    assert!(
        overlap_text.contains("overlap"),
        "overlap rejection should mention overlap; got: {overlap_text}"
    );
}

#[test]
fn fragmented_body_rejects_line_operations_with_recovery_guidance() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    // Write a fragmented bookmark directly into the fixture notebook and
    // commit it so the worktree stays clean. Two reserved body sections
    // (`## Content` and a second `## Content`) separated by a canonical
    // `## Tags` section yield TWO body fragments.
    let notebook_root = env.nb_dir().join(env.notebook());
    let file_path = notebook_root.join("session-notes/fragmented.bookmark.md");
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    let body = b"<https://example.com>\n\n## Content\n\nhello\n\n## Tags\n\n#beta\n\n## Content\n\nagain\n";
    std::fs::write(&file_path, body).unwrap();
    commit_file(&env, &notebook_root, "session-notes/fragmented.bookmark.md");
    let path = "session-notes/fragmented.bookmark.md";

    // Confirm the fixture is genuinely fragmented via show.
    let show = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let envelope = RealNbServer::result_json(&show);
    assert_eq!(
        envelope["body_contiguous"].as_bool(),
        Some(false),
        "the crafted bookmark should be fragmented; got: {envelope}"
    );

    // Body-replace on a fragmented body is refused.
    let replace = server.call_first_class(
        "replace_note_body",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "new_body": "fragment attempt\n",
            "fingerprint": "b3:0000000000000000000000000000000000000000000000000000000000000000",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        replace["result"]["isError"].as_bool(),
        Some(true),
        "body replace on a fragmented body should be rejected; got: {replace}"
    );
    let text = RealNbServer::error_text(&replace);
    assert!(
        text.contains("fragment"),
        "fragmented-body rejection should mention fragments; got: {text}"
    );
    assert!(
        text.contains("retitle_note") && text.contains("edit_note_tags"),
        "fragmented-body rejection should point at metadata ops; got: {text}"
    );

    // Line listing on a fragmented body is refused too.
    let lines = server.call_first_class(
        "show_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        lines["result"]["isError"].as_bool(),
        Some(true),
        "show_note_lines on a fragmented body should be rejected; got: {lines}"
    );
}

#[test]
fn edit_note_lines_bare_text_replace_preserves_line_breaks() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response = add_note_via_mcp(
        &mut server,
        &env,
        "Breaks Note",
        "line one\nline two\nline three\n",
    );
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    let read = server.call_first_class(
        "show_note_lines",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let read_json = RealNbServer::result_json(&read);
    let anchor2 = read_json["lines"][1]["anchor"]
        .as_str()
        .unwrap()
        .to_string();

    // Bare-text content (no terminator): the library appends document eol,
    // so the following line must remain a separate line (no merge).
    let ok = server.call_first_class(
        "edit_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "edits": [
                {
                    "type": "replace",
                    "start": {"number": 2, "anchor": anchor2},
                    "end": {"number": 2, "anchor": anchor2},
                    "content": "LINE TWO",
                }
            ],
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        ok["result"]["isError"].as_bool(),
        Some(false),
        "bare-text replace should succeed; got: {ok}"
    );

    let after = server.call_first_class(
        "show_note_lines",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let after_json = RealNbServer::result_json(&after);
    assert_eq!(
        after_json["total_lines"].as_u64(),
        Some(3),
        "replace must not merge lines; got: {after_json}"
    );
    let texts: Vec<&str> = after_json["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["text"].as_str().unwrap())
        .collect();
    assert_eq!(
        texts,
        vec!["line one", "LINE TWO", "line three"],
        "line texts should reflect the bare-text replace; got: {texts:?}"
    );
}

#[test]
fn edit_note_lines_dollar_insert_terminates_final_line() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    // Unterminated final line: eol declared, has_final_eol false.
    let add_response =
        add_note_via_mcp(&mut server, &env, "Dollar Note", "one\ntwo");
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    let read = server.call_first_class(
        "show_note_lines",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let read_json = RealNbServer::result_json(&read);
    assert_eq!(
        read_json["has_final_eol"].as_bool(),
        Some(false),
        "unterminated body should report has_final_eol false; got: {read_json}"
    );

    let ok = server.call_first_class(
        "edit_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "edits": [
                {"type": "insert", "at": {"type": "boundary", "at": "dollar"}, "content": "three"}
            ],
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        ok["result"]["isError"].as_bool(),
        Some(false),
        "dollar insert should succeed; got: {ok}"
    );

    let show = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let show_json = RealNbServer::result_json(&show);
    assert_eq!(
        show_json["body"].as_str(),
        Some("one\ntwo\nthree\n"),
        "dollar insert should terminate the final line then append; got: {show_json}"
    );
}
