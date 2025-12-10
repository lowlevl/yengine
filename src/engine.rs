use std::{
    collections::HashMap,
    io::{self, Stdin, Stdout},
    sync::atomic::{AtomicUsize, Ordering},
    time::SystemTime,
};

use futures::{
    AsyncRead, AsyncWrite, SinkExt, StreamExt, TryStream, io::AllowStdIo, lock::Mutex,
    stream::Peekable,
};
use futures_codec::{FramedRead, FramedWrite, LinesCodec};

use super::format::{
    self, Connect, Install, Message, Output, Quit, SetLocal, Uninstall, Unwatch, Watch,
};

/// The main connector to the Yate Telephone Engine.
pub struct Engine<I: AsyncRead + Unpin, O: AsyncWrite + Unpin> {
    pid: u32,
    seq: AtomicUsize,

    rx: Peekable<FramedRead<I, LinesCodec>>,
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

            rx: rx.peekable(),
            tx: tx.into(),
        }
    }
}

impl<I: AsyncRead + Unpin, O: AsyncWrite + Unpin> Engine<I, O> {
    async fn wait(&self) -> io::Result<()> {
        Ok(())
    }

    /// Request the engine to install a message handler with the provided `priority`.
    pub async fn install(
        &self,
        priority: impl Into<Option<u64>>,
        name: impl Into<String>,
        filter: impl Into<Option<(String, Option<String>)>>,
    ) -> io::Result<bool> {
        let message = Install {
            priority: priority.into(),
            name: name.into(),
            filter: filter.into(),
        };

        self.tx
            .lock()
            .await
            .send(format::to_string(&message))
            .await?;

        // TODO: await response

        unimplemented!()
    }

    /// Request the engine to remove a previously installed handler.
    pub async fn uninstall(&self, name: impl Into<String>) -> io::Result<bool> {
        let message = Uninstall { name: name.into() };

        self.tx
            .lock()
            .await
            .send(format::to_string(&message))
            .await?;

        // TODO: await response

        unimplemented!()
    }

    /// Request the engine to install a message watcher.
    pub async fn watch(&self, name: impl Into<String>) -> io::Result<bool> {
        let message = Watch { name: name.into() };

        self.tx
            .lock()
            .await
            .send(format::to_string(&message))
            .await?;

        // TODO: await response

        unimplemented!()
    }

    /// Request the engine to remove a previously installed watcher.
    pub async fn unwatch(&self, name: impl Into<String>) -> io::Result<bool> {
        let message = Unwatch { name: name.into() };

        self.tx
            .lock()
            .await
            .send(format::to_string(&message))
            .await?;

        // TODO: await response

        unimplemented!()
    }

    /// Request the engine to set a _local variable_.
    pub async fn setlocal(
        &self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> io::Result<bool> {
        let message = SetLocal {
            name: name.into(),
            value: Some(value.into()),
        };

        self.tx
            .lock()
            .await
            .send(format::to_string(&message))
            .await?;

        // TODO: await response

        unimplemented!()
    }

    /// Request the value of a _local variable_.
    pub async fn getlocal(&self, name: impl Into<String>) -> io::Result<String> {
        let message = SetLocal {
            name: name.into(),
            value: None,
        };

        self.tx
            .lock()
            .await
            .send(format::to_string(&message))
            .await?;

        // TODO: await response

        unimplemented!()
    }

    /// Send a [`Message`] to the telephony engine for processing.
    pub async fn message(
        &self,
        name: impl Into<String>,
        retvalue: impl Into<String>,
        kv: HashMap<String, String>,
    ) -> io::Result<(bool, String, HashMap<String, String>)> {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let id = format!("yengine.{}.{seq}", self.pid);

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

        self.tx
            .lock()
            .await
            .send(format::to_string(&message))
            .await?;

        // TODO: await response

        unimplemented!()
    }

    /// Receive messages from teh telephony engine for processing.
    pub async fn messages(&self) -> impl TryStream<Ok = Message, Error = io::Error> {
        futures::stream::empty()
    }

    /// Send a [`Connect`] message to the engine for
    /// _socket-based_ modules.
    pub async fn connect(
        &self,
        role: impl Into<String>,
        id: impl Into<Option<String>>,
        type_: impl Into<Option<String>>,
    ) -> io::Result<()> {
        let message = Connect {
            role: role.into(),
            id: id.into(),
            type_: type_.into(),
        };

        self.tx.lock().await.send(format::to_string(&message)).await
    }

    /// Output some text to engine's log, this is
    /// especially useful on _socket-based_ modules.
    pub async fn output(&self, text: impl Into<String>) -> io::Result<()> {
        let message = Output { text: text.into() };

        self.tx.lock().await.send(format::to_string(&message)).await
    }

    /// Tell the engine we desire to stop handling messages.
    pub async fn quit(&self) -> io::Result<()> {
        self.tx.lock().await.send(format::to_string(&Quit)).await?;

        // TODO: await response
        Ok(())
    }
}
