//! An abstraction of the telephony [`Engine`].

use std::{
    collections::BTreeMap,
    io::{self, Stdin, Stdout},
    time::SystemTime,
};

use facet::Facet;
use futures::{
    AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, StreamExt, TryStream, TryStreamExt,
    io::{AllowStdIo, BufReader, Lines},
    lock::Mutex,
};
use subable::{Item, Subable};

use super::wire::{
    self, Connect, ConnectRole, Debug, DebugLevel, ErrorIn, Install, InstallAck, Message,
    MessageAck, Output, Quit, QuitAck, SetLocal, SetLocalAck, Uninstall, UninstallAck, Unwatch,
    UnwatchAck, Watch, WatchAck,
};
use crate::module::Module;

mod error;
pub use error::{Error, Result};

mod topic;
use topic::Topic;

mod request;
pub use request::Request;

/// A connector to the telephony engine.
pub struct Engine<I, O>
where
    I: AsyncRead + Send + Unpin,
    O: AsyncWrite + Send + Unpin,
{
    rx: Subable<Lines<BufReader<I>>, Topic>,
    tx: Mutex<O>,
}

impl Engine<AllowStdIo<Stdin>, AllowStdIo<Stdout>> {
    /// Initialize a connection to the engine via standard I/O.
    pub fn stdio() -> Self {
        Self::from_io(AllowStdIo::new(io::stdin()), AllowStdIo::new(io::stdout()))
    }
}

impl<I, O> Engine<I, O>
where
    I: AsyncRead + Send + Unpin,
    O: AsyncWrite + Send + Unpin,
{
    /// Initialize a connection to the engine with the provided I/O.
    ///
    /// If the I/O is a socket or a TCP stream, the module must register itself
    /// with a [`Self::connect`] before doing anything.
    pub fn from_io(rx: I, tx: O) -> Self {
        Self {
            rx: Subable::new(BufReader::new(rx).lines()),
            tx: tx.into(),
        }
    }

    async fn default_response(&self, recvd: &str) -> Result<()> {
        if let Ok(Message {
            id, retvalue, kv, ..
        }) = wire::from_str(recvd)
        {
            self.send(&MessageAck {
                id,
                processed: false,
                name: None,
                retvalue,
                kv,
            })
            .await
        } else if let Ok(ErrorIn { original }) = wire::from_str(recvd) {
            tracing::error!("received an error: {original}");

            // FIXME: treat error case with a correct topic

            Ok(())
        } else {
            tracing::warn!("unhandled message, dropped: {recvd}");

            Ok(())
        }
    }

    #[tracing::instrument(skip(self))]
    fn subscribe<T: Facet<'static>>(&self, topic: Topic) -> impl TryStream<Ok = T, Error = Error> {
        let queue = self.rx.subscribe(topic);

        futures::stream::try_unfold(queue, async |mut queue| {
            loop {
                match queue.try_next().await? {
                    None => break Ok(None),
                    Some(Item::Unhandled(recvd)) => self.default_response(&recvd).await?,
                    Some(Item::Subscribed(recvd)) => {
                        break Ok(Some((wire::from_str(&recvd)?, queue)));
                    }
                }
            }
        })
        .boxed() // FIXME: maybe remove this `Box`
    }

    async fn send<T: Facet<'static>>(&self, message: &T) -> Result<()> {
        let item = wire::to_string(message);

        let mut wr = self.tx.lock().await;
        wr.write_all(item.as_bytes()).await?;
        wr.write_all(b"\n").await?;

        wr.flush().await.map_err(Into::into)
    }

    /// Attach a [`Module`] to the engine to process messages and watches.
    ///
    /// This is a handy helper to abstract the management of message requests
    /// and ensure they are always acknowledged while reducing boilerplate code.
    pub async fn attach<M: Module>(self, module: M) -> Result<(), M::Error> {
        futures::try_join!(
            self.watches()
                .err_into::<M::Error>()
                .try_for_each_concurrent(None, |watch| { module.on_watch(&self, watch) }),
            self.messages()
                .err_into::<M::Error>()
                .try_for_each_concurrent(None, async |mut req| {
                    let processed = module.on_message(&self, &mut req).await?;

                    Ok(self.ack(req, processed).await?)
                }),
            module.install(&self)
        )?;

        tracing::debug!("processed all messages, exiting");

        Ok(())
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

    /// Receive _watches_ from the telephony engine.
    pub fn watches(&self) -> impl TryStream<Ok = MessageAck, Error = Error> {
        self.subscribe(Topic::Watch)
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

    fn id() -> String {
        let id = (0..12)
            .map(|_| fastrand::alphanumeric())
            .collect::<String>();

        format!("{}.{id}", env!("CARGO_PKG_NAME"))
    }

    /// Send a [`Message`] to the telephony engine for processing.
    pub async fn message(
        &self,
        name: impl Into<String>,
        retvalue: impl Into<String>,
        kv: BTreeMap<String, String>,
    ) -> Result<(bool, String, BTreeMap<String, String>)> {
        let id = Self::id();
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

    /// Receive _messages_ from the telephony engine for processing.
    pub fn messages(&self) -> impl TryStream<Ok = Request, Error = Error> {
        self.subscribe(Topic::Message).map_ok(Request::new)
    }

    /// Acknowledge the message from the engine,
    /// letting it forward it to the next handler if `!processed`.
    pub async fn ack(&self, req: Request, processed: bool) -> Result<()> {
        let original = req.into_inner();

        let message = MessageAck {
            id: original.id,
            processed,
            name: Some(original.name),
            retvalue: original.retvalue,
            kv: original.kv,
        };

        self.send(&message).await
    }

    /// Send a _connect_ message to the engine for
    /// _socket-based_ modules.
    pub async fn connect(
        &self,
        role: ConnectRole,
        channel: impl Into<Option<(String, Option<String>)>>,
    ) -> Result<()> {
        let message = Connect {
            role,
            channel: channel.into(),
        };

        self.send(&message).await
    }

    /// Output some _arbitrary text_ to engine's log, this is
    /// especially useful on _socket-based_ modules.
    pub async fn output(&self, text: impl Into<String>) -> Result<()> {
        let message = Output { text: text.into() };

        self.send(&message).await
    }

    /// Output some _debug text_ to engine's log, this is
    /// especially useful on _socket-based_ modules.
    pub async fn debug(&self, level: DebugLevel, text: impl Into<String>) -> Result<()> {
        let message = Debug {
            level,
            text: text.into(),
        };

        self.send(&message).await
    }

    /// Tell the engine we desire to stop handling messages.
    pub async fn quit(&self) -> Result<()> {
        self.send(&Quit).await?;
        self.subscribe::<QuitAck>(Topic::QuitAck)
            .try_next()
            .await?
            .ok_or(Error::UnexpectedEof)?;

        self.rx.unsubscribe_all();

        Ok(())
    }
}
