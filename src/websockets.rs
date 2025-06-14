use embedded_io_async::{Read, Write};
use framez::{
    Framed,
    state::{ReadState, ReadWriteState, WriteState},
};
use rand::RngCore;

use crate::{
    Message, Options, WebsocketsCore,
    codec::FramesCodec,
    error::{ReadError, WriteError},
};

#[derive(Debug)]
pub struct Websockets<'buf, RW, Rng> {
    core: WebsocketsCore<'buf, RW, Rng>,
}

impl<'buf, RW, Rng> Websockets<'buf, RW, Rng> {
    /// Creates a new [`Websockets`] for a client after a successful handshake.
    pub fn client(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebsocketsCore::client(inner, rng, read_buffer, write_buffer, fragments_buffer),
        }
    }

    /// Creates a new [`Websockets`] for a server after a successful handshake.
    pub fn server(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebsocketsCore::server(inner, rng, read_buffer, write_buffer, fragments_buffer),
        }
    }

    pub async fn connect<const N: usize>(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
        options: Options<'_, '_>,
    ) -> Result<Self, ()>
    where
        RW: Read + Write,
        Rng: RngCore,
    {
        Self::client(inner, rng, read_buffer, write_buffer, fragments_buffer)
            .handshake::<N>(options)
            .await
            .map_err(|_| ())
    }

    /// Returns reference to the reader/writer.
    #[inline]
    pub const fn inner(&self) -> &RW {
        self.core.inner()
    }

    /// Returns mutable reference to the reader/writer.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut RW {
        self.core.inner_mut()
    }

    /// Consumes the [`Websockets`] and returns the reader/writer.
    #[inline]
    pub fn into_inner(self) -> RW {
        self.core.into_inner()
    }

    pub async fn handshake<const N: usize>(self, options: Options<'_, '_>) -> Result<Self, ()>
    where
        RW: Read + Write,
        Rng: RngCore,
    {
        Ok(Self {
            core: self.core.handshake::<N>(options).await?,
        })
    }

    /// Tries to read a message from the underlying reader.
    ///
    /// # Return value
    ///
    /// - `Some(Ok(None))` if the buffer is not framable or the fragments do not add up to a complete message. Call `maybe_next` again to read more bytes.
    /// - `Some(Ok(Some(message)))` if a frame was successfully decoded. Call `maybe_next` again to read more bytes.
    /// - `Some(Err(error))` if an error occurred. The caller should stop reading.
    /// - `None` if eof was reached. The caller should stop reading.
    ///
    /// # Usage
    ///
    /// See [`next!`](crate::next!).
    ///
    /// # Example
    ///
    /// ```rust
    /// use core::{error::Error};
    ///
    /// use websocketz::{Websockets, mock::Noop, next};
    ///
    /// async fn read() -> Result<(), Box<dyn Error>> {
    ///     let stream = Noop;
    ///     let rng = Noop;
    ///     let r_buf = &mut [0u8; 1024];
    ///     let w_buf = &mut [0u8; 1024];
    ///     let f_buf = &mut [0u8; 1024];
    ///     
    ///     let mut websocketz = Websockets::client(stream, rng, r_buf, w_buf, f_buf);
    ///     
    ///     while let Some(message) = next!(websocketz).transpose()? {
    ///         println!("Message: {message:?}");
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn maybe_next<'this>(
        &'this mut self,
    ) -> Option<Result<Option<Message<'this>>, ReadError<RW::Error>>>
    where
        RW: Read,
    {
        self.core.maybe_next().await
    }

    pub async fn send(&mut self, message: Message<'_>) -> Result<(), WriteError<RW::Error>>
    where
        RW: Write,
        Rng: RngCore,
    {
        self.core.send(message).await
    }

    pub async fn send_fragmented(
        &mut self,
        message: Message<'_>,
        fragment_size: usize,
    ) -> Result<(), WriteError<RW::Error>>
    where
        RW: Write,
        Rng: RngCore,
    {
        self.core.send_fragmented(message, fragment_size).await
    }

    pub fn split_with<F, R, W>(
        self,
        f: F,
    ) -> (WebsocketsRead<'buf, R>, WebsocketsWrite<'buf, W, Rng>)
    where
        F: FnOnce(RW) -> (R, W),
    {
        let (codec, inner, state) = self.core.framed.into_parts();
        let (read_codec, write_codec) = codec.split();

        let (read_inner, write_inner) = f(inner);

        let framed_read = Framed::from_parts(
            read_codec,
            read_inner,
            ReadWriteState::new(state.read, WriteState::new(&mut [])),
        );

        let framed_write = Framed::from_parts(
            write_codec,
            write_inner,
            ReadWriteState::new(ReadState::new(&mut []), state.write),
        );

        (
            WebsocketsRead::from_framed(framed_read, self.core.fragments_buffer),
            WebsocketsWrite::from_framed(framed_write),
        )
    }
}

