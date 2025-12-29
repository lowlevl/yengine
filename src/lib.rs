#![doc = include_str!("../README.md")]
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, clippy::unimplemented)]

pub mod format;
mod subable;

mod engine;
pub use engine::{Engine, Error, Req};
