use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Homebrew not installed")]
    HomebrewNotFound,

    #[error("Activation script not found at {0}")]
    NoActivationScript(String),

    #[error("Brewfile not found in activation script")]
    BrewfileNotFound,

    #[error("Failed to parse Brewfile: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),
}

pub type Result<T> = std::result::Result<T, Error>;
