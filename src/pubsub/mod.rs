use std::{hash::Hash, sync::Arc};

use anyhow::Result;
use dashmap::DashMap;
use futures::{
    lock::Mutex,
    task::{self, AtomicWaker},
};

mod sub;
pub use sub::Sub;

pub trait PubSubable {
    type Topic: Copy + Hash + Eq;

    fn topic(&self) -> Self::Topic;
}

struct Inner<I: PubSubable> {
    wakers: DashMap<I::Topic, AtomicWaker>,

    signal: AtomicWaker,
    data: Mutex<Option<I>>,
}

impl<I: PubSubable> Default for Inner<I> {
    fn default() -> Self {
        Self {
            wakers: Default::default(),

            signal: Default::default(),
            data: Default::default(),
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self, topic: I::Topic) -> Sub<I> {
        if self
            .inner
            .wakers
            .insert(topic, Default::default())
            .is_some()
        {
            panic!("category already subscribed, bailing");
        }

        Sub::new(self.inner.clone(), topic)
    }

    pub async fn publish(&mut self, item: I) -> Result<(), I> {
        if let Some(waker) = self.inner.wakers.get(&item.topic()) {
            *self.inner.data.lock().await = Some(item);

            futures::future::poll_fn({
                let inner = self.inner.clone();
                let mut registered = false;

                move |cx| {
                    if !registered {
                        inner.signal.register(cx.waker());
                        registered = true;

                        waker.wake();

                        task::Poll::Pending
                    } else {
                        task::Poll::Ready(())
                    }
                }
            })
            .await;

            Ok(())
        } else {
            Err(item)
        }
    }
}

impl<I: PubSubable> Drop for PubSub<I> {
    fn drop(&mut self) {
        // NOTE: We collect here to remove reference to the DashMap
        // which would deadlock on calls to `remove`.
        for topic in self
            .inner
            .wakers
            .iter()
            .map(|task| *task.key())
            .collect::<Vec<_>>()
        {
            if let Some((_, waker)) = self.inner.wakers.remove(&topic) {
                // Wake all tasks, that will subsequently return `None`
                waker.wake();
            }
        }
    }
}