#[derive(Debug)]
pub struct WebsocketsRead<'buf, RW> {
    core: WebsocketsCore<'buf, RW, ()>,
}

impl<'buf, RW> WebsocketsRead<'buf, RW> {
    fn from_framed(
        framed: Framed<'buf, FramesCodec<()>, RW>,
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebsocketsCore::from_framed(framed, fragments_buffer),
        }
    }

    /// Creates a new [`WebsocketsRead`] for a client after a successful handshake.
    pub fn client(
        inner: RW,
        read_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebsocketsCore::client(inner, (), read_buffer, &mut [], fragments_buffer),
        }
    }

    /// Creates a new [`WebsocketsRead`] for a server after a successful handshake.
    pub fn server(
        inner: RW,
        read_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebsocketsCore::server(inner, (), read_buffer, &mut [], fragments_buffer),
        }
    }

    /// Returns reference to the reader.
    #[inline]
    pub const fn inner(&self) -> &RW {
        self.core.inner()
    }

    /// Returns mutable reference to the reader.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut RW {
        self.core.inner_mut()
    }

    /// Consumes the [`WebsocketsRead`] and returns the reader.
    #[inline]
    pub fn into_inner(self) -> RW {
        self.core.into_inner()
    }

    pub async fn maybe_next<'this>(
        &'this mut self,
    ) -> Option<Result<Option<Message<'this>>, ReadError<RW::Error>>>
    where
        RW: Read,
    {
        self.core.maybe_next().await
    }
}

#[derive(Debug)]
pub struct WebsocketsWrite<'buf, RW, Rng> {
    core: WebsocketsCore<'buf, RW, Rng>,
}

impl<'buf, RW, Rng> WebsocketsWrite<'buf, RW, Rng> {
    fn from_framed(framed: Framed<'buf, FramesCodec<Rng>, RW>) -> Self {
        Self {
            core: WebsocketsCore::from_framed(framed, &mut []),
        }
    }

    /// Creates a new [`WebsocketsWrite`] for a client after a successful handshake.
    pub fn client(inner: RW, rng: Rng, write_buffer: &'buf mut [u8]) -> Self {
        Self {
            core: WebsocketsCore::client(inner, rng, &mut [], write_buffer, &mut []),
        }
    }

    /// Creates a new [`WebsocketsWrite`] for a server after a successful handshake.
    pub fn server(inner: RW, rng: Rng, write_buffer: &'buf mut [u8]) -> Self {
        Self {
            core: WebsocketsCore::server(inner, rng, &mut [], write_buffer, &mut []),
        }
    }

    /// Returns reference to the writer.
    #[inline]
    pub const fn inner(&self) -> &RW {
        self.core.inner()
    }

    /// Returns mutable reference to the writer.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut RW {
        self.core.inner_mut()
    }

    /// Consumes the [`WebsocketsWrite`] and returns the writer.
    #[inline]
    pub fn into_inner(self) -> RW {
        self.core.into_inner()
    }

    pub async fn send(&mut self, message: Message<'_>) -> Result<(), WriteError<RW::Error>>
    where
        RW: Write,
        Rng: RngCore,
    {
        self.core.send(message).await
    }

    pub async fn send_fragmented(
        &mut self,
        message: Message<'_>,
        fragment_size: usize,
    ) -> Result<(), WriteError<RW::Error>>
    where
        RW: Write,
        Rng: RngCore,
    {
        self.core.send_fragmented(message, fragment_size).await
    }
}
