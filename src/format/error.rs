use facet::Shape;
use thiserror::Error;

/// A handy [`std::fmt::Result`] alias with the [`enum@Error`] type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error that may occur when (de-)serializing messages.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Reflect(#[from] facet::ReflectError),

    #[error("no message tag found")]
    MissingTag,

    #[error("expected tag `{0}`, but got `{1}`")]
    MismatchedTag(String, &'static str),

    #[error("expected value for field {0:?}")]
    MissingField(&'static Shape),

    #[error("expected a format with <key>=<value>")]
    MisformatedMap,
}
