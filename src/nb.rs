//! nb client re-exports from `nb-api`.
//!
//! This module preserves backward compatibility for consumers using
//! `nb_mcp_server::nb::{NbClient, NbError, EditMode, SearchMode, TaskStatus}`.

pub use nb_api::{EditMode, NbClient, NbError, SearchMode, TaskStatus};
