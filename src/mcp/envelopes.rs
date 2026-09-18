//! Text-first result envelopes (native `String` wire, dual-path title mapping).

use rmcp::model::{CallToolResult, Content};
use serde::Serialize;

use crate::nb::{
    DocumentKind, Fingerprint, LineAnchor, LineEdit, LineEol, NbError, SearchNoteLines, ShowNote,
    ShowNoteLines, TodoState,
};

use super::errors::{json_success, present_nb_error};
use super::schemas::McpLineEdit;

/// Text-first slim `show` envelope (native `String` wire, normalized title).
#[derive(Debug, Serialize)]
pub(crate) struct ShowEnvelope {
    selector: String,
    path: String,
    kind: DocumentKind,
    todo_state: Option<TodoState>,
    title: Option<String>,
    tags: Vec<String>,
    body: String,
    body_contiguous: bool,
    fingerprint: Fingerprint,
    numeric_id: Option<u32>,
}

/// Normalize a raw title H1 line to plain title text: strip the trailing
/// newline, leading `#` run, and surrounding whitespace. Used ONLY for the
/// raw `ShowNoteLines.title` H1 line (upstream exposes no `title_text` on
/// that type). MUST NOT be applied to the already-normalized upstream
/// `ShowNote.title_text` — a genuine leading-`#` title (raw `# #hashtag`)
/// would otherwise lose it on one path but not the other.
fn normalize_title_text(raw: &str) -> String {
    raw.trim_end_matches('\n')
        .trim_end_matches('\r')
        .trim_start_matches('#')
        .trim()
        .to_string()
}

impl ShowEnvelope {
    pub(crate) fn from_show(shown: ShowNote) -> Self {
        // `show` maps the already-normalized upstream `title_text` directly;
        // the raw `title` H1 line is never exposed. Body/source arrive as
        // native `String` (non-UTF-8 surfaces as typed `NonUtf8` upstream,
        // translated in `present_nb_error`).
        Self {
            selector: shown.selector,
            path: shown.path,
            kind: shown.kind,
            todo_state: shown.todo_state,
            title: shown.title_text,
            tags: shown.tags,
            body: shown.body,
            body_contiguous: shown.body_contiguous,
            fingerprint: shown.fingerprint,
            numeric_id: shown.numeric_id,
        }
    }
}

/// Text-first line envelope with document-level EOL declaration.
#[derive(Debug, Serialize)]
struct ShowNoteLinesEnvelope {
    selector: String,
    path: String,
    kind: DocumentKind,
    total_lines: u32,
    offset: u32,
    limit: u32,
    next_offset: Option<u32>,
    lines: Vec<NoteLineEnvelope>,
    title: Option<String>,
    body_fingerprint: Fingerprint,
    eol: Option<LineEol>,
    has_final_eol: bool,
    numeric_id: Option<u32>,
}

#[derive(Debug, Serialize)]
struct NoteLineEnvelope {
    number: u32,
    anchor: LineAnchor,
    text: String,
}

#[derive(Debug, Serialize)]
struct SearchNoteLinesEnvelope {
    selector: String,
    path: String,
    kind: DocumentKind,
    hits: Vec<HitEnvelope>,
    body_fingerprint: Fingerprint,
}

#[derive(Debug, Serialize)]
struct HitEnvelope {
    number: u32,
    anchor: LineAnchor,
    start_byte: u32,
    end_byte: u32,
    text: String,
}

pub(crate) fn convert_mcp_line_edits(edits: Vec<McpLineEdit>) -> Vec<LineEdit> {
    edits
        .into_iter()
        .map(|e| match e {
            // Bare-text content; the library appends document `eol` bytes.
            McpLineEdit::Insert { at, content } => LineEdit::Insert { at, content },
            McpLineEdit::Delete { start, end } => LineEdit::Delete { start, end },
            McpLineEdit::Replace {
                start,
                end,
                content,
            } => LineEdit::Replace {
                start,
                end,
                content,
            },
        })
        .collect()
}

pub(crate) fn show_note_lines_result(result: Result<ShowNoteLines, NbError>) -> CallToolResult {
    match result {
        Ok(envelope) => json_success(&ShowNoteLinesEnvelope::from_wire(envelope)),
        Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
    }
}

pub(crate) fn search_note_lines_result(result: Result<SearchNoteLines, NbError>) -> CallToolResult {
    match result {
        Ok(shown) => match SearchNoteLinesEnvelope::from_wire(shown) {
            Ok(envelope) => json_success(&envelope),
            Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
        },
        Err(err) => CallToolResult::error(vec![Content::text(present_nb_error(&err))]),
    }
}

impl ShowNoteLinesEnvelope {
    fn from_wire(shown: ShowNoteLines) -> Self {
        // Upstream `ShowNoteLines.title` is the raw H1 line (no `title_text`
        // on this type); retain the normalization mapping so MCP `title`
        // matches the `show` path. Lines/eol/numeric_id pass through.
        let title = shown.title.as_deref().map(normalize_title_text);
        let lines = shown
            .lines
            .into_iter()
            .map(|l| NoteLineEnvelope {
                number: l.number,
                anchor: l.anchor,
                text: l.text,
            })
            .collect();
        Self {
            selector: shown.selector,
            path: shown.path,
            kind: shown.kind,
            total_lines: shown.total_lines,
            offset: shown.offset,
            limit: shown.limit,
            next_offset: shown.next_offset,
            lines,
            title,
            body_fingerprint: shown.body_fingerprint,
            eol: shown.eol,
            has_final_eol: shown.has_final_eol,
            numeric_id: shown.numeric_id,
        }
    }
}

impl SearchNoteLinesEnvelope {
    fn from_wire(shown: SearchNoteLines) -> Result<Self, NbError> {
        // No EOL fields and no `numeric_id` on this upstream type by design;
        // hit text passes through as native `String`.
        let hits = shown
            .hits
            .into_iter()
            .map(|h| {
                let text = h.text.ok_or_else(|| NbError::ValidationError {
                    reason: format!(
                        "hit line {} text is absent; expected plain string (upstream None)",
                        h.number
                    ),
                    location: None,
                })?;
                Ok(HitEnvelope {
                    number: h.number,
                    anchor: h.anchor,
                    start_byte: h.start_byte,
                    end_byte: h.end_byte,
                    text,
                })
            })
            .collect::<Result<Vec<_>, NbError>>()?;
        Ok(Self {
            selector: shown.selector,
            path: shown.path,
            kind: shown.kind,
            hits,
            body_fingerprint: shown.body_fingerprint,
        })
    }
}
