use nb_mcp_server::nb::{Fingerprint, NoteTarget, Occurrence, SearchMode, TaskStatus};

#[test]
fn task_status_deserializes_lowercase_values() {
    let status: TaskStatus = serde_json::from_str("\"open\"").unwrap();
    assert_eq!(status, TaskStatus::Open);
    let status: TaskStatus = serde_json::from_str("\"closed\"").unwrap();
    assert_eq!(status, TaskStatus::Closed);
}

#[test]
fn search_mode_deserializes_lowercase_values() {
    let mode: SearchMode = serde_json::from_str("\"any\"").unwrap();
    assert_eq!(mode, SearchMode::Any);
    let mode: SearchMode = serde_json::from_str("\"all\"").unwrap();
    assert_eq!(mode, SearchMode::All);
}

#[test]
fn note_target_deserializes_tagged_variants() {
    let selector: NoteTarget =
        serde_json::from_str(r#"{"type":"selector","value":"my-note.md"}"#).unwrap();
    assert_eq!(selector.value(), "my-note.md");
    let path: NoteTarget =
        serde_json::from_str(r#"{"type":"path","value":"folder/my-note.md"}"#).unwrap();
    assert_eq!(path.value(), "folder/my-note.md");
}

#[test]
fn occurrence_deserializes_tagged_variants() {
    let first: Occurrence = serde_json::from_str(r#"{"type":"first"}"#).unwrap();
    assert!(matches!(first, Occurrence::First));
    let all: Occurrence = serde_json::from_str(r#"{"type":"all"}"#).unwrap();
    assert!(matches!(all, Occurrence::All));
    let nth: Occurrence = serde_json::from_str(r#"{"type":"nth","n":3}"#).unwrap();
    assert!(matches!(nth, Occurrence::Nth { n: 3 }));
}

#[test]
fn fingerprint_validates_and_serializes_canonical() {
    let fp: Fingerprint = serde_json::from_str(
        "\"b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"",
    )
    .unwrap();
    assert!(fp.as_str().starts_with("b3:"));
    assert_eq!(fp.as_hex().len(), 64);
    let serialized = serde_json::to_string(&fp).unwrap();
    assert!(serialized.starts_with("\"b3:"));
    let invalid: Result<Fingerprint, _> = serde_json::from_str("\"not-a-fingerprint\"");
    assert!(invalid.is_err());
}

#[test]
fn re_exported_types_resolve_from_nb_module() {
    let search: SearchMode = serde_json::from_str("\"any\"").unwrap();
    assert_eq!(search, SearchMode::Any);
    let status: TaskStatus = serde_json::from_str("\"open\"").unwrap();
    assert_eq!(status, TaskStatus::Open);
    let target: NoteTarget = serde_json::from_str(r#"{"type":"selector","value":"x"}"#).unwrap();
    assert_eq!(target.value(), "x");
}
