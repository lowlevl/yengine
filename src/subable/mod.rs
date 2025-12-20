use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::{Arc, RwLock},
};

use futures::{StreamExt, TryStream, lock::Mutex, stream::Peekable, task::AtomicWaker};

mod sub;
pub use sub::{Sub, Subed};

pub trait Subable {
    type Topic: Debug + Clone + Hash + Eq;

    fn topic(&self) -> Self::Topic;
}

struct Inner<S: TryStream>
where
    S::Ok: Subable,
{
    wakers: RwLock<HashMap<<S::Ok as Subable>::Topic, Arc<AtomicWaker>>>,
    stream: Mutex<Peekable<S>>,
}

pub struct Subscriber<S: TryStream>
where
    S::Ok: Subable,
{
    inner: Arc<Inner<S>>,
}

impl<S: TryStream> Subscriber<S>
where
    S::Ok: Subable,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: Inner {
                wakers: Default::default(),
                stream: stream.peekable().into(),
            }
            .into(),
        }
    }

    pub fn subscribe(&self, topic: <S::Ok as Subable>::Topic) -> Sub<S> {
        if self
            .inner
            .wakers
            .write()
            .unwrap()
            .insert(topic.clone(), Default::default())
            .is_some()
        {
            panic!("category already subscribed, bailing");
        }

        tracing::trace!("subscribing {topic:?}");

        Sub::new(self.inner.clone(), topic)
    }
}

impl<S: TryStream> Drop for Subscriber<S>
where
    S::Ok: Subable,
{
    fn drop(&mut self) {
        for (_, waker) in self.inner.wakers.write().unwrap().drain() {
            // Wake all tasks, that will subsequently return `None`
            waker.wake();
        }
    }
}
