//! nb client re-exports from `nb-api`.
//!
//! Preserves the re-export and import location for the listed types
//! so dependents that named `nb_mcp_server::nb::{NbClient, NbError,
//! EditMode, SearchMode, TaskStatus}` continue to compile. Method
//! names on `NbClient` follow the upstream `nb-api` 0.2 surface.

pub use nb_api::{EditMode, NbClient, NbError, SearchMode, TaskStatus};
