use thiserror::Error;

/// A handy [`std::fmt::Result`] alias with the [`enum@Error`] type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error that may occur when interracting with the engine.
#[derive(Debug, Error)]
pub enum Error {
    /// An I/O error occured.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// An error occured while (de)-serializing messages.
    #[error("format error: {0}")]
    Format(#[from] crate::format::Error),

    /// The data stream was closed before expected.
    #[error("got an unexpected end of stream from engine")]
    UnexpectedEof,
}
