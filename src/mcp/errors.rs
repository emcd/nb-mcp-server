//! Recovery-oriented error translation plus shared result and guard helpers.

use rmcp::model::{CallToolResult, Content};
use serde::Serialize;

use crate::nb::{CommitOutcome, Fingerprint, NbError, ShowNote};

use super::envelopes::ShowEnvelope;

pub(crate) fn parse_fingerprint(value: &str) -> Result<Fingerprint, NbError> {
    value.parse()
}

pub(crate) fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(message.into())])
}

/// A fenced code block opener: marker character plus run length, per
/// CommonMark rules (up to three leading spaces, run of at least three).
/// The closing fence must use the same marker, a run at least as long, and
/// carry no info text — so a shorter same-marker run (e.g. a documented
/// nested ``` sample inside a ```` fence) stays content, and an indented
/// (4+ space) marker line never opens a fence.
struct OpenFence {
    marker: char,
    len: usize,
}

/// Parse a fence marker line: up to three leading spaces, then a run of at
/// least three backticks or tildes. Returns marker, run length, and the
/// trailing remainder (info text, if any).
pub(crate) fn fence_marker(line: &str) -> Option<(char, usize, &str)> {
    let spaces = line.bytes().take_while(|&b| b == b' ').count();
    if spaces > 3 {
        return None;
    }
    let rest = &line[spaces..];
    let marker = rest.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let len = rest.chars().take_while(|&c| c == marker).count();
    if len < 3 {
        return None;
    }
    // CommonMark: a backtick fence's info string must not contain a
    // backtick, so such a line is not an opener at all (tilde fences may
    // retain backticks in info). Without this, an invalid opener could
    // swallow a following real checklist into phantom fenced content.
    let info = &rest[len..];
    if marker == '`' && info.contains('`') {
        return None;
    }
    Some((marker, len, info))
}

