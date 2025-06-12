use embedded_io_async::{Read, Write};
use rand::RngCore;

use crate::{
    Message,
    error::{ReadError, WriteError},
    websockets_core::WebsocketsCore,
};

#[derive(Debug)]
pub struct Websockets<'buf, RW, Rng> {
    core: WebsocketsCore<'buf, RW, Rng>,
    auto_pong: bool,
    auto_close: bool,
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
            auto_pong: true,
            auto_close: true,
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
            auto_pong: true,
            auto_close: true,
        }
    }

    pub fn with_auto_pong(mut self, auto_pong: bool) -> Self {
        self.auto_pong = auto_pong;
        self
    }

    pub fn with_auto_close(mut self, auto_close: bool) -> Self {
        self.auto_close = auto_close;
        self
    }

    // TODO: implement auto-pong and auto-close logic
    // That is why we have the Write and RngCore trait bounds here.
    pub async fn maybe_next<'this>(
        &'this mut self,
    ) -> Option<Result<Option<Message<'this>>, ReadError<RW::Error>>>
    where
        RW: Read + Write,
        Rng: RngCore,
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
