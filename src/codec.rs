use futures::{AsyncRead, AsyncWrite};
use futures_codec::{Framed, LinesCodec};

pub struct Codec<IO> {
    framed: Framed<IO, LinesCodec>,
    peek: Option<String>,
}

impl<IO: AsyncRead + AsyncWrite> Codec<IO> {
    pub fn new(io: IO) -> Self {
        Self {
            framed: Framed::new(io, LinesCodec),
            peek: None,
        }
    }

    pub async fn send<T>(&mut self, item: T) -> anyhow::Result<()> {
        todo!()
    }

    pub async fn recv<T>(&mut self) -> anyhow::Result<Option<T>> {
        todo!()
    }
}
