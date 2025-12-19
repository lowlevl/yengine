use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use async_std::sync::{Condvar, Mutex};
use futures::task::AtomicWaker;

mod sub;
pub use sub::Sub;

pub trait PubSubable {
    type Topic: Debug + Clone + Hash + Eq;

    fn topic(&self) -> Self::Topic;
}

struct Inner<I: PubSubable> {
    wakers: RwLock<HashMap<I::Topic, Arc<AtomicWaker>>>,

    data: Mutex<Option<I>>,
    condvar: Condvar,
}

impl<I: PubSubable> Default for Inner<I> {
    fn default() -> Self {
        Self {
            wakers: Default::default(),

            data: Default::default(),
            condvar: Default::default(),
        }
    }
}

pub struct PubSub<I: PubSubable> {
    inner: Arc<Inner<I>>,
}

impl<I: PubSubable> Default for PubSub<I> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<I: PubSubable> PubSub<I> {
    pub fn subscribe(&self, topic: I::Topic) -> Sub<I> {
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

    pub async fn publish(&self, item: I) -> Result<(), I> {
        let topic = item.topic();

        tracing::trace!("publishing {topic:?}");

        let waker = self.inner.wakers.read().unwrap().get(&topic).cloned();
        if let Some(waker) = waker {
            let mut guard = self
                .inner
                .condvar
                .wait_until(self.inner.data.lock().await, |data| data.is_none())
                .await;

            *guard = Some(item);
            waker.wake();

            self.inner
                .condvar
                .wait_until(guard, |data| data.is_none())
                .await;
            self.inner.condvar.notify_one(); // FIXME: check if this is necessary

            Ok(())
        } else {
            Err(item)
        }
    }
}

impl<I: PubSubable> Drop for PubSub<I> {
    fn drop(&mut self) {
        for (_, waker) in self.inner.wakers.write().unwrap().drain() {
            // Wake all tasks, that will subsequently return `None`
            waker.wake();
        }
    }
}