/// Detect todo-shaped `add` content: markdown checkboxes (`-`/`*` + `[ ]`/`[x]`/`[X]`)
/// or an ATX `Tasks` heading, in either case outside fenced code blocks.
/// Inline code spans do not exempt a line.
pub(crate) fn todo_shaped_content(content: &str) -> bool {
    let mut fence: Option<OpenFence> = None;
    for line in content.lines() {
        if let Some((marker, len, info)) = fence_marker(line) {
            match &fence {
                None => {
                    fence = Some(OpenFence { marker, len });
                },
                Some(open)
                    if open.marker == marker
                        && len >= open.len
                        && info.trim().is_empty() =>
                {
                    fence = None;
                },
                _ => {},
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed
            .strip_prefix('-')
            .or_else(|| trimmed.strip_prefix('*'))
            .map(str::trim_start)
        {
            let bytes = rest.as_bytes();
            if bytes.len() >= 3
                && bytes[0] == b'['
                && (bytes[1] == b' ' || bytes[1] == b'x' || bytes[1] == b'X')
                && bytes[2] == b']'
            {
                return true;
            }
        }
        let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
        if (1..=6).contains(&hashes) {
            let rest = &trimmed[hashes..];
            if rest.starts_with([' ', '\t']) && rest.trim() == "Tasks" {
                return true;
            }
        }
    }
    false
}

pub(crate) fn todo_shaped_add_rejection() -> CallToolResult {
    tool_error(
        "Invalid args for add.\n\
         Reason: content looks like a todo (checkbox or Tasks heading); `add` \
         creates a note, not a todo.\n\
         Hint: use the `todo` tool instead of `add` for checklists.",
    )
}

/// Present a `Result<String, NbError>` (reads) as a text tool result.
pub(crate) fn text_result(result: Result<String, NbError>) -> CallToolResult {
    match result {
        Ok(output) => CallToolResult::success(vec![Content::text(output)]),
        Err(err) => {
            CallToolResult::error(vec![Content::text(present_nb_error(&err))])
        },
    }
}

/// Present a `Result<CommitOutcome, NbError>` as a structured JSON tool result.
pub(crate) fn outcome_result(
    result: Result<CommitOutcome, NbError>,
) -> CallToolResult {
    match result {
        Ok(outcome) => json_success(&outcome),
        Err(err) => {
            CallToolResult::error(vec![Content::text(present_nb_error(&err))])
        },
    }
}

/// Present a `Result<ShowNote, NbError>` as the structured envelope.
pub(crate) fn show_result(
    result: Result<ShowNote, NbError>,
) -> CallToolResult {
    match result {
        Ok(shown) => json_success(&ShowEnvelope::from_show(shown)),
        Err(err) => {
            CallToolResult::error(vec![Content::text(present_nb_error(&err))])
        },
    }
}

/// Present a serializable typed result (lines/search lines) as JSON.
#[allow(dead_code)]
fn typed_result<T: Serialize>(result: Result<T, NbError>) -> CallToolResult {
    match result {
        Ok(value) => json_success(&value),
        Err(err) => {
            CallToolResult::error(vec![Content::text(present_nb_error(&err))])
        },
    }
}

/// Present an `Err(NbError)` as a structured error result.
pub(crate) fn nb_error_result(err: NbError) -> CallToolResult {
    CallToolResult::error(vec![Content::text(present_nb_error(&err))])
}

pub(crate) fn json_success(value: &impl Serialize) -> CallToolResult {
    match serde_json::to_value(value) {
        Ok(json) => {
            CallToolResult::success(vec![Content::json(json).unwrap_or_else(
                |_| Content::text("failed to serialize result".to_string()),
            )])
        },
        Err(_) => CallToolResult::success(vec![Content::text(
            "failed to serialize result".to_string(),
        )]),
    }
}

pub(crate) fn present_nb_error(err: &NbError) -> String {
    match err {
        NbError::UnsupportedShowTarget {
            selector,
            actual_type,
        } => format!(
            "Invalid args for show.\n\
             Reason: selector `{selector}` resolved to a non-textual type \
             ({actual_type}); `show` reads text notes only.\n\
             Hint: for folders use `folders` or `list`; for other non-text \
             types there is no MCP read operation in this release."
        ),
        NbError::DuplicateTitleHeading { title, heading } => format!(
            "Invalid args for add.\n\
             Reason: title `{title}` duplicates the first H1 heading in content \
             (`{heading}`); both produce a top-level title and double-render.\n\
             Hint: remove the duplicate H1 from content, or omit the separate \
             `title` field."
        ),
        NbError::FingerprintMismatch { target, .. } => format!(
            "Stale edit rejected: body fingerprint does not match the current \
             note `{}`.\n\
             Hint: re-read the note with `show` and retry with the fresh \
             fingerprint.",
            target.value()
        ),
        NbError::AnchorMismatch {
            target,
            number,
            guidance,
        } => format!(
            "Stale edit rejected for `{target}` line {number}: {guidance}.\n\
             Hint: re-read the note lines with `show_note_lines` and retry with \
             fresh anchors."
        ),
        NbError::OccurrenceMismatch { expected, actual } => format!(
            "Substring edit rejected: expected {expected} match(es) but found \
             {actual}.\n\
             Hint: re-read the note and correct expected_count."
        ),
        NbError::OverlappingEdits { indices } => format!(
            "Line edit batch rejected: edits at indices {indices:?} overlap.\n\
             Hint: provide disjoint edits against one original snapshot."
        ),
        NbError::InvalidLineWindow {
            offset,
            limit,
            total_lines,
        } => format!(
            "Invalid line window: offset={offset} limit={limit} but note has \
             {total_lines} line(s).\n\
             Hint: re-read with `show_note_lines` and use valid bounds."
        ),
        NbError::EmptySubstringPattern => {
            "Substring edit rejected: the pattern must be non-empty.\n\
             Hint: provide a non-empty pattern."
                .to_string()
        },
        NbError::FragmentedBody {
            fragment_count,
            guidance,
        } => format!(
            "Body operation rejected: the note has {fragment_count} body \
             fragment(s), so line/substring/body-replace operations are not \
             available. {guidance}\n\
             Hint: metadata operations (`retitle_note`, `edit_note_tags`) still \
             apply."
        ),
        NbError::DirtyBaseline { guidance } => format!(
            "Mutation rejected because the notebook worktree/index is dirty.\n\
             Hint: commit or clean the notebook worktree/index before retrying. \
             ({guidance})"
        ),
        NbError::IndeterminateCommit {
            pre_revision,
            post_revision_observed,
            guidance,
        } => format!(
            "Commit completion is unknown (pre={pre_revision}, \
             post={post_revision_observed:?}).\n\
             Hint: do not auto-retry the same plan blindly; inspect HEAD and \
             notebook status first. ({guidance})"
        ),
        NbError::RecoveryRequired {
            pre_revision,
            post_revision_observed,
            status_observed,
            preserved_paths,
            guidance,
        } => format!(
            "Recovery required after a failed commit (pre={pre_revision}, \
             post={post_revision_observed:?}, preserved={preserved_paths:?}).\n\
             Hint: inspect HEAD and status before any retry. \
             status={status_observed:?} ({guidance})"
        ),
        NbError::GateTimeout { gate, timeout_ms } => format!(
            "The notebook `{gate}` is busy; the operation timed out after \
             {timeout_ms}ms.\n\
             Hint: retry later; concurrent notebook operations are serialized."
        ),
        NbError::PathCollision { path, plan_index } => format!(
            "Path collision at `{path}` (plan op {plan_index:?}).\n\
             Hint: choose a different path or resolve the existing item first."
        ),
        NbError::PathIgnored {
            path,
            guidance,
            plan_index,
        } => format!(
            "Path `{path}` is Git-ignored (plan op {plan_index:?}); the \
             transaction cannot represent it.\n\
             Hint: {guidance}"
        ),
        NbError::PlanValidation {
            kind,
            message,
            plan_index,
        } => format!(
            "Plan validation ({kind}) failed at op {plan_index:?}: {message}"
        ),
        NbError::UnsupportedStructure { reason } => {
            format!("Unsupported notebook structure: {reason}")
        },
        NbError::NonUtf8 {
            selector,
            path,
            kind,
            mime_hint,
        } => format!(
            "Invalid args for show.\n\
             Reason: `{selector}` (`{path}`, kind={kind}) is not valid UTF-8{}; \
             structured reads carry text only, never base64 bytes.\n\
             Hint: retrieve raw bytes outside MCP with \
             `nb show {selector} --print --no-color` and handle locally.",
            mime_hint
                .as_deref()
                .map(|m| format!("; detected {m}"))
                .unwrap_or_default()
        ),
        NbError::IndexLockTimeout { path, timeout_ms } => format!(
            "The notebook index at `{path}` is busy; the lock wait timed out \
             after {timeout_ms}ms and nothing was mutated.\n\
             Hint: retry later; concurrent notebook operations are serialized."
        ),
        NbError::InvalidFingerprint { reason } => format!(
            "Invalid fingerprint: {reason}.\n\
             Hint: a fingerprint is `b3:` followed by 64 lowercase hex digits, \
             obtained from `show`."
        ),
        other => other.to_string(),
    }
}
