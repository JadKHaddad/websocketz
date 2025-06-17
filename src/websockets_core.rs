use base64::{Engine as _, engine::general_purpose};
use embedded_io_async::{Read, Write};
use framez::Framed;
use httparse::Header;
use rand::RngCore;

use sha1::{Digest, Sha1};

use crate::{
    CloseCode, CloseFrame, FramesCodec, Message, OpCode, Options, Request, RequestCodec,
    ResponseCodec,
    error::{Error, HandshakeError, ReadError, WriteError},
    next,
};

#[derive(Debug)]
pub struct Fragmented {
    opcode: OpCode,
    index: usize,
}

#[derive(Debug)]
pub struct WebsocketsCore<'buf, RW, Rng> {
    pub fragmented: Option<Fragmented>,
    pub fragments_buffer: &'buf mut [u8],
    pub framed: Framed<'buf, FramesCodec<Rng>, RW>,
}

impl<'buf, RW, Rng> WebsocketsCore<'buf, RW, Rng> {
    fn new(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        let framed = Framed::new(FramesCodec::new(rng), inner, read_buffer, write_buffer);

        Self::from_framed(framed, fragments_buffer)
    }

    pub fn from_framed(
        framed: Framed<'buf, FramesCodec<Rng>, RW>,
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self {
            fragmented: None,
            fragments_buffer,
            framed,
        }
    }

    pub fn client(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self::new(inner, rng, read_buffer, write_buffer, fragments_buffer)
            .with_mask(true)
            .with_unmask(false)
    }

    pub fn server(
        inner: RW,
        rng: Rng,
        read_buffer: &'buf mut [u8],
        write_buffer: &'buf mut [u8],
        fragments_buffer: &'buf mut [u8],
    ) -> Self {
        Self::new(inner, rng, read_buffer, write_buffer, fragments_buffer)
            .with_mask(false)
            .with_unmask(true)
    }

    fn with_mask(mut self, mask: bool) -> Self {
        self.framed.codec_mut().set_mask(mask);
        self
    }

    fn with_unmask(mut self, unmask: bool) -> Self {
        self.framed.codec_mut().set_unmask(unmask);
        self
    }

    /// Returns reference to the reader/writer.
    #[inline]
    pub const fn inner(&self) -> &RW {
        self.framed.inner()
    }

    /// Returns mutable reference to the reader/writer.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut RW {
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

    fn generate_sec_key(&mut self) -> Result<[u8; 24], HandshakeError>
    where
        Rng: RngCore,
    {
        let mut key: [u8; 16] = [0; 16];

        self.framed.codec_mut().rng_mut().fill_bytes(&mut key);

        let mut encoded: [u8; 24] = [0; 24];

        general_purpose::STANDARD
            .encode_slice(key, &mut encoded)
            .map_err(HandshakeError::SecKeyGeneration)?;

        Ok(encoded)
    }

    fn generate_sec_accept(sec_key: &[u8]) -> Result<[u8; 28], HandshakeError> {
        let mut sha1 = Sha1::new();

        sha1.update(sec_key);
        sha1.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");

        let hash = sha1.finalize();

        let mut encoded: [u8; 28] = [0; 28];

        general_purpose::STANDARD
            .encode_slice(hash, &mut encoded)
            .map_err(HandshakeError::SecAcceptGeneration)?;

        Ok(encoded)
    }

