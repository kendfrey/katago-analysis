//! A wrapper around KataGo's analysis protocol.
//!
//! See [KataGo Parallel Analysis Engine](https://github.com/lightvector/KataGo/blob/master/docs/Analysis_Engine.md)
//! for official documentation of the analysis engine.

#![warn(missing_docs)]

use std::io;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod engine;

mod config;
pub use config::*;

mod rules;
pub use rules::*;

/// Player colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Player {
    /// The black player.
    #[serde(rename = "B")]
    Black,
    /// The white player.
    #[serde(rename = "W")]
    White,
}

/// Errors that can occur while interacting with the analysis engine.
#[derive(Debug, Error)]
pub enum Error {
    /// An I/O error occurred while launching, writing to, or reading from the analysis engine.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// An error occurred while serializing or deserializing a message.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// The engine's stdin was unavailable after launch.
    #[error("stdin unavailable")]
    StdinUnavailable,

    /// The engine's stdout was unavailable after launch.
    #[error("stdout unavailable")]
    StdoutUnavailable,

    /// The specified config setting's value could not be serialized.
    #[error("unserializable config value for key {0}")]
    UnserializableConfig(String),
}
