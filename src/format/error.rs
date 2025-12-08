use thiserror::Error;

/// A handy [`std::fmt::Result`] alias with the [`enum@Error`] type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error that may occur when (de-)serializing messages.
#[derive(Debug, Error)]
pub enum Error {
    /// An error during reflection.
    #[error(transparent)]
    Reflect(#[from] facet::ReflectError),

    /// The message didn't include a tag.
    #[error("no message tag found")]
    MissingTag,

    /// The message didn't include required tag.
    #[error("message tag didn't match struct tag")]
    MismatchedTag,

    /// The message didn't include the required value.
    #[error("expected value, but input is exhausted")]
    MissingValue,

    /// The format of the map wasn't respected.
    #[error("expected a format with <key>=<value>")]
    MisformatedMap,
}