    pub async fn client_handshake<const N: usize>(
        mut self,
        options: Options<'_, '_>,
    ) -> Result<Self, Error<RW::Error>>
    where
        RW: Read + Write,
        Rng: RngCore,
    {
        let sec_key = self.generate_sec_key()?;

        let headers = &[
            Header {
                name: "Upgrade",
                value: b"websocket",
            },
            Header {
                name: "Connection",
                value: b"Upgrade",
            },
            Header {
                name: "Sec-WebSocket-Version",
                value: b"13",
            },
            Header {
                name: "Sec-WebSocket-Key",
                value: &sec_key,
            },
        ];

        let request = Request::get(options.path, headers, options.headers);

        let (codec, inner, state) = self.framed.into_parts();

        let mut framed = Framed::from_parts(RequestCodec::new(), inner, state.reset());

        framed
            .send(request)
            .await
            .map_err(|err| Error::Write(WriteError::WriteHttp(err)))?;

        let (_, inner, state) = framed.into_parts();

        let mut framed = Framed::from_parts(ResponseCodec::<N>::new(), inner, state.reset());

        match next!(framed) {
            None => {
                return Err(Error::Handshake(HandshakeError::ConnectionClosed));
            }
            Some(Err(err)) => {
                return Err(Error::Read(ReadError::ReadHttp(err)));
            }
            Some(Ok(response)) => {
                if !matches!(response.code(), Some(101)) {
                    return Err(Error::Handshake(HandshakeError::InvalidStatusCode {
                        code: response.code(),
                    }));
                }

                if !response
                    .header_value_str("upgrade")
                    .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
                {
                    return Err(Error::Handshake(HandshakeError::InvalidUpgradeHeader));
                }

                if !response
                    .header_value_str("connection")
                    .is_some_and(|v| v.eq_ignore_ascii_case("upgrade"))
                {
                    return Err(Error::Handshake(HandshakeError::InvalidConnectionHeader));
                }

                let sec_accept = response
                    .header_value("sec-websocket-accept")
                    .ok_or_else(|| Error::Handshake(HandshakeError::MissingAcceptHeader))?;

                let expected_sec_accept =
                    Self::generate_sec_accept(&sec_key).map_err(Error::Handshake)?;

                if sec_accept != expected_sec_accept {
                    return Err(Error::Handshake(HandshakeError::InvalidAcceptHeader));
                }
            }
        }

        let (_, inner, state) = framed.into_parts();

        let framed = Framed::from_parts(codec, inner, state);

        Ok(Self {
            fragmented: None,
            fragments_buffer: self.fragments_buffer,
            framed,
        })
    }

    pub async fn server_handshake<const N: usize>(
        mut self,
        options: Options<'_, '_>,
    ) -> Result<Self, Error<RW::Error>>
    where
        RW: Read + Write,
        Rng: RngCore,
    {
        todo!()
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

        match frame.opcode() {
            OpCode::Text | OpCode::Binary => {
                if frame.is_final() {
                    if self.fragmented.is_some() {
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
                        _ => unreachable!(),
                    }
                }

                if frame.payload().len() > self.fragments_buffer.len() {
                    return Some(Err(Error::Read(ReadError::FragmentsBufferTooSmall)));
                }

                self.fragments_buffer[..frame.payload().len()].copy_from_slice(frame.payload());

                self.fragmented = Some(Fragmented {
                    opcode: frame.opcode(),
                    index: frame.payload().len(),
                });
            }
            OpCode::Continuation => {
                let message = match self.fragmented.as_mut() {
                    None => {
                        return Some(Err(Error::Read(ReadError::InvalidContinuationFrame)));
                    }
                    Some(fragmented) => {
                        if fragmented.index + frame.payload().len() > self.fragments_buffer.len() {
                            return Some(Err(Error::Read(ReadError::FragmentsBufferTooSmall)));
                        }

                        self.fragments_buffer[fragmented.index..][..frame.payload().len()]
                            .copy_from_slice(frame.payload());

                        fragmented.index += frame.payload().len();

                        if frame.is_final() {
                            match fragmented.opcode {
                                OpCode::Text => {
                                    match core::str::from_utf8(
                                        &self.fragments_buffer[..fragmented.index],
                                    ) {
                                        Ok(text) => Some(Message::Text(text)),
                                        Err(_) => {
                                            return Some(Err(Error::Read(ReadError::InvalidUTF8)));
                                        }
                                    }
                                }
                                OpCode::Binary => Some(Message::Binary(
                                    &self.fragments_buffer[..fragmented.index],
                                )),
                                _ => unreachable!(),
                            }
                        } else {
                            None
                        }
                    }
                };

                if let Some(message) = message {
                    self.fragmented = None;

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
                        let code = CloseCode::from(u16::from_be_bytes([payload[0], payload[1]]));

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
        for frame in message.fragments(fragment_size) {
            self.framed
                .send(frame)
                .await
                .map_err(|err| Error::Write(WriteError::WriteFrame(err)))?;
        }

        Ok(())
    }
}
