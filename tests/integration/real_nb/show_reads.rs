use serde_json::json;

use super::harness::*;

#[test]
fn direct_show_on_folder_selector_returns_actionable_typed_error() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    let mkdir_response = server.call_first_class(
        "mkdir",
        json!({"path": FOLDER, "notebook": env.notebook()}),
    );
    assert_eq!(
        mkdir_response["result"]["isError"].as_bool(),
        Some(false),
        "mkdir should succeed against a fresh fixture; got: {mkdir_response}"
    );

    let response = server.call_first_class(
        "show",
        json!({"id": FOLDER, "notebook": env.notebook()}),
    );
    assert_typed_show_error(&response, "direct");
}

#[test]
fn multiplexed_show_on_folder_selector_returns_actionable_typed_error() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    let mkdir_response = server.call_first_class(
        "mkdir",
        json!({"path": FOLDER, "notebook": env.notebook()}),
    );
    assert_eq!(
        mkdir_response["result"]["isError"].as_bool(),
        Some(false),
        "mkdir should succeed; got: {mkdir_response}"
    );

    let response = server.call_multiplexed(
        "nb.show",
        json!({"id": FOLDER, "notebook": env.notebook()}),
    );
    assert_typed_show_error(&response, "multiplexed");
}

#[test]
fn show_folder_typed_error_is_parity_across_surfaces() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    let mkdir_response = server.call_first_class(
        "mkdir",
        json!({"path": FOLDER, "notebook": env.notebook()}),
    );
    assert_eq!(
        mkdir_response["result"]["isError"].as_bool(),
        Some(false),
        "mkdir should succeed; got: {mkdir_response}"
    );

    let payload = json!({"id": FOLDER, "notebook": env.notebook()});
    let direct = server.call_first_class("show", payload.clone());
    let multiplexed = server.call_multiplexed("nb.show", payload);
    let direct_text = RealNbServer::error_text(&direct);
    let multiplexed_text = RealNbServer::error_text(&multiplexed);
    assert_eq!(
        direct_text, multiplexed_text,
        "direct and multiplexed folder-show errors must produce identical wording"
    );
}

#[test]
fn add_duplicate_h1_typed_error_is_parity_across_surfaces() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    let payload = json!({
        "title": "Routing Notes",
        "content": "# Routing Notes\n\nDetails here.",
    });
    let direct = server.call_first_class("add", payload.clone());
    let multiplexed = server.call_multiplexed("nb.add", payload);
    let direct_text = RealNbServer::error_text(&direct);
    let multiplexed_text = RealNbServer::error_text(&multiplexed);
    assert_eq!(
        direct_text, multiplexed_text,
        "direct and multiplexed duplicate-h1 errors must produce identical wording"
    );
    assert_duplicate_h1_error(&direct_text, "direct");
    assert_duplicate_h1_error(&multiplexed_text, "multiplexed");
}

// ---- nb-api 0.3 native-engine surface regressions ----

#[test]
fn mutation_returns_structured_commit_outcome() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let response =
        add_note_via_mcp(&mut server, &env, "Outcome Note", "Body text.\n");
    let outcome = RealNbServer::result_json(&response);
    assert_eq!(
        outcome["commit_created"].as_bool(),
        Some(true),
        "add should create a commit; got: {outcome}"
    );
    assert!(
        outcome["revision_id"].as_str().is_some(),
        "add should report a revision id; got: {outcome}"
    );
    assert!(
        outcome["pre_revision"].as_str().is_some(),
        "add should report a pre_revision; got: {outcome}"
    );
    let ops = outcome["ops"].as_array().unwrap();
    assert_eq!(ops.len(), 1, "add should report one op; got: {outcome}");
    assert_eq!(
        ops[0]["noop"].as_bool(),
        Some(false),
        "add op should not be a noop; got: {outcome}"
    );
    let path = ops[0]["path"].as_str().unwrap();
    assert!(
        path.contains("session-notes/"),
        "add op path should be inside the folder; got: {path}"
    );
    // nb-faithful mangled filename: "Outcome Note" -> outcome_note.md.
    assert!(
        path.ends_with("session-notes/outcome_note.md"),
        "add op path should be title-mangled; got: {path}"
    );
    assert!(
        ops[0]["numeric_id"].as_u64().is_some(),
        "add op should report a numeric_id; got: {outcome}"
    );
    assert!(
        ops[0]["fingerprint"].as_str().is_some(),
        "add op should report a fingerprint; got: {outcome}"
    );
}

