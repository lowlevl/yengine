use futures::{AsyncRead, AsyncWrite};

use crate::{
    engine::{Engine, Error, Request},
    wire::MessageAck,
};

/// Abstraction of an external [`Module`].
pub trait Module {
    /// Errors which may occur while processing messages.
    type Error: From<Error>;

    /// Install handlers and watches for processing.
    ///
    /// After installing your handlers and watches,
    /// you might want to **.await** on some signal
    /// (`SIGINT` for example) and then call [`Engine::quit`].
    /// This will make the engine stop sending us messages
    /// and exit upon processing the last one.
    fn install<I, O>(&self, engine: &Engine<I, O>) -> impl Future<Output = Result<(), Self::Error>>
    where
        I: AsyncRead + Send + Unpin,
        O: AsyncWrite + Send + Unpin;

    /// Process an incoming `watch` from the engine.
    fn on_watch<I, O>(
        &self,
        _engine: &Engine<I, O>,
        _watch: MessageAck,
    ) -> impl Future<Output = Result<(), Self::Error>>
    where
        I: AsyncRead + Send + Unpin,
        O: AsyncWrite + Send + Unpin,
    {
        futures::future::ok(())
    }

    /// Process an incoming `message` from the engine.
    fn on_message<I, O>(
        &self,
        _engine: &Engine<I, O>,
        _request: &mut Request,
    ) -> impl Future<Output = Result<bool, Self::Error>>
    where
        I: AsyncRead + Send + Unpin,
        O: AsyncWrite + Send + Unpin,
    {
        futures::future::ok(false)
    }
}
