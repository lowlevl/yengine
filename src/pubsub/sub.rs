use std::sync::Arc;

use futures::{FutureExt, Stream, task};

use super::{Inner, PubSubable};

pub struct Sub<I: PubSubable> {
    inner: Arc<Inner<I>>,
    topic: I::Topic,
}

impl<I: PubSubable> Sub<I> {
    pub fn new(inner: Arc<Inner<I>>, topic: I::Topic) -> Self {
        Self { inner, topic }
    }
}

impl<I: PubSubable> Drop for Sub<I> {
    fn drop(&mut self) {
        self.inner.wakers.remove(&self.topic);
    }
}

impl<I: PubSubable> Stream for Sub<I> {
    type Item = I;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if let Some(waker) = self.inner.wakers.get(&self.topic) {
            // Register this task for other `Sub`s to wake
            waker.register(cx.waker());
        } else {
            return task::Poll::Ready(None);
        }

        let mut data = futures::ready!(self.inner.data.lock().poll_unpin(cx));
        match data.take() {
            // The topic matched ours, pop the item from the PubSub, and wakeup the publisher
            Some(item) if item.topic() == self.topic => {
                self.inner.signal.wake();

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
