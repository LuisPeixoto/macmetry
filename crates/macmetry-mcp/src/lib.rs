//! A [Model Context Protocol](https://modelcontextprotocol.io) server over standard input and output, so AI assistants
//! can read how the Mac they run on is doing, with the same collectors, history and numbers the app and the command
//! line use.
//!
//! Start it with `macmetry mcp`. **Every tool only reads.** Nothing is offered that changes the Mac, and network
//! addresses are never sent to the assistant.

mod args;
mod privacy;
mod prompts;
mod resources;
mod server;
mod watch;

pub use server::MacmetryServer;

use rmcp::ServiceExt;
use std::path::PathBuf;

/// Serves the tools on stdin and stdout until the client disconnects, then ends every watch it started.
pub async fn serve(history: PathBuf) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = MacmetryServer::new(history);
    let handle = server.clone();
    let service = server.serve(rmcp::transport::stdio()).await?;
    // The client closing standard input ends the session, and so does Ctrl+C or a termination signal.
    tokio::select! {
        result = service.waiting() => { result?; }
        _ = tokio::signal::ctrl_c() => {}
    }
    handle.shutdown();
    Ok(())
}
