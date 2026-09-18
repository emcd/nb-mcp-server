//! MCP tool surface: schemas, envelopes, dispatch, errors, and help.

mod dispatch;
mod envelopes;
mod errors;
mod help;
mod schemas;
mod server;

use crate::nb::NbClient;

#[derive(Clone)]
pub(crate) struct McpServer {
    nb: NbClient,
}

pub use server::run;
