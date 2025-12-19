use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use futures::{
    lock::Mutex,
    task::{self, AtomicWaker},
};

mod sub;
pub use sub::Sub;

pub trait PubSubable {
    type Topic: Debug + Clone + Hash + Eq;

    fn topic(&self) -> Self::Topic;
}

struct Inner<I: PubSubable> {
    wakers: RwLock<HashMap<I::Topic, Arc<AtomicWaker>>>,

    signal: AtomicWaker,
    data: Mutex<Option<I>>,
    // FIXME: Condvar
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

    pub async fn publish(&mut self, item: I) -> Result<(), I> {
        let topic = item.topic();

        tracing::trace!("publishing {topic:?}");

        let waker = self.inner.wakers.read().unwrap().get(&topic).cloned();
        if let Some(waker) = waker {
            if self.inner.data.lock().await.replace(item).is_some() {
                unreachable!("replaced a Some() value, aborting");
            }

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
        for (_, waker) in self.inner.wakers.write().unwrap().drain() {
            // Wake all tasks, that will subsequently return `None`
            waker.wake();
        }
    }
}
