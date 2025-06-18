use embedded_io_async::{Read, Write};
use framez::{
    Framed,
    state::{ReadState, ReadWriteState, WriteState},
};
use httparse::Header;
use rand::RngCore;

use crate::{FramesCodec, Message, WebsocketsCore, error::Error};

#[derive(Debug)]
pub struct Websockets<'buf, RW, Rng> {
    core: WebsocketsCore<'buf, RW, Rng>,
}

impl<'buf, RW, Rng> Websockets<'buf, RW, Rng> {
    /// Creates a new [`Websockets`] client after a successful handshake.
    pub const fn client(
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

    /// Creates a new [`Websockets`] server after a successful handshake.
    pub const fn server(
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

    /// Creates a new [`Websockets`] client and performs the handshake.
    pub async fn connect<const N: usize>(
        path: &str,
        headers: &[Header<'_>],
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Result<Self, Error<RW::Error>>
    where
        RW: Read + Write,
        Rng: RngCore,
    {
        Self::client(inner, rng, read_buffer, write_buffer, fragments_buffer)
            .client_handshake::<N>(path, headers)
            .await
    }

    /// Creates a new [`Websockets`] server and performs the handshake.
    pub async fn accept<const N: usize>(
        headers: &[Header<'_>],
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Result<Self, Error<RW::Error>>
    where
        RW: Read + Write,
    {
        Self::server(inner, rng, read_buffer, write_buffer, fragments_buffer)
            .server_handshake::<N>(headers)
            .await
    }

    /// Returns reference to the reader/writer.
    #[inline]
    pub const fn inner(&self) -> &RW {
        self.core.inner()
    }

    /// Returns mutable reference to the reader/writer.
    #[inline]
    pub const fn inner_mut(&mut self) -> &mut RW {
        self.core.inner_mut()
    }

    /// Consumes the [`Websockets`] and returns the reader/writer.
    #[inline]
    pub fn into_inner(self) -> RW {
        self.core.into_inner()
    }

    /// Returns the number of bytes that can be framed.
    #[inline]
    pub fn framable(&self) -> usize {
        self.core.framable()
    }

    async fn client_handshake<const N: usize>(
        self,
        path: &str,
        headers: &[Header<'_>],
    ) -> Result<Self, Error<RW::Error>>
    where
        RW: Read + Write,
        Rng: RngCore,
    {
        Ok(Self {
            core: self.core.client_handshake::<N>(path, headers).await?,
        })
    }

    async fn server_handshake<const N: usize>(
        self,
        headers: &[Header<'_>],
    ) -> Result<Self, Error<RW::Error>>
    where
        RW: Read + Write,
    {
        Ok(Self {
            core: self.core.server_handshake::<N>(headers).await?,
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
    ) -> Option<Result<Option<Message<'this>>, Error<RW::Error>>>
    where
        RW: Read,
    {
        self.core.maybe_next().await
    }

    pub async fn send(&mut self, message: Message<'_>) -> Result<(), Error<RW::Error>>
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
    ) -> Result<(), Error<RW::Error>>
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

        let (read, write) = f(inner);

        let framed_read = Framed::from_parts(
            read_codec,
            read,
            ReadWriteState::new(state.read, WriteState::empty()),
        );

        let framed_write = Framed::from_parts(
            write_codec,
            write,
            ReadWriteState::new(ReadState::empty(), state.write),
        );

        (
            WebsocketsRead::new_from_framed(framed_read, self.core.fragments_buffer),
            WebsocketsWrite::new_from_framed(framed_write),
        )
    }
}

#[derive(Debug)]
pub struct WebsocketsRead<'buf, RW> {
    core: WebsocketsCore<'buf, RW, ()>,
}

impl<'buf, RW> WebsocketsRead<'buf, RW> {
    const fn new_from_framed(
        framed: Framed<'buf, FramesCodec<()>, RW>,
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebsocketsCore::new_from_framed(framed, fragments_buffer),
        }
    }

    /// Creates a new [`WebsocketsRead`] client after a successful handshake.
    pub const fn client(
        inner: RW,
        read_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebsocketsCore::client(inner, (), read_buffer, &mut [], fragments_buffer),
        }
    }

    /// Creates a new [`WebsocketsRead`] server after a successful handshake.
    pub const fn server(
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
    pub const fn inner_mut(&mut self) -> &mut RW {
        self.core.inner_mut()
    }

    /// Consumes the [`WebsocketsRead`] and returns the reader.
    #[inline]
    pub fn into_inner(self) -> RW {
        self.core.into_inner()
    }

    /// Returns the number of bytes that can be framed.
    #[inline]
    pub fn framable(&self) -> usize {
        self.core.framable()
    }

    pub async fn maybe_next<'this>(
        &'this mut self,
    ) -> Option<Result<Option<Message<'this>>, Error<RW::Error>>>
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
    const fn new_from_framed(framed: Framed<'buf, FramesCodec<Rng>, RW>) -> Self {
        Self {
            core: WebsocketsCore::new_from_framed(framed, &mut []),
        }
    }

    /// Creates a new [`WebsocketsWrite`] client after a successful handshake.
    pub const fn client(inner: RW, rng: Rng, write_buffer: &'buf mut [u8]) -> Self {
        Self {
            core: WebsocketsCore::client(inner, rng, &mut [], write_buffer, &mut []),
        }
    }

    /// Creates a new [`WebsocketsWrite`] server after a successful handshake.
    pub const fn server(inner: RW, rng: Rng, write_buffer: &'buf mut [u8]) -> Self {
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
    pub const fn inner_mut(&mut self) -> &mut RW {
        self.core.inner_mut()
    }

    /// Consumes the [`WebsocketsWrite`] and returns the writer.
    #[inline]
    pub fn into_inner(self) -> RW {
        self.core.into_inner()
    }

    pub async fn send(&mut self, message: Message<'_>) -> Result<(), Error<RW::Error>>
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
    ) -> Result<(), Error<RW::Error>>
    where
        RW: Write,
        Rng: RngCore,
    {
        self.core.send_fragmented(message, fragment_size).await
    }
}
