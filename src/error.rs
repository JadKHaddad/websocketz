use crate::CloseCode;

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("Reserved bits must be zero")]
    ReservedBitsNotZero,
    #[error("Invalid opcode")]
    InvalidOpCode,
    #[error("Control frame fragmented")]
    ControlFrameFragmented,
    #[error("Ping frame too large")]
    PingFrameTooLarge,
}

#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("Buffer too small")]
    BufferTooSmall,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError<I> {
    #[error("Read error: {0}")]
    Read(
        #[source]
        #[from]
        framez::ReadError<I, DecodeError>,
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
    #[error("Write error: {0}")]
    Write(
        #[source]
        #[from]
        framez::WriteError<I, EncodeError>,
    ),
}

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("Failed to generate websockets key: {0}")]
    KeyGeneration(base64::EncodeSliceError),
}

#[derive(Debug, thiserror::Error)]
pub enum Error<I> {
    #[error(transparent)]
    Read(#[from] ReadError<I>),
    #[error(transparent)]
    Write(#[from] WriteError<I>),
    #[error("Handshake error: {0}")]
    Handshake(#[from] HandshakeError),
}
