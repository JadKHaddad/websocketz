use embedded_io_async::{Read, Write};
use framez::{
    Framed,
    state::{ReadState, ReadWriteState, WriteState},
};
use rand::RngCore;

use crate::{
    FramesCodec, Message, WebSocketCore,
    error::Error,
    http::{Request, Response},
    options::{AcceptOptions, ConnectOptions},
};

#[derive(Debug)]
pub struct WebSocket<'buf, RW, Rng> {
    core: WebSocketCore<'buf, RW, Rng>,
}

impl<'buf, RW, Rng> WebSocket<'buf, RW, Rng> {
    /// Creates a new [`WebSocket`] client after a successful handshake.
    pub const fn client(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebSocketCore::client(inner, rng, read_buffer, write_buffer, fragments_buffer),
        }
    }

    /// Creates a new [`WebSocket`] server after a successful handshake.
    pub const fn server(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebSocketCore::server(inner, rng, read_buffer, write_buffer, fragments_buffer),
        }
    }

    /// Creates a new [`WebSocket`] client and performs the handshake.
    pub async fn connect<const N: usize>(
        options: ConnectOptions<'_, '_>,
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
        Ok(Self::connect_with::<N, _, _, _>(
            options,
            inner,
            rng,
            read_buffer,
            write_buffer,
            fragments_buffer,
            |_| Ok(()),
        )
        .await?
        .0)
    }

    pub async fn connect_with<const N: usize, F, T, E>(
        options: ConnectOptions<'_, '_>,
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
        on_response: F,
    ) -> Result<(Self, T), Error<RW::Error, E>>
    where
        F: for<'a> Fn(&Response<'a, N>) -> Result<T, E>,
        RW: Read + Write,
        Rng: RngCore,
    {
        Self::client(inner, rng, read_buffer, write_buffer, fragments_buffer)
            .client_handshake::<N, _, _, _>(options, on_response)
            .await
    }

    /// Creates a new [`WebSocket`] server and performs the handshake.
    pub async fn accept<const N: usize>(
        options: AcceptOptions<'_, '_>,
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Result<Self, Error<RW::Error>>
    where
        RW: Read + Write,
    {
        Ok(Self::accept_with::<N, _, _, _>(
            options,
            inner,
            rng,
            read_buffer,
            write_buffer,
            fragments_buffer,
            |_| Ok(()),
        )
        .await?
        .0)
    }

    pub async fn accept_with<const N: usize, F, T, E>(
        options: AcceptOptions<'_, '_>,
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
        on_request: F,
    ) -> Result<(Self, T), Error<RW::Error, E>>
    where
        F: for<'a> Fn(&Request<'a, N>) -> Result<T, E>,
        RW: Read + Write,
    {
        Self::server(inner, rng, read_buffer, write_buffer, fragments_buffer)
            .server_handshake::<N, _, _, _>(options, on_request)
            .await
    }

    #[inline]
    pub const fn with_auto_pong(mut self, auto_pong: bool) -> Self {
        self.core.set_auto_pong(auto_pong);
        self
    }

    #[inline]
    pub const fn with_auto_close(mut self, auto_close: bool) -> Self {
        self.core.set_auto_close(auto_close);
        self
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

    /// Consumes the [`WebSocket`] and returns the reader/writer.
    #[inline]
    pub fn into_inner(self) -> RW {
        self.core.into_inner()
    }

    /// Returns the number of bytes that can be framed.
    #[inline]
    pub fn framable(&self) -> usize {
        self.core.framable()
    }

    async fn client_handshake<const N: usize, F, T, E>(
        self,
        options: ConnectOptions<'_, '_>,
        on_response: F,
    ) -> Result<(Self, T), Error<RW::Error, E>>
    where
        F: for<'a> Fn(&Response<'a, N>) -> Result<T, E>,
        RW: Read + Write,
        Rng: RngCore,
    {
        let (core, custom) = self
            .core
            .client_handshake::<N, _, _, _>(options, on_response)
            .await?;

        Ok((Self { core }, custom))
    }

    async fn server_handshake<const N: usize, F, T, E>(
        self,
        options: AcceptOptions<'_, '_>,
        on_request: F,
    ) -> Result<(Self, T), Error<RW::Error, E>>
    where
        F: for<'a> Fn(&Request<'a, N>) -> Result<T, E>,
        RW: Read + Write,
    {
        let (core, custom) = self
            .core
            .server_handshake::<N, _, _, _>(options, on_request)
            .await?;

        Ok((Self { core }, custom))
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
    /// use websocketz::{WebSocket, mock::Noop, next};
    ///
    /// async fn read() -> Result<(), Box<dyn Error>> {
    ///     let stream = Noop;
    ///     let rng = Noop;
    ///     let r_buf = &mut [0u8; 1024];
    ///     let w_buf = &mut [0u8; 1024];
    ///     let f_buf = &mut [0u8; 1024];
    ///     
    ///     let mut websocketz = WebSocket::client(stream, rng, r_buf, w_buf, f_buf);
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
        RW: Read + Write,
        Rng: RngCore,
    {
        self.core.maybe_next_echoed().await
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

    /// Splits the [`WebSocket`] into a [`WebSocketRead`] and a [`WebSocketWrite`].
    ///
    /// # Note
    ///
    /// `auto_pong` and `auto_close` will `NOT` be applied to the split instances.
    pub fn split_with<F, R, W>(self, f: F) -> (WebSocketRead<'buf, R>, WebSocketWrite<'buf, W, Rng>)
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
            WebSocketRead::new_from_framed(framed_read, self.core.fragments_buffer),
            WebSocketWrite::new_from_framed(framed_write),
        )
    }
}

#[derive(Debug)]
pub struct WebSocketRead<'buf, RW> {
    core: WebSocketCore<'buf, RW, ()>,
}

impl<'buf, RW> WebSocketRead<'buf, RW> {
    const fn new_from_framed(
        framed: Framed<'buf, FramesCodec<()>, RW>,
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebSocketCore::new_from_framed(framed, fragments_buffer),
        }
    }

    /// Creates a new [`WebSocketRead`] client after a successful handshake.
    pub const fn client(
        inner: RW,
        read_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebSocketCore::client(inner, (), read_buffer, &mut [], fragments_buffer),
        }
    }

    /// Creates a new [`WebSocketRead`] server after a successful handshake.
    pub const fn server(
        inner: RW,
        read_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebSocketCore::server(inner, (), read_buffer, &mut [], fragments_buffer),
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

    /// Consumes the [`WebSocketRead`] and returns the reader.
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
pub struct WebSocketWrite<'buf, RW, Rng> {
    core: WebSocketCore<'buf, RW, Rng>,
}

impl<'buf, RW, Rng> WebSocketWrite<'buf, RW, Rng> {
    const fn new_from_framed(framed: Framed<'buf, FramesCodec<Rng>, RW>) -> Self {
        Self {
            core: WebSocketCore::new_from_framed(framed, &mut []),
        }
    }

    /// Creates a new [`WebSocketWrite`] client after a successful handshake.
    pub const fn client(inner: RW, rng: Rng, write_buffer: &'buf mut [u8]) -> Self {
        Self {
            core: WebSocketCore::client(inner, rng, &mut [], write_buffer, &mut []),
        }
    }

    /// Creates a new [`WebSocketWrite`] server after a successful handshake.
    pub const fn server(inner: RW, rng: Rng, write_buffer: &'buf mut [u8]) -> Self {
        Self {
            core: WebSocketCore::server(inner, rng, &mut [], write_buffer, &mut []),
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

    /// Consumes the [`WebSocketWrite`] and returns the writer.
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
