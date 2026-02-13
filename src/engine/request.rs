use std::ops::{Deref, DerefMut};

use crate::wire::Message;

#[cfg(doc)]
use super::Engine;

/// A request to process a [`Message`], it _must_ be ack'd or the messages will block server-side.
#[derive(Debug)]
#[must_use = "messages must be ack'ed, even if not processed with Engine::ack"]
pub struct Request {
    inner: Option<Message>,
}

impl Request {
    pub(super) fn new(inner: Message) -> Self {
        Self { inner: Some(inner) }
    }

    pub(super) fn into_inner(mut self) -> Message {
        self.inner.take().expect("Req was already into_inner'ed")
    }
}

impl Deref for Request {
    type Target = Message;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().expect("Req was already into_inner'ed")
    }
}

impl DerefMut for Request {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().expect("Req was already into_inner'ed")
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        if let Some(inner) = &self.inner {
            tracing::error!("message `{inner:?}` was not ack'ed, every message must be ack'ed");
        }
    }
}
