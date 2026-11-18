//! The `macmetry` command line.

mod args;
mod commands;
mod render;

use clap::Parser;

/// The main thread of a program has a small stack on some systems; a command tree this size has overflowed it on
/// Windows before, so the work runs on a thread with room to spare.
const STACK_BYTES: usize = 16 * 1024 * 1024;

fn main() {
    let cli = args::Cli::parse();
    let worker = std::thread::Builder::new().stack_size(STACK_BYTES).spawn(move || commands::run_command(cli.command));
    let result = match worker {
        Ok(handle) => handle.join().unwrap_or_else(|_| Err(anyhow::anyhow!("macmetry stopped unexpectedly."))),
        Err(e) => Err(anyhow::anyhow!("Could not start: {e}")),
    };
    if let Err(e) = result {
        eprintln!("macmetry: {e}");
        std::process::exit(1);
    }
}
