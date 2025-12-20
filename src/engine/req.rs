use std::ops::{Deref, DerefMut};

use crate::format::Message;

#[derive(Debug)]
pub struct Req {
    inner: Option<Message>,
}

impl Req {
    pub(super) fn new(inner: Message) -> Self {
        Self { inner: Some(inner) }
    }

    pub(super) fn into_inner(mut self) -> Message {
        self.inner.take().expect("Req was already into_inner'ed")
    }
}

impl Deref for Req {
    type Target = Message;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().expect("Req was already into_inner'ed")
    }
}

impl DerefMut for Req {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().expect("Req was already into_inner'ed")
    }
}

impl Drop for Req {
    fn drop(&mut self) {
        if let Some(inner) = &self.inner {
            panic!("message `{inner:?}` was not ack'ed, every message must be ack'ed");
        }
    }
}
