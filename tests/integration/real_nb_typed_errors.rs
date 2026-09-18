#![cfg(unix)]

use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use nb_api::testing::NbTestEnv;
use serde_json::{Value, json};

struct RealNbServer {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Drop for RealNbServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl RealNbServer {
    fn spawn(env: &NbTestEnv) -> Self {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_nb-mcp"));
        cmd.arg("--notebook")
            .arg(env.notebook())
            .arg("--no-commit-signing")
            .arg("--no-create-notebook")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        env.configure_std(&mut cmd);
        let mut child = cmd.spawn().expect("spawn nb-mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut server = RealNbServer {
            child,
            stdin,
            stdout,
            next_id: 1,
        };
        server.initialize();
        server
    }

    fn initialize(&mut self) {
        let response = self.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "real-nb-test", "version": "0.0.0"}
            }),
        );
        assert!(response.get("result").is_some(), "initialize: {response}");
        self.notification("notifications/initialized", json!({}));
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        writeln!(self.stdin, "{}", serde_json::to_string(&message).unwrap()).unwrap();
        self.stdin.flush().unwrap();
        loop {
            let mut line = String::new();
            let read = self.stdout.read_line(&mut line).unwrap();
            assert!(read > 0, "server stdout closed");
            let response: Value = serde_json::from_str(line.trim()).unwrap();
            if response.get("id") == Some(&json!(id)) {
                return response;
            }
        }
    }

    fn notification(&mut self, method: &str, params: Value) {
        let message = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        writeln!(self.stdin, "{}", serde_json::to_string(&message).unwrap()).unwrap();
        self.stdin.flush().unwrap();
    }

    fn call_first_class(&mut self, tool: &str, args: Value) -> Value {
        self.request("tools/call", json!({"name": tool, "arguments": args}))
    }

    fn call_multiplexed(&mut self, command: &str, args: Value) -> Value {
        self.request(
            "tools/call",
            json!({
                "name": "nb",
                "arguments": {"command": command, "args": args},
            }),
        )
    }

    fn error_text(response: &Value) -> String {
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }

    fn result_json(response: &Value) -> Value {
        let content = &response["result"]["content"][0];
        if let Some(value) = content.get("json") {
            return value.clone();
        }
        let text = content["text"].as_str().unwrap();
        serde_json::from_str(text).unwrap()
    }
}

/// Path of the regression folder inside each fixture. `nb` 7.24.0
/// reserves `archive` as a top-level command (`nb archive
/// <notebook>`). With a fixture whose current notebook is
/// `scratch`, the qualified-folder argv `nb add folder
/// scratch:archive` is misparsed — the resolved
/// `nb-api::add_folder("archive", ...)` invocation produces the
/// `nb command failed: ! Notebook not found: folder` error and the
/// folder is never created. `subfolder` is unambiguous and
/// matches the upstream `show_rejects_folder_selector` regression.
const FOLDER: &str = "subfolder";

fn fresh_env() -> NbTestEnv {
    NbTestEnv::new().expect("real-nb fixture initialization")
}

fn assert_typed_show_error(response: &Value, label: &str) {
    assert_eq!(
        response["result"]["isError"].as_bool(),
        Some(true),
        "[{label}] expected isError, got: {response}"
    );
    let error_text = RealNbServer::error_text(response);
    assert!(
        error_text.contains("subfolder"),
        "[{label}] selector name `subfolder` missing: {error_text}"
    );
    assert!(
        error_text.contains("folder"),
        "[{label}] actual_type `folder` missing: {error_text}"
    );
    assert!(
        error_text.contains("non-textual") || error_text.contains("non-text"),
        "[{label}] non-text classification missing: {error_text}"
    );
    assert!(
        error_text.contains("`show` reads text notes only"),
        "[{label}] show restriction missing: {error_text}"
    );
    assert!(
        error_text.contains("folders") && error_text.contains("list"),
        "[{label}] recovery toward folders/list missing: {error_text}"
    );
}

fn assert_duplicate_h1_error(error_text: &str, label: &str) {
    assert!(
        error_text.contains("`Routing Notes`"),
        "[{label}] title missing: {error_text}"
    );
    assert!(
        error_text.contains("# Routing Notes"),
        "[{label}] duplicate heading missing: {error_text}"
    );
    assert!(
        error_text.contains("duplicate")
            && (error_text.contains("remove the duplicate")
                || error_text.contains("omit the separate")),
        "[{label}] recovery guidance missing: {error_text}"
    );
}

