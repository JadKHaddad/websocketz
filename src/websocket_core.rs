use base64::{Engine as _, engine::general_purpose};
use embedded_io_async::{Read, Write};
use framez::{Echo, Framed, ReadWriteError};
use httparse::Header;
use rand::RngCore;

use sha1::{Digest, Sha1};

use crate::{
    CloseCode, CloseFrame, FramesCodec, Message, OpCode,
    error::{Error, HandshakeError, ReadError, WriteError},
    frame::Frame,
    http::{
        HeaderExt, InRequestCodec, InResponseCodec, OutRequest, OutRequestCodec, OutResponse,
        OutResponseCodec, Request, Response,
    },
    next,
    options::{AcceptOptions, ConnectOptions},
};

#[derive(Debug)]
pub struct Fragmented {
    opcode: OpCode,
    index: usize,
}

#[derive(Debug)]
pub struct WebSocketCore<'buf, RW, Rng> {
    pub fragmented: Option<Fragmented>,
    pub fragments_buffer: &'buf mut [u8],
    pub framed: Framed<'buf, FramesCodec<Rng>, RW>,
    auto_pong: bool,
    auto_close: bool,
}

impl<'buf, RW, Rng> WebSocketCore<'buf, RW, Rng> {
    #[inline]
    const fn from_framed(
        framed: Framed<'buf, FramesCodec<Rng>, RW>,
        fragmented: Option<Fragmented>,
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            fragmented,
            fragments_buffer,
            framed,
            auto_pong: true,
            auto_close: true,
        }
    }

    #[inline]
    pub const fn new_from_framed(
        framed: Framed<'buf, FramesCodec<Rng>, RW>,
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self::from_framed(framed, None, fragments_buffer)
    }

    #[inline]
    const fn new(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self::new_from_framed(
            Framed::new(FramesCodec::new(rng), inner, read_buffer, write_buffer),
            fragments_buffer,
        )
    }

    #[inline]
    pub const fn client(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self::new(inner, rng, read_buffer, write_buffer, fragments_buffer).into_server()
    }

    #[inline]
    pub const fn server(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self::new(inner, rng, read_buffer, write_buffer, fragments_buffer).into_client()
    }

    #[inline]
    const fn into_client(mut self) -> Self {
        self.framed.codec_mut().set_mask(false);
        self.framed.codec_mut().set_unmask(true);
        self
    }

    #[inline]
    const fn into_server(mut self) -> Self {
        self.framed.codec_mut().set_mask(true);
        self.framed.codec_mut().set_unmask(false);
        self
    }

    #[inline]
    pub const fn set_auto_pong(&mut self, auto_pong: bool) {
        self.auto_pong = auto_pong;
    }

    #[inline]
    pub const fn set_auto_close(&mut self, auto_close: bool) {
        self.auto_close = auto_close;
    }

    /// Returns reference to the reader/writer.
    #[inline]
    pub const fn inner(&self) -> &RW {
        self.framed.inner()
    }

    /// Returns mutable reference to the reader/writer.
    #[inline]
    pub const fn inner_mut(&mut self) -> &mut RW {
        self.framed.inner_mut()
    }

    /// Consumes the [`WebsocketsCore`] and returns the reader/writer.
    #[inline]
    pub fn into_inner(self) -> RW {
        self.framed.into_parts().1
    }

    /// Returns the number of bytes that can be framed.
    #[inline]
    pub fn framable(&self) -> usize {
        self.framed.framable()
    }

    fn generate_sec_key(&mut self) -> [u8; 24]
    where
        Rng: RngCore,
    {
        let mut key: [u8; 16] = [0; 16];

        debug_assert!(key.len() == 16, "Key should be 16 bytes long");

        self.framed.codec_mut().rng_mut().fill_bytes(&mut key);

        // 24 = ((4 * key.len() + 2) / 3 + 3) & !3 = ((4 * 16 + 2) / 3 + 3) & !3
        let mut encoded: [u8; 24] = [0; 24];

        general_purpose::STANDARD
            .encode_slice(key, &mut encoded)
            .expect("Bug: sec_key encoding failed");

        encoded
    }

    fn generate_sec_accept(sec_key: &[u8]) -> [u8; 28] {
        let mut sha1 = Sha1::new();

        sha1.update(sec_key);
        sha1.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");

        let hash = sha1.finalize();

        debug_assert!(hash.len() == 20, "SHA1 hash should be 20 bytes long");

        // 28 = ((4 * hash.len() + 2) / 3 + 3) & !3 = ((4 * 20 + 2) / 3 + 3) & !3
        let mut encoded: [u8; 28] = [0; 28];

        general_purpose::STANDARD
            .encode_slice(hash, &mut encoded)
            .expect("Bug: sec_accept encoding failed");

        encoded
    }

    pub async fn client_handshake<const N: usize, F, T, E>(
        mut self,
        options: ConnectOptions<'_, '_>,
        on_response: F,
    ) -> Result<(Self, T), Error<RW::Error, E>>
    where
        F: for<'a> Fn(&Response<'a, N>) -> Result<T, E>,
        RW: Read + Write,
        Rng: RngCore,
    {
        let sec_key = self.generate_sec_key();

        let headers = &[
            Header {
                name: "upgrade",
                value: b"websocket",
            },
            Header {
                name: "connection",
                value: b"upgrade",
            },
            Header {
                name: "sec-websocket-version",
                value: b"13",
            },
            Header {
                name: "sec-websocket-key",
                value: &sec_key,
            },
        ];

        let request = OutRequest::get_unchecked(options.path, headers, options.headers);

        let (codec, inner, state) = self.framed.into_parts();

        let mut framed = Framed::from_parts(OutRequestCodec::new(), inner, state.reset());

        framed
            .send(request)
            .await
            .map_err(|err| Error::Write(WriteError::WriteHttp(err)))?;

        let (_, inner, state) = framed.into_parts();

        let mut framed = Framed::from_parts(InResponseCodec::<N>::new(), inner, state.reset());

        let custom = match next!(framed) {
            None => {
                return Err(Error::Handshake(HandshakeError::ConnectionClosed));
            }
            Some(Err(err)) => {
                return Err(Error::Read(ReadError::ReadHttp(err)));
            }
            Some(Ok(response)) => {
                let custom = on_response(&response).map_err(HandshakeError::Other)?;

                if !matches!(response.code(), 101) {
                    return Err(Error::Handshake(HandshakeError::InvalidStatusCode));
                }

                if !response
                    .headers()
                    .header_value_str("upgrade")
                    .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
                {
                    return Err(Error::Handshake(HandshakeError::MissingOrInvalidUpgrade));
                }

                if !response
                    .headers()
                    .header_value_str("connection")
                    .is_some_and(|v| v.eq_ignore_ascii_case("upgrade"))
                {
                    return Err(Error::Handshake(HandshakeError::MissingOrInvalidConnection));
                }

                let sec_accept = Self::generate_sec_accept(&sec_key);

                if response
                    .headers()
                    .header_value("sec-websocket-accept")
                    .is_none_or(|v| v != sec_accept)
                {
                    return Err(Error::Handshake(HandshakeError::MissingOrInvalidAccept));
                }

                custom
            }
        };

        let (_, inner, state) = framed.into_parts();

        let framed = Framed::from_parts(codec, inner, state);

        Ok((
            Self::from_framed(framed, self.fragmented, self.fragments_buffer),
            custom,
        ))
    }

    pub async fn server_handshake<const N: usize, F, T, E>(
        self,
        options: AcceptOptions<'_, '_>,
        on_request: F,
    ) -> Result<(Self, T), Error<RW::Error, E>>
    where
        F: for<'a> Fn(&Request<'a, N>) -> Result<T, E>,
        RW: Read + Write,
    {
        let (codec, inner, state) = self.framed.into_parts();

        let mut framed = Framed::from_parts(InRequestCodec::<N>::new(), inner, state);

        let (accept_key, custom) = match next!(framed) {
            None => {
                return Err(Error::Handshake(HandshakeError::ConnectionClosed));
            }
            Some(Err(err)) => {
                return Err(Error::Read(ReadError::ReadHttp(err)));
            }
            Some(Ok(request)) => {
                let custom = on_request(&request).map_err(HandshakeError::Other)?;

                if !matches!(request.method(), "GET") {
                    return Err(Error::Handshake(HandshakeError::WrongHttpMethod));
                }

                // http version must be 1.1 or higher
                if request.version() < 1 {
                    return Err(Error::Handshake(HandshakeError::WrongHttpVersion));
                }

                if !request
                    .headers()
                    .header_value_str("sec-websocket-version")
                    .is_some_and(|v| v.eq_ignore_ascii_case("13"))
                {
                    return Err(Error::Handshake(HandshakeError::MissingOrInvalidSecVersion));
                }

                let sec_key = request
                    .headers()
                    .header_value("sec-websocket-key")
                    .ok_or(Error::Handshake(HandshakeError::MissingSecKey))?;

                (Self::generate_sec_accept(sec_key), custom)
            }
        };

        let headers = &[
            Header {
                name: "upgrade",
                value: b"websocket",
            },
            Header {
                name: "connection",
                value: b"upgrade",
            },
            Header {
                name: "sec-websocket-version",
                value: b"13",
            },
            Header {
                name: "sec-websocket-accept",
                value: &accept_key,
            },
        ];

        let response = OutResponse::switching_protocols(headers, options.headers);

        let (_, inner, state) = framed.into_parts();

        let mut framed = Framed::from_parts(OutResponseCodec::new(), inner, state);

        framed
            .send(response)
            .await
            .map_err(|err| Error::Write(WriteError::WriteHttp(err)))?;

        let (_, inner, state) = framed.into_parts();

        let framed = Framed::from_parts(codec, inner, state);

        Ok((
            Self::from_framed(framed, self.fragmented, self.fragments_buffer),
            custom,
        ))
    }

    pub async fn maybe_next<'this>(
        &'this mut self,
    ) -> Option<Result<Option<Message<'this>>, Error<RW::Error>>>
    where
        RW: Read,
    {
        let frame = match self.framed.maybe_next().await? {
            Ok(Some(frame)) => frame,
            Ok(None) => return Some(Ok(None)),
            Err(err) => return Some(Err(Error::Read(ReadError::ReadFrame(err)))),
        };

        Self::on_frame(&mut self.fragmented, self.fragments_buffer, frame)
    }

    pub async fn maybe_next_echoed<'this>(
        &'this mut self,
    ) -> Option<Result<Option<Message<'this>>, Error<RW::Error>>>
    where
        RW: Read + Write,
        Rng: RngCore,
    {
        let frame = match self
            .framed
            .maybe_next_echoed(|frame| {
                if self.auto_pong && frame.opcode() == OpCode::Ping {
                    return Echo::Echo(Frame::new_final(OpCode::Pong, frame.payload()));
                };

                if self.auto_close && frame.opcode() == OpCode::Close {
                    const CLOSE_CODE: &[u8] = &CloseCode::Normal.into_u16().to_be_bytes();

                    return Echo::Echo(Frame::new_final(OpCode::Close, CLOSE_CODE));
                }

                Echo::NoEcho(frame)
            })
            .await?
        {
            Ok(Some(frame)) => frame,
            Ok(None) => return Some(Ok(None)),
            Err(err) => match err {
                ReadWriteError::Read(err) => {
                    return Some(Err(Error::Read(ReadError::ReadFrame(err))));
                }
                ReadWriteError::Write(err) => {
                    return Some(Err(Error::Write(WriteError::WriteFrame(err))));
                }
            },
        };

        Self::on_frame(&mut self.fragmented, self.fragments_buffer, frame)
    }

    fn on_frame<'this>(
        fragmented: &mut Option<Fragmented>,
        fragments_buffer: &'this mut [u8],
        frame: Frame<'this>,
    ) -> Option<Result<Option<Message<'this>>, Error<RW::Error>>>
    where
        RW: Read,
    {
        match frame.opcode() {
            OpCode::Text | OpCode::Binary => {
                if frame.is_final() {
                    if fragmented.is_some() {
                        return Some(Err(Error::Read(ReadError::InvalidFragment)));
                    }

                    match frame.opcode() {
                        OpCode::Binary => {
                            return Some(Ok(Some(Message::Binary(frame.payload()))));
                        }
                        OpCode::Text => match core::str::from_utf8(frame.payload()) {
                            Ok(text) => {
                                return Some(Ok(Some(Message::Text(text))));
                            }
                            Err(_) => {
                                return Some(Err(Error::Read(ReadError::InvalidUTF8)));
                            }
                        },
                        _ => unreachable!("Already matched for OpCode::Text | OpCode::Binary"),
                    }
                }

                if frame.payload().len() > fragments_buffer.len() {
                    return Some(Err(Error::Read(ReadError::FragmentsBufferTooSmall)));
                }

                fragments_buffer[..frame.payload().len()].copy_from_slice(frame.payload());

                *fragmented = Some(Fragmented {
                    opcode: frame.opcode(),
                    index: frame.payload().len(),
                });
            }
            OpCode::Continuation => {
                let message = match fragmented.as_mut() {
                    None => {
                        return Some(Err(Error::Read(ReadError::InvalidContinuationFrame)));
                    }
                    Some(fragmented) => {
                        if fragmented.index + frame.payload().len() > fragments_buffer.len() {
                            return Some(Err(Error::Read(ReadError::FragmentsBufferTooSmall)));
                        }

                        fragments_buffer[fragmented.index..][..frame.payload().len()]
                            .copy_from_slice(frame.payload());

                        fragmented.index += frame.payload().len();

                        if frame.is_final() {
                            match fragmented.opcode {
                                OpCode::Text => {
                                    match core::str::from_utf8(
                                        &fragments_buffer[..fragmented.index],
                                    ) {
                                        Ok(text) => Some(Message::Text(text)),
                                        Err(_) => {
                                            return Some(Err(Error::Read(ReadError::InvalidUTF8)));
                                        }
                                    }
                                }
                                OpCode::Binary => {
                                    Some(Message::Binary(&fragments_buffer[..fragmented.index]))
                                }
                                _ => unreachable!(
                                    "Opcode can only be set to OpCode::Text | OpCode::Binary in the first match branch"
                                ),
                            }
                        } else {
                            None
                        }
                    }
                };

                if let Some(message) = message {
                    *fragmented = None;

                    return Some(Ok(Some(message)));
                }
            }
            OpCode::Close => {
                let payload = frame.payload();

                match payload.len() {
                    0 => {}
                    1 => {
                        return Some(Err(Error::Read(ReadError::InvalidCloseFrame)));
                    }
                    _ => {
                        let code =
                            CloseCode::from_u16(u16::from_be_bytes([payload[0], payload[1]]));

                        if !code.is_allowed() {
                            return Some(Err(Error::Read(ReadError::InvalidCloseCode { code })));
                        }

                        match core::str::from_utf8(&payload[2..]) {
                            Ok(reason) => {
                                let close_frame = CloseFrame::new(code, reason);

                                return Some(Ok(Some(Message::Close(Some(close_frame)))));
                            }
                            Err(_) => {
                                return Some(Err(Error::Read(ReadError::InvalidUTF8)));
                            }
                        }
                    }
                }

                return Some(Ok(Some(Message::Close(None))));
            }
            OpCode::Ping => {
                return Some(Ok(Some(Message::Ping(frame.payload()))));
            }
            OpCode::Pong => {
                return Some(Ok(Some(Message::Pong(frame.payload()))));
            }
        }

        Some(Ok(None))
    }

    pub async fn send(&mut self, message: Message<'_>) -> Result<(), Error<RW::Error>>
    where
        RW: Write,
        Rng: RngCore,
    {
        self.framed
            .send(message)
            .await
            .map_err(|err| Error::Write(WriteError::WriteFrame(err)))?;

        Ok(())
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
        for frame in message
            .fragments(fragment_size)
            .map_err(Error::Fragmentation)?
        {
            self.framed
                .send(frame)
                .await
                .map_err(|err| Error::Write(WriteError::WriteFrame(err)))?;
        }

        Ok(())
    }
}
