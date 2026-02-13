#![doc = include_str!("../README.md")]
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, clippy::unimplemented)]

pub mod engine;
pub mod wire;

mod module;
pub use module::Module;

pub use engine::Engine;
