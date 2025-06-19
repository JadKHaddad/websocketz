use crate::CloseCode;

#[derive(Debug, thiserror::Error)]
pub enum FrameDecodeError {
    #[error("Reserved bits must be zero")]
    ReservedBitsNotZero,
    #[error("Invalid opcode")]
    InvalidOpCode,
    // The payload length comes as an u64, converting it to usize might fail on 32-bit systems
    #[error("Payload too large")]
    PayloadTooLarge,
    #[error("Control frame fragmented")]
    ControlFrameFragmented,
    #[error("Ping frame too large")]
    PingFrameTooLarge,
}

#[derive(Debug, thiserror::Error)]
pub enum FrameEncodeError {
    #[error("Buffer too small")]
    BufferTooSmall,
}

#[derive(Debug, thiserror::Error)]
pub enum HttpDecodeError {
    #[error("Parse error: {0}")]
    Parse(httparse::Error),
}

impl From<httparse::Error> for HttpDecodeError {
    fn from(err: httparse::Error) -> Self {
        HttpDecodeError::Parse(err)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HttpEncodeError {
    #[error("Buffer too small")]
    BufferTooSmall,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError<I> {
    #[error("Read frame error: {0}")]
    ReadFrame(
        #[source]
        #[from]
        framez::ReadError<I, FrameDecodeError>,
    ),
    #[error("Read http error: {0}")]
    ReadHttp(
        #[source]
        #[from]
        framez::ReadError<I, HttpDecodeError>,
    ),
    #[error("Invalid close frame")]
    InvalidCloseFrame,
    #[error("Invalid close code: {code:?}")]
    InvalidCloseCode { code: CloseCode },
    #[error("Invalid UTF-8")]
    InvalidUTF8,
    #[error("Invalid fragment")]
    InvalidFragment,
    #[error("Invalid continuation frame")]
    InvalidContinuationFrame,
    #[error("Fragments buffer too small to read a frame")]
    FragmentsBufferTooSmall,
}

#[derive(Debug, thiserror::Error)]
pub enum WriteError<I> {
    #[error("Write frame error: {0}")]
    WriteFrame(
        #[source]
        #[from]
        framez::WriteError<I, FrameEncodeError>,
    ),
    #[error("Write http error: {0}")]
    WriteHttp(
        #[source]
        #[from]
        framez::WriteError<I, HttpEncodeError>,
    ),
}

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("Connection closed during handshake")]
    ConnectionClosed,
    #[error("Missing or Invalid status code")]
    MissingOrInvalidStatusCode,
    #[error("Missing or invalid upgrade header")]
    MissingOrInvalidUpgrade,
    #[error("Missing or invalid connection header")]
    MissingOrInvalidConnection,
    #[error("Missing or invalid sec websocket accept header")]
    MissingOrInvalidAccept,
    #[error("Missing or invalid sec websocket version header")]
    MissingOrInvalidSecVersion,
    #[error("Missing sec websocket key header")]
    MissingSecKey,
}

#[derive(Debug, thiserror::Error)]
pub enum FragmentationError {
    #[error("Fragment size must be greater than 0")]
    InvalidFragmentSize,
    #[error("Only text and binary messages can be fragmented")]
    CanNotBeFragmented,
}

#[derive(Debug, thiserror::Error)]
pub enum Error<I> {
    #[error("Read error: {0}")]
    Read(
        #[from]
        #[source]
        ReadError<I>,
    ),
    #[error("Write error: {0}")]
    Write(
        #[from]
        #[source]
        WriteError<I>,
    ),
    #[error("Handshake error: {0}")]
    Handshake(
        #[from]
        #[source]
        HandshakeError,
    ),
    #[error("Fragment error: {0}")]
    Fragmentation(
        #[from]
        #[source]
        FragmentationError,
    ),
}