#[test]
fn show_returns_structured_envelope_with_byte_exact_source() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let note_body = "# Headline\n\nSome body with backticks `code`.\n";
    let add_response =
        add_note_via_mcp(&mut server, &env, "Envelope Note", note_body);
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    let response = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    assert_eq!(
        response["result"]["isError"].as_bool(),
        Some(false),
        "show should succeed; got: {response}"
    );
    let envelope = RealNbServer::result_json(&response);
    // Slim envelope: body is plain String, title is Option<String>
    assert!(
        envelope.get("source").is_none(),
        "slim envelope should not contain source; got: {envelope}"
    );
    assert!(
        envelope.get("non_utf8").is_none(),
        "slim envelope should not contain non_utf8; got: {envelope}"
    );
    assert!(
        envelope.get("body_fragments").is_none(),
        "slim envelope should not contain body_fragments; got: {envelope}"
    );
    assert_eq!(
        envelope["title"].as_str().unwrap(),
        "Envelope Note",
        "envelope title should be Envelope Note; got: {envelope}"
    );
    let body = envelope["body"].as_str().unwrap();
    assert!(
        body.contains("# Headline"),
        "envelope body should contain the note body; got: {body:?}"
    );
    assert!(
        body.contains("backticks `code`"),
        "envelope body should preserve backticks; got: {body:?}"
    );
    assert_eq!(
        envelope["body_contiguous"].as_bool(),
        Some(true),
        "contiguous body should be true; got: {envelope}"
    );
    assert!(
        envelope["selector"].as_str().is_some(),
        "envelope should have selector; got: {envelope}"
    );
    assert!(
        envelope["path"].as_str().is_some(),
        "envelope should have path; got: {envelope}"
    );
    assert!(
        envelope["tags"].is_array(),
        "envelope should have tags; got: {envelope}"
    );
    // Body should be the note body exactly
    assert_eq!(
        body, note_body,
        "envelope body must equal the original note body"
    );
    let fingerprint = envelope["fingerprint"].as_str().unwrap();
    assert!(
        fingerprint.starts_with("b3:"),
        "envelope fingerprint should be b3:; got: {fingerprint}"
    );
    // nb-api 0.4.1 numeric identity: creates are indexed, so reads carry it.
    assert!(
        envelope["numeric_id"].as_u64().is_some(),
        "envelope should carry numeric_id; got: {envelope}"
    );
    assert!(
        envelope["selector"].as_str().unwrap().contains(':'),
        "envelope selector should be notebook-qualified; got: {envelope}"
    );
}

#[test]
fn non_utf8_body_round_trips_through_show_and_replace() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response =
        add_note_via_mcp(&mut server, &env, "Binary Note", "text body\n");
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    // Under the text-first slim envelope, `replace_note_body` takes plain
    // UTF-8 String and `show` returns a typed ValidationError for non-UTF-8
    // source. Verify the new contract: a valid UTF-8 replace succeeds and
    // the subsequent show returns the new body, while a non-UTF-8 note
    // written directly to the filesystem is reported as an error with
    // recovery guidance.
    let show1 = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let envelope1 = RealNbServer::result_json(&show1);
    let fingerprint = envelope1["fingerprint"].as_str().unwrap().to_string();

    // Valid UTF-8 replace should succeed.
    let replaced = server.call_first_class(
        "replace_note_body",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "new_body": "Replaced valid body.\n",
            "fingerprint": fingerprint,
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        replaced["result"]["isError"].as_bool(),
        Some(false),
        "replace with valid UTF-8 should succeed; got: {replaced}"
    );
    let show_valid = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    assert_eq!(
        show_valid["result"]["isError"].as_bool(),
        Some(false),
        "show after valid replace should succeed; got: {show_valid}"
    );
    let envelope_valid = RealNbServer::result_json(&show_valid);
    assert_eq!(
        envelope_valid["body"].as_str().unwrap(),
        "Replaced valid body.\n",
        "body should reflect the valid replace; got: {envelope_valid}"
    );

    // Write a non-UTF-8 note directly to the filesystem (bypassing MCP
    // String validation) and verify `show` returns a typed error with
    // guidance to the external `nb` CLI.
    let notebook_root = env.nb_dir().join(env.notebook());
    let file_path = notebook_root.join(&path);
    let non_utf8: &[u8] = &[0xff, 0xfe, b'x', 0x00, b'\n', 0x80];
    let mut raw = Vec::new();
    raw.extend_from_slice(b"# Binary Note\n\n");
    raw.extend_from_slice(non_utf8);
    std::fs::write(&file_path, &raw).unwrap();
    commit_file(&env, &notebook_root, &path);

    let show2 = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    assert_eq!(
        show2["result"]["isError"].as_bool(),
        Some(true),
        "show on non-UTF-8 source should be rejected; got: {show2}"
    );
    let error_text = RealNbServer::error_text(&show2);
    assert!(
        error_text.contains("not valid UTF-8"),
        "non-UTF-8 error should mention UTF-8; got: {error_text}"
    );
    assert!(
        error_text.contains("nb show"),
        "non-UTF-8 error should guide to nb show CLI; got: {error_text}"
    );
    assert!(
        error_text.contains("outside MCP"),
        "non-UTF-8 error should direct raw retrieval outside MCP; got: {error_text}"
    );
}

