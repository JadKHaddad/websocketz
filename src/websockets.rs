use embedded_io_async::{Read, Write};
use rand::RngCore;

use crate::{
    Message, Options, WebsocketsCore,
    error::{ReadError, WriteError},
};

#[derive(Debug)]
pub struct Websockets<'buf, RW, Rng> {
    core: WebsocketsCore<'buf, RW, Rng>,
}

impl<'buf, RW, Rng> Websockets<'buf, RW, Rng> {
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

    // TODO
    pub async fn handshake<const N: usize>(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        options: Options<'_, '_>,
    ) -> Result<RW, ()>
    where
        RW: Read + Write,
        Rng: RngCore,
    {
        WebsocketsCore::handshake::<N>(inner, rng, read_buffer, write_buffer, options).await
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
}

#[derive(Debug)]
pub struct WebsocketsRead<'buf, RW> {
    core: WebsocketsCore<'buf, RW, ()>,
}

impl<'buf, RW> WebsocketsRead<'buf, RW> {
    pub fn client(
        inner: RW,
        read_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            core: WebsocketsCore::client(inner, (), read_buffer, &mut [], fragments_buffer),
        }
    }

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
    pub fn client(inner: RW, rng: Rng, write_buffer: &'buf mut [u8]) -> Self {
        Self {
            core: WebsocketsCore::client(inner, rng, &mut [], write_buffer, &mut []),
        }
    }

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
