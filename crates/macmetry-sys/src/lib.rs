//! Collectors for macOS: small FFI wrappers over the kernel, IOKit and the private reporting frameworks, plus pure
//! parsers that are tested with fixtures captured from real Macs.
//!
//! The layers, from the bottom up:
//!
//! - [`value`]: a property-list-shaped tree. Parsers read this, and fixtures are loaded into it.
//! - `sysctl`, `cf` and `iokit` (macOS only): the only places that touch raw system interfaces.
//! - One module per area (added as the collectors land), each with a pure parser and a thin reader.

pub mod error;
pub mod value;

#[cfg(target_os = "macos")]
pub mod battery;
#[cfg(target_os = "macos")]
pub mod cf;
#[cfg(target_os = "macos")]
pub mod cpu;
#[cfg(target_os = "macos")]
pub mod disk;
#[cfg(target_os = "macos")]
pub mod gpu;
#[cfg(target_os = "macos")]
pub mod iokit;
#[cfg(target_os = "macos")]
pub mod mach;
#[cfg(target_os = "macos")]
pub mod memory;
#[cfg(target_os = "macos")]
pub mod network;
#[cfg(target_os = "macos")]
pub mod process;
#[cfg(target_os = "macos")]
pub mod sysctl;
#[cfg(target_os = "macos")]
pub mod system;
#[cfg(target_os = "macos")]
pub mod thermal;

pub use error::{Result, SysError};
pub use value::Value;
