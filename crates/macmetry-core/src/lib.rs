//! The macmetry data model: what is measured, how rates are derived from counters, how a number is named and read
//! back from a snapshot, and how the numbers are formatted. Nothing here talks to the operating system, so it builds
//! and tests on any host.

pub mod metric;
pub mod model;
pub mod rates;
pub mod ring;
pub mod units;

pub use metric::{Metric, Unit};
pub use model::*;
pub use ring::Ring;
