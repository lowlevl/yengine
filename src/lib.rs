#![doc = include_str!("../README.md")]
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, clippy::unimplemented)]

mod msg;
mod pubsub;

pub mod format;

mod engine;
pub use engine::Engine;
