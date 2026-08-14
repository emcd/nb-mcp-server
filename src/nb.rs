//! nb client re-exports from `nb-api`.
//!
//! Preserves the re-export and import location for the listed types
//! so dependents that named `nb_mcp_server::nb::{NbClient, NbError,
//! SearchMode, TaskStatus, ...}` continue to compile. Method names on
//! `NbClient` follow the upstream `nb-api` 0.3 surface.

pub use nb_api::{
    BodyFragment, BoundaryAt, ByteString, CommitOutcome, DocumentKind, Fingerprint, LineAnchor,
    LineEdit, LinePosition, LineRef, NbClient, NbError, NoteTarget, Occurrence, SearchMode,
    SearchNoteLines, ShowNote, ShowNoteLines, TaskStatus, TodoState,
};
