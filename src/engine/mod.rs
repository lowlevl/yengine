use std::{
    collections::BTreeMap,
    io::{Stdin, Stdout},
    sync::atomic::{AtomicUsize, Ordering},
    time::SystemTime,
};

use facet::Facet;
use futures::{
    AsyncRead, AsyncWrite, SinkExt, StreamExt, TryStream, TryStreamExt, io::AllowStdIo, lock::Mutex,
};
use futures_codec::{FramedRead, FramedWrite, LinesCodec};

use crate::format::ErrorIn;

use super::{
    format::{
        self, Connect, Install, InstallAck, Message, MessageAck, Output, Quit, QuitAck, SetLocal,
        SetLocalAck, Uninstall, UninstallAck, Unwatch, UnwatchAck, Watch, WatchAck,
    },
    pubsub::PubSub,
};

mod error;
pub use error::{Error, Result};

mod msg;
use msg::{Msg, Topic};

/// The main connector to the Yate Telephone Engine.
pub struct Engine<I: AsyncRead + Unpin, O: AsyncWrite + Unpin> {
    pid: u32,
    seq: AtomicUsize,

    pubsub: Mutex<PubSub<Msg>>,

    rx: Mutex<FramedRead<I, LinesCodec>>,
    tx: Mutex<FramedWrite<O, LinesCodec>>,
}

impl Engine<AllowStdIo<Stdin>, AllowStdIo<Stdout>> {
    /// Initialize a connection to the engine via standard I/O.
    pub fn stdio() -> Self {
        let rx = FramedRead::new(AllowStdIo::new(std::io::stdin()), LinesCodec);
        let tx = FramedWrite::new(AllowStdIo::new(std::io::stdout()), LinesCodec);

        Self {
            pid: std::process::id(),
            seq: Default::default(),

            pubsub: Default::default(),

            rx: rx.into(),
            tx: tx.into(),
        }
    }
}

impl<I: AsyncRead + Unpin, O: AsyncWrite + Unpin> Engine<I, O> {
    async fn send<T: Facet<'static>>(&self, message: &T) -> Result<()> {
        let item = format::to_string(message);

        self.tx.lock().await.send(item).await.map_err(Into::into)
    }

    async fn recv<T: Facet<'static>>(&self, topic: Topic) -> Result<T> {
        let mut subscribed = self.pubsub.lock().await.subscribe(topic);

        loop {
            let Some(msg) = self.rx.lock().await.try_next().await? else {
                break Err(Error::UnexpectedEof);
            };

            match self.pubsub.lock().await.publish(Msg(msg)).await {
                Err(Msg(msg)) => {
                    if let Ok(Message {
                        id, retvalue, kv, ..
                    }) = format::from_str(&msg)
                    {
                        self.send(&MessageAck {
                            id,
                            processed: false,
                            name: None,
                            retvalue,
                            kv,
                        })
                        .await?
                    } else if let Ok(ErrorIn { original }) = format::from_str(&msg) {
                        tracing::error!("received an error from the engine: {original}");

                        // TODO: treat error case
                    } else {
                        tracing::warn!("unhandled message, dropping: {msg}");
                    }
                }

                Ok(_) => {
                    break subscribed
                        .next()
                        .await
                        .map(|item| format::from_str(&item.0))
                        .ok_or(Error::UnexpectedEof)?
                        .map_err(Into::into);
                }
            }
        }
    }

    /// Request the engine to install a message handler with the provided `priority`.
    pub async fn install(
        &self,
        priority: impl Into<Option<u64>>,
        name: impl Into<String>,
        filter: impl Into<Option<(String, Option<String>)>>,
    ) -> Result<bool> {
        let message = Install {
            priority: priority.into(),
            name: name.into(),
            filter: filter.into(),
        };

        self.send(&message).await?;
        let ack = self
            .recv::<InstallAck>(Topic::InstallAck(message.name))
            .await?;

        Ok(ack.success)
    }

    /// Request the engine to remove a previously installed handler.
    pub async fn uninstall(&self, name: impl Into<String>) -> Result<bool> {
        let message = Uninstall { name: name.into() };

        self.send(&message).await?;
        let ack = self
            .recv::<UninstallAck>(Topic::UninstallAck(message.name))
            .await?;

        Ok(ack.success)
    }

    /// Request the engine to install a message watcher.
    pub async fn watch(&self, name: impl Into<String>) -> Result<bool> {
        let message = Watch { name: name.into() };

        self.send(&message).await?;
        let ack = self.recv::<WatchAck>(Topic::WatchAck(message.name)).await?;

        Ok(ack.success)
    }

    /// Request the engine to remove a previously installed watcher.
    pub async fn unwatch(&self, name: impl Into<String>) -> Result<bool> {
        let message = Unwatch { name: name.into() };

        self.send(&message).await?;
        let ack = self
            .recv::<UnwatchAck>(Topic::UnwatchAck(message.name))
            .await?;

        Ok(ack.success)
    }

    /// Request the engine to set a _local variable_.
    pub async fn setlocal(
        &self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<bool> {
        let message = SetLocal {
            name: name.into(),
            value: Some(value.into()),
        };

        self.send(&message).await?;
        let ack = self
            .recv::<SetLocalAck>(Topic::SetLocalAck(message.name))
            .await?;

        Ok(ack.success)
    }

    /// Request the value of a _local variable_.
    pub async fn getlocal(&self, name: impl Into<String>) -> Result<String> {
        let message = SetLocal {
            name: name.into(),
            value: None,
        };

        self.send(&message).await?;
        let ack = self
            .recv::<SetLocalAck>(Topic::SetLocalAck(message.name))
            .await?;

        Ok(ack.value)
    }

    fn id(&self) -> String {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);

        format!("yengine.{}.{seq}", self.pid)
    }

    /// Send a [`Message`] to the telephony engine for processing.
    pub async fn message(
        &self,
        name: impl Into<String>,
        retvalue: impl Into<String>,
        kv: BTreeMap<String, String>,
    ) -> Result<(bool, String, BTreeMap<String, String>)> {
        let id = self.id();
        let message = Message {
            id,
            time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("time went backward, run you foul >:(")
                .as_secs(),
            name: name.into(),
            retvalue: retvalue.into(),
            kv,
        };

        self.send(&message).await?;
        let ack = self
            .recv::<MessageAck>(Topic::MessageAck(message.id))
            .await?;

        Ok((ack.processed, ack.retvalue, ack.kv))
    }

    /// Receive messages from teh telephony engine for processing.
    pub async fn messages(&self) -> impl TryStream<Ok = Message, Error = Error> {
        self.pubsub
            .lock()
            .await
            .subscribe(Topic::Message)
            .map(|Msg(msg)| format::from_str(&msg))
            .err_into()
    }

    /// Send a [`Connect`] message to the engine for
    /// _socket-based_ modules.
    pub async fn connect(
        &self,
        role: impl Into<String>,
        channel: impl Into<Option<(String, Option<String>)>>,
    ) -> Result<()> {
        let message = Connect {
            role: role.into(),
            channel: channel.into(),
        };

        self.send(&message).await
    }

    /// Output some text to engine's log, this is
    /// especially useful on _socket-based_ modules.
    pub async fn output(&self, text: impl Into<String>) -> Result<()> {
        let message = Output { text: text.into() };

        self.send(&message).await
    }

    /// Tell the engine we desire to stop handling messages.
    pub async fn quit(&self) -> Result<()> {
        self.send(&Quit).await?;
        self.recv::<QuitAck>(Topic::QuitAck).await?;

        Ok(())
    }
}