#[test]
fn show_note_lines_returns_windowed_anchored_lines() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let body = "line one\nline two\nline three\n";
    let add_response = add_note_via_mcp(&mut server, &env, "Lines Note", body);
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    let response = server.call_first_class(
        "show_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "offset": 1,
            "limit": 2,
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        response["result"]["isError"].as_bool(),
        Some(false),
        "show_note_lines should succeed; got: {response}"
    );
    let lines = RealNbServer::result_json(&response);
    assert_eq!(
        lines["total_lines"].as_u64(),
        Some(3),
        "total_lines should be 3; got: {lines}"
    );
    let window = lines["lines"].as_array().unwrap();
    assert_eq!(window.len(), 2, "window should have 2 lines; got: {lines}");
    assert!(
        lines["next_offset"].as_u64().is_some(),
        "windowed result should have next_offset; got: {lines}"
    );
    let first = &window[0];
    assert_eq!(first["number"].as_u64(), Some(1));
    assert!(first["anchor"].as_str().unwrap().starts_with("b3l1:"));
    // Document-level EOL declaration (nb-api 0.4.1): no per-line terminator.
    assert_eq!(
        lines["eol"].as_str(),
        Some("lf"),
        "eol should declare lf; got: {lines}"
    );
    assert_eq!(
        lines["has_final_eol"].as_bool(),
        Some(true),
        "body ends with newline so has_final_eol; got: {lines}"
    );
    assert!(
        lines["numeric_id"].as_u64().is_some(),
        "lines result should carry numeric_id; got: {lines}"
    );
    for line in window {
        assert!(
            line.get("terminator").is_none(),
            "no per-line terminator field; got: {line}"
        );
        assert!(
            line["text"].as_str().is_some(),
            "line text should be a plain string; got: {line}"
        );
    }

    // Invalid window (offset 0) rejected with guidance.
    let invalid = server.call_first_class(
        "show_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "offset": 0,
            "limit": 10,
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        invalid["result"]["isError"].as_bool(),
        Some(true),
        "offset 0 should be rejected; got: {invalid}"
    );
    let invalid_text = RealNbServer::error_text(&invalid);
    assert!(
        invalid_text.contains("window"),
        "invalid-window rejection should mention window; got: {invalid_text}"
    );
}

