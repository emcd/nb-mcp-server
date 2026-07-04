use nb_api::{EditMode, SearchMode, TaskStatus};

#[test]
fn edit_mode_deserializes_lowercase_values() {
    let mode: EditMode = serde_json::from_str("\"replace\"").unwrap();
    assert_eq!(mode, EditMode::Replace);
    let mode: EditMode = serde_json::from_str("\"append\"").unwrap();
    assert_eq!(mode, EditMode::Append);
    let mode: EditMode = serde_json::from_str("\"prepend\"").unwrap();
    assert_eq!(mode, EditMode::Prepend);
}

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
