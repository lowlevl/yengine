use thiserror::Error;

/// A handy [`std::fmt::Result`] alias with the [`enum@Error`] type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error that may occur when interracting with the engine.
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("format error: {0}")]
    Format(#[from] crate::format::Error),

    #[error("got an unexpected end of stream from engine")]
    UnexpectedEof,
}
