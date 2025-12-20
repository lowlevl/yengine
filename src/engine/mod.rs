use std::{
    collections::BTreeMap,
    io::{Stdin, Stdout},
    sync::atomic::{AtomicUsize, Ordering},
    time::SystemTime,
};

use bytes::{Buf, BufMut};
use facet::Facet;
use futures::{
    AsyncRead, AsyncWrite, SinkExt, StreamExt, TryStream, TryStreamExt, io::AllowStdIo, lock::Mutex,
};
use futures_codec::{Decoder, Encoder, FramedRead, FramedWrite};

use super::{
    format::{
        self, Connect, Install, InstallAck, Message, MessageAck, Output, Quit, QuitAck, SetLocal,
        SetLocalAck, Uninstall, UninstallAck, Unwatch, UnwatchAck, Watch, WatchAck,
    },
    subable::Subscriber,
};
use crate::{format::ErrorIn, subable::Subed};

mod error;
pub use error::{Error, Result};

mod msg;
use msg::{Msg, Topic};

struct Codec;

impl Encoder for Codec {
    type Item = Msg;
    type Error = Error;

    fn encode(
        &mut self,
        item: Self::Item,
        dst: &mut futures_codec::BytesMut,
    ) -> Result<(), Self::Error> {
        let bytes = item.0.as_bytes();

        dst.reserve(bytes.len() + 1);
        dst.put(bytes);
        dst.put_u8(b'\n');

        Ok(())
    }
}

impl Decoder for Codec {
    type Item = Msg;
    type Error = Error;

    fn decode(
        &mut self,
        src: &mut futures_codec::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        match src.iter().position(|ch| *ch == b'\n') {
            Some(pos) => {
                let buf = src.split_to(pos);
                src.advance(1);

                Ok(Some(Msg(String::from_utf8_lossy(&buf).into_owned())))
            }
            None => Ok(None),
        }
    }
}

/// The main connector to the Yate Telephone Engine.
pub struct Engine<I: AsyncRead + Unpin, O: AsyncWrite + Unpin> {
    pid: u32,
    seq: AtomicUsize,

    rx: Subscriber<FramedRead<I, Codec>>,
    tx: Mutex<FramedWrite<O, Codec>>,
}

impl Engine<AllowStdIo<Stdin>, AllowStdIo<Stdout>> {
    /// Initialize a connection to the engine via standard I/O.
    pub fn stdio() -> Self {
        let rx = FramedRead::new(AllowStdIo::new(std::io::stdin()), Codec);
        let tx = FramedWrite::new(AllowStdIo::new(std::io::stdout()), Codec);

        Self {
            pid: std::process::id(),
            seq: Default::default(),

            rx: Subscriber::new(rx),
            tx: tx.into(),
        }
    }
}

impl<I: AsyncRead + Unpin, O: AsyncWrite + Unpin> Engine<I, O> {
    async fn send<T: Facet<'static>>(&self, message: &T) -> Result<()> {
        let item = format::to_string(message);

        self.tx.lock().await.send(Msg(item)).await
    }

    async fn default_response(&self, recvd: &str) -> Result<()> {
        if let Ok(Message {
            id, retvalue, kv, ..
        }) = format::from_str(recvd)
        {
            self.send(&MessageAck {
                id,
                processed: false,
                name: None,
                retvalue,
                kv,
            })
            .await
        } else if let Ok(ErrorIn { original }) = format::from_str(recvd) {
            tracing::error!("received an error from the engine: {original}");

            // TODO: treat error case

            Ok(())
        } else {
            tracing::debug!("unhandled message, dropping: {recvd}");

            Ok(())
        }
    }

    #[tracing::instrument(skip(self))]
    fn subscribe<T: Facet<'static>>(&self, topic: Topic) -> impl TryStream<Ok = T, Error = Error> {
        let sub = self.rx.subscribe(topic);

        futures::stream::try_unfold(sub, async |mut sub| {
            loop {
                let Some(recvd) = sub.try_next().await? else {
                    break Ok(None);
                };

                match recvd {
                    Subed::Match(recvd) => break Ok(Some((format::from_str(&recvd.0)?, sub))),
                    Subed::Default(recvd) => self.default_response(&recvd.0).await?,
                }
            }
        })
        .boxed_local()
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
            .subscribe::<InstallAck>(Topic::InstallAck(message.name))
            .try_next()
            .await?
            .ok_or(Error::UnexpectedEof)?;

        Ok(ack.success)
    }

    /// Request the engine to remove a previously installed handler.
    pub async fn uninstall(&self, name: impl Into<String>) -> Result<bool> {
        let message = Uninstall { name: name.into() };

        self.send(&message).await?;
        let ack = self
            .subscribe::<UninstallAck>(Topic::UninstallAck(message.name))
            .try_next()
            .await?
            .ok_or(Error::UnexpectedEof)?;

        Ok(ack.success)
    }

    /// Request the engine to install a message watcher.
    pub async fn watch(&self, name: impl Into<String>) -> Result<bool> {
        let message = Watch { name: name.into() };

        self.send(&message).await?;
        let ack = self
            .subscribe::<WatchAck>(Topic::WatchAck(message.name))
            .try_next()
            .await?
            .ok_or(Error::UnexpectedEof)?;

        Ok(ack.success)
    }

    /// Request the engine to remove a previously installed watcher.
    pub async fn unwatch(&self, name: impl Into<String>) -> Result<bool> {
        let message = Unwatch { name: name.into() };

        self.send(&message).await?;
        let ack = self
            .subscribe::<UnwatchAck>(Topic::UnwatchAck(message.name))
            .try_next()
            .await?
            .ok_or(Error::UnexpectedEof)?;

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
            .subscribe::<SetLocalAck>(Topic::SetLocalAck(message.name))
            .try_next()
            .await?
            .ok_or(Error::UnexpectedEof)?;

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
            .subscribe::<SetLocalAck>(Topic::SetLocalAck(message.name))
            .try_next()
            .await?
            .ok_or(Error::UnexpectedEof)?;

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
            .subscribe::<MessageAck>(Topic::MessageAck(message.id))
            .try_next()
            .await?
            .ok_or(Error::UnexpectedEof)?;

        Ok((ack.processed, ack.retvalue, ack.kv))
    }

    /// Receive messages from teh telephony engine for processing.
    pub fn messages(&self) -> impl TryStream<Ok = Message, Error = Error> {
        self.subscribe(Topic::Message)
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
        self.subscribe::<QuitAck>(Topic::QuitAck)
            .try_next()
            .await?
            .ok_or(Error::UnexpectedEof)?;

        Ok(())
    }
}