#[test]
fn search_note_lines_returns_anchored_hits() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let body = "alpha beta\ngamma alpha\n";
    let add_response =
        add_note_via_mcp(&mut server, &env, "Search Lines", body);
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    let response = server.call_first_class(
        "search_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "pattern": "alpha",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        response["result"]["isError"].as_bool(),
        Some(false),
        "search_note_lines should succeed; got: {response}"
    );
    let result = RealNbServer::result_json(&response);
    let hits = result["hits"].as_array().unwrap();
    assert_eq!(
        hits.len(),
        2,
        "should find alpha on two lines; got: {result}"
    );
    assert!(
        hits[0]["anchor"].as_str().unwrap().starts_with("b3l1:"),
        "hit should carry an anchor; got: {result}"
    );
    // 10.5: hits[].text must be a plain string (not null, not ByteString object)
    for hit in hits {
        assert!(
            hit["text"].as_str().is_some(),
            "hit text should be a plain string, got: {hit}"
        );
    }
    // search_note_lines carries no EOL fields and no numeric_id by design.
    assert!(
        result.get("eol").is_none() && result.get("has_final_eol").is_none(),
        "search result should not declare eol; got: {result}"
    );
    assert!(
        result.get("numeric_id").is_none(),
        "search result should not carry numeric_id; got: {result}"
    );
    assert!(
        hits[0]["text"].as_str().unwrap().contains("alpha"),
        "hit text should contain the pattern, got: {hits:?}"
    );

    // Empty pattern rejected.
    let empty = server.call_first_class(
        "search_note_lines",
        json!({
            "id": format!("{}:{path}", env.notebook()),
            "pattern": "",
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        empty["result"]["isError"].as_bool(),
        Some(true),
        "empty search pattern should be rejected; got: {empty}"
    );
}

#[test]
fn numeric_selector_round_trip_resolves_same_note() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response =
        add_note_via_mcp(&mut server, &env, "Numeric Note", "Body.\n");
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();
    let numeric_id = outcome["ops"][0]["numeric_id"]
        .as_u64()
        .expect("create op should report numeric_id");
    let selector = outcome["ops"][0]["selector"]
        .as_str()
        .expect("create op should report selector")
        .to_string();

    // Show by folder-qualified numeric id resolves to the same path as
    // the path form (numeric ids are folder-local: root id 1 here is the
    // session-notes folder itself, recorded per the 0.4.1 add_folder fix).
    let by_id = server.call_first_class(
        "show",
        json!({"id": format!("{}:session-notes/{numeric_id}", env.notebook()), "notebook": env.notebook()}),
    );
    assert_eq!(
        by_id["result"]["isError"].as_bool(),
        Some(false),
        "show by numeric id should succeed; got: {by_id}"
    );
    let by_id_json = RealNbServer::result_json(&by_id);
    assert_eq!(
        by_id_json["path"].as_str(),
        Some(path.as_str()),
        "numeric-id show should resolve the same path; got: {by_id_json}"
    );
    assert_eq!(
        by_id_json["numeric_id"].as_u64(),
        Some(numeric_id),
        "numeric-id show should echo numeric_id; got: {by_id_json}"
    );
    // The outcome selector itself round-trips.
    let by_selector = server.call_first_class(
        "show",
        json!({"id": selector, "notebook": env.notebook()}),
    );
    assert_eq!(
        by_selector["result"]["isError"].as_bool(),
        Some(false),
        "show by outcome selector should succeed; got: {by_selector}"
    );
}

#[test]
fn show_note_lines_title_stays_normalized() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response =
        add_note_via_mcp(&mut server, &env, "Raw H1 Title", "Body.\n");
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    // show path: normalized via upstream title_text.
    let show = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let show_json = RealNbServer::result_json(&show);
    assert_eq!(
        show_json["title"].as_str(),
        Some("Raw H1 Title"),
        "show title should be normalized; got: {show_json}"
    );

    // show_note_lines path: retained normalization over raw H1 must match.
    let lines = server.call_first_class(
        "show_note_lines",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let lines_json = RealNbServer::result_json(&lines);
    assert_eq!(
        lines_json["title"].as_str(),
        Some("Raw H1 Title"),
        "show_note_lines title should match show; got: {lines_json}"
    );
}

#[test]
fn show_titles_agree_on_leading_hash_title() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    // Raw H1 line is `# #hashtag`; upstream title_text is `#hashtag`.
    // Both MCP paths must return `#hashtag` (no double-strip on show).
    let add_response =
        add_note_via_mcp(&mut server, &env, "#hashtag", "Body text.\n");
    let outcome = RealNbServer::result_json(&add_response);
    let path = outcome["ops"][0]["path"].as_str().unwrap().to_string();

    let show = server.call_first_class(
        "show",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let show_json = RealNbServer::result_json(&show);
    assert_eq!(
        show_json["title"].as_str(),
        Some("#hashtag"),
        "show must not strip a genuine leading hash; got: {show_json}"
    );

    let lines = server.call_first_class(
        "show_note_lines",
        json!({"id": format!("{}:{path}", env.notebook()), "notebook": env.notebook()}),
    );
    let lines_json = RealNbServer::result_json(&lines);
    assert_eq!(
        lines_json["title"].as_str(),
        Some("#hashtag"),
        "show_note_lines must agree with show; got: {lines_json}"
    );
}
