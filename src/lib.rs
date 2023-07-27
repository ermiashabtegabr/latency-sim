use std::{
    env::VarError,
    num::{ParseFloatError, ParseIntError},
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("latency config parsing error: {0}")]
    LatencyConfigParseError(String),

    #[error("latency config env error: {0}")]
    LatencyConfigEnvError(#[source] VarError),
}

impl From<ParseFloatError> for Error {
    fn from(value: ParseFloatError) -> Self {
        Self::LatencyConfigParseError(value.to_string())
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Self::LatencyConfigParseError(value.to_string())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub mod config;
pub mod netem;

pub use netem::*;
