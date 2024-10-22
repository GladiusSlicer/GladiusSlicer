#![deny(clippy::unwrap_used)]
#![deny(missing_docs)]
//!Crate for shared types between slicer and external applications like GUI and Mods

/// Error types
pub mod error;

/// Load in model files
pub mod loader;

/// Settings types
pub mod settings;

/// Common shared types
pub mod types;

/// Messages for IPC
pub mod messages;

/// Warning Types
pub mod warning;