#[test]
fn direct_show_on_folder_selector_returns_actionable_typed_error() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    let mkdir_response =
        server.call_first_class("mkdir", json!({"path": FOLDER, "notebook": env.notebook()}));
    assert_eq!(
        mkdir_response["result"]["isError"].as_bool(),
        Some(false),
        "mkdir should succeed against a fresh fixture; got: {mkdir_response}"
    );

    let response =
        server.call_first_class("show", json!({"id": FOLDER, "notebook": env.notebook()}));
    assert_typed_show_error(&response, "direct");
}

#[test]
fn multiplexed_show_on_folder_selector_returns_actionable_typed_error() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    let mkdir_response =
        server.call_first_class("mkdir", json!({"path": FOLDER, "notebook": env.notebook()}));
    assert_eq!(
        mkdir_response["result"]["isError"].as_bool(),
        Some(false),
        "mkdir should succeed; got: {mkdir_response}"
    );

    let response =
        server.call_multiplexed("nb.show", json!({"id": FOLDER, "notebook": env.notebook()}));
    assert_typed_show_error(&response, "multiplexed");
}

#[test]
fn show_folder_typed_error_is_parity_across_surfaces() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);

    let mkdir_response =
        server.call_first_class("mkdir", json!({"path": FOLDER, "notebook": env.notebook()}));
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

/// Address a note by its fixture-relative path (e.g. `folder/name.md`).
#[allow(dead_code)]
fn target_path(path: &str) -> Value {
    json!({"type": "path", "value": path})
}

/// Create a note through the MCP `add` tool and return the created note's
/// path (from the structured CommitOutcome) plus the response.
fn add_note_via_mcp(
    server: &mut RealNbServer,
    env: &NbTestEnv,
    title: &str,
    content: &str,
) -> Value {
    let response = server.call_first_class(
        "add",
        json!({
            "folder": "session-notes",
            "title": title,
            "content": content,
            "notebook": env.notebook(),
        }),
    );
    assert_eq!(
        response["result"]["isError"].as_bool(),
        Some(false),
        "add should succeed; got: {response}"
    );
    response
}

#[test]
fn mutation_returns_structured_commit_outcome() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let response = add_note_via_mcp(&mut server, &env, "Outcome Note", "Body text.\n");
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
    let add_response = add_note_via_mcp(&mut server, &env, "Envelope Note", note_body);
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
fn replace_note_body_requires_fresh_fingerprint() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response = add_note_via_mcp(&mut server, &env, "Replace Me", "Original body.\n");
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
    let stale = "b3:0000000000000000000000000000000000000000000000000000000000000000";
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
fn non_utf8_body_round_trips_through_show_and_replace() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response = add_note_via_mcp(&mut server, &env, "Binary Note", "text body\n");
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
fn edit_note_substring_replaces_occurrences_and_rejects_mismatches() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response = add_note_via_mcp(&mut server, &env, "Substring Note", "foo bar foo baz\n");
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
    let add_response = add_note_via_mcp(&mut server, &env, "Search Lines", body);
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

/// Commit one new file in the fixture notebook so the worktree returns
/// clean (the native Transaction engine refuses a dirty baseline).
fn commit_file(env: &NbTestEnv, notebook_root: &std::path::Path, rel_path: &str) {
    let mut git = Command::new("git");
    env.configure_std(&mut git);
    git.current_dir(notebook_root);
    git.args(["add", rel_path]);
    let add = git.output().expect("git add");
    assert!(add.status.success(), "git add failed: {:?}", add);

    let mut git = Command::new("git");
    env.configure_std(&mut git);
    git.current_dir(notebook_root);
    git.args(["commit", "-m", "add fragmented fixture"]);
    let commit = git.output().expect("git commit");
    assert!(
        commit.status.success(),
        "git commit failed: {:?} {}",
        commit,
        String::from_utf8_lossy(&commit.stderr)
    );
}

// ---- nb-api 0.4.1 adaptation regressions ----

#[test]
fn numeric_selector_round_trip_resolves_same_note() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    let add_response = add_note_via_mcp(&mut server, &env, "Numeric Note", "Body.\n");
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
    let by_selector =
        server.call_first_class("show", json!({"id": selector, "notebook": env.notebook()}));
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
    let add_response = add_note_via_mcp(&mut server, &env, "Raw H1 Title", "Body.\n");
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
    let add_response = add_note_via_mcp(&mut server, &env, "Dollar Note", "one\ntwo");
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
fn show_titles_agree_on_leading_hash_title() {
    let env = fresh_env();
    let mut server = RealNbServer::spawn(&env);
    // Raw H1 line is `# #hashtag`; upstream title_text is `#hashtag`.
    // Both MCP paths must return `#hashtag` (no double-strip on show).
    let add_response = add_note_via_mcp(&mut server, &env, "#hashtag", "Body text.\n");
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
