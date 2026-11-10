//! Core library for Storage Manager: RAM monitoring, disk analysis,
//! cleanup planning and alert rules. No UI code lives here, so the same logic
//! serves the CLI and the desktop app.

pub mod alerts;
pub mod cleanup;
pub mod disk;
pub mod error;
pub mod ram;
pub mod util;

pub use error::{Error, Result};
