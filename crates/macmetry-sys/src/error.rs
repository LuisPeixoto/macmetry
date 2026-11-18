//! Errors a collector can return. The sentences are written for people: the app and the command line show them as
//! they are.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SysError {
    /// This Mac does not have the hardware, or macOS does not expose it to a program without root. The interface
    /// hides the matching card instead of showing an error.
    #[error("{0} is not available on this Mac.")]
    Unsupported(&'static str),
    /// A system call failed.
    #[error("{call} failed: {reason}")]
    Os { call: &'static str, reason: String },
    /// The system returned something this version does not understand.
    #[error("Could not read {what}: {reason}")]
    Parse { what: &'static str, reason: String },
}

impl SysError {
    /// Wraps the last operating system error (`errno`) of `call`.
    pub fn os(call: &'static str) -> Self {
        Self::Os { call, reason: std::io::Error::last_os_error().to_string() }
    }

    /// Wraps a Mach or IOKit return code, which is not an `errno`.
    pub fn kern(call: &'static str, code: i32) -> Self {
        Self::Os { call, reason: format!("kernel return code {code:#x}") }
    }

    pub fn parse(what: &'static str, reason: impl Into<String>) -> Self {
        Self::Parse { what, reason: reason.into() }
    }

    /// True when the card should simply be hidden.
    pub fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported(_))
    }
}

pub type Result<T> = std::result::Result<T, SysError>;
