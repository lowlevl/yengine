#![doc = include_str!("../README.md")]
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, clippy::unimplemented)]

use futures::{AsyncRead, AsyncWrite};

pub mod codec;
pub mod format;

#[derive(Debug, Default)]
pub struct Engine {}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connect(io: impl AsyncRead + AsyncWrite) -> anyhow::Result<()> {
        let codec = codec::Codec::new(io);

        Ok(())
    }
}
