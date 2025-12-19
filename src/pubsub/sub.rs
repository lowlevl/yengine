use std::sync::Arc;

use futures::{FutureExt, Stream, task};

use super::{Inner, PubSubable};

pub struct Sub<I: PubSubable> {
    inner: Arc<Inner<I>>,
    topic: I::Topic,
}

impl<I: PubSubable> Sub<I> {
    pub(super) fn new(inner: Arc<Inner<I>>, topic: I::Topic) -> Self {
        Self { inner, topic }
    }
}

impl<I: PubSubable> Drop for Sub<I> {
    fn drop(&mut self) {
        tracing::trace!("unsubscribing {:?}", self.topic);

        self.inner.wakers.write().unwrap().remove(&self.topic);
    }
}

impl<I: PubSubable> Stream for Sub<I> {
    type Item = I;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if let Some(waker) = self.inner.wakers.read().unwrap().get(&self.topic) {
            // Register this task for other `Sub`s to wake
            waker.register(cx.waker());
        } else {
            return task::Poll::Ready(None);
        }

        let mut mutex = std::pin::pin!(self.inner.data.lock());
        let mut data = futures::ready!(mutex.poll_unpin(cx));

        match data.take() {
            // The topic matched ours, pop the item from the PubSub, and wakeup the publisher
            Some(item) if item.topic() == self.topic => {
                self.inner.condvar.notify_one();

                task::Poll::Ready(Some(item))
            }

            // Otherwise, place it back in the buffer, and stay pending
            value => {
                *data = value;

                task::Poll::Pending
            }
        }
    }
}
