//! Crate's error module.
//!
//! Contains all error types used throughout the crate.

use core::convert::Infallible;

/// Error decoding a WebSocket frame.
#[derive(Debug, thiserror::Error)]
pub enum FrameDecodeError {
    /// Reserved bits are not zero.
    #[error("Reserved bits must be zero")]
    ReservedBitsNotZero,
    /// Unmasked frame received from client.
    ///
    /// The server must close the connection when an unmasked frame is received.
    #[error("Received an unmasked frame from client")]
    UnmaskedFrameFromClient,
    /// Masked frame received from server.
    ///
    /// The client must close the connection when a masked frame is received.
    #[error("Received a masked frame from server")]
    MaskedFrameFromServer,
    /// Invalid opcode.
    #[error("Invalid opcode")]
    InvalidOpCode,
    /// Payload length is too large.
    // XXX: The payload length comes as a u64, converting it to usize might fail on 32-bit systems
    #[error("Payload too large")]
    PayloadTooLarge,
    /// Control frame fragmented.
    ///
    /// Control frames must not be fragmented.
    #[error("Control frame fragmented")]
    ControlFrameFragmented,
    /// Control frame too large.
    ///
    /// Control frames must have a payload length of 125 bytes or less.
    #[error("Control frame too large")]
    ControlFrameTooLarge,
}

/// Error encoding a WebSocket frame.
#[derive(Debug, thiserror::Error)]
pub enum FrameEncodeError {
    /// Write buffer is too small to hold the encoded frame.
    #[error("Buffer too small")]
    BufferTooSmall,
}

/// Error decoding an HTTP request/response.
#[derive(Debug, thiserror::Error)]
pub enum HttpDecodeError {
    /// Error parsing the HTTP request/response.
    #[error("Parse error: {0}")]
    Parse(httparse::Error),
}

impl From<httparse::Error> for HttpDecodeError {
    fn from(err: httparse::Error) -> Self {
        HttpDecodeError::Parse(err)
    }
}

/// Error encoding an HTTP request/response.
#[derive(Debug, thiserror::Error)]
pub enum HttpEncodeError {
    /// Write buffer is too small to hold the encoded HTTP request/response.
    #[error("Buffer too small")]
    BufferTooSmall,
}

/// Protocol specific errors/violations.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    /// Close frame is invalid.
    #[error("Invalid close frame")]
    InvalidCloseFrame,
    /// Close code is invalid.
    #[error("Invalid close code")]
    InvalidCloseCode,
    /// Text message contains invalid UTF-8.
    #[error("Invalid UTF-8")]
    InvalidUTF8,
    /// Fragment is invalid.
    ///
    /// This happens when a final fragment is received without any prior fragments.
    #[error("Invalid fragment")]
    InvalidFragment,
    /// Continuation frame is invalid.
    ///
    /// This happens when a continuation frame is received without an ongoing fragmented message.
    #[error("Invalid continuation frame")]
    InvalidContinuationFrame,
}

/// Error reading from a WebSocket connection.
#[derive(Debug, thiserror::Error)]
pub enum ReadError<I> {
    /// Error reading a WebSocket frame from the underlying I/O.
    #[error("Read frame error: {0}")]
    ReadFrame(
        #[source]
        #[from]
        framez::ReadError<I, FrameDecodeError>,
    ),
    /// Error reading an HTTP request/response from the underlying I/O.
    #[error("Read http error: {0}")]
    ReadHttp(
        #[source]
        #[from]
        framez::ReadError<I, HttpDecodeError>,
    ),
    /// Protocol error.
    #[error("Protocol error: {0}")]
    Protocol(
        #[source]
        #[from]
        ProtocolError,
    ),
    /// Fragments buffer is too small to read a frame.
    #[error("Fragments buffer too small to read a frame")]
    FragmentsBufferTooSmall,
}

/// Error writing to a WebSocket connection.
#[derive(Debug, thiserror::Error)]
pub enum WriteError<I> {
    /// Websocket connection is closed.
    ///
    /// To close the TCP connection, you should drop/close the underlying I/O instance.
    #[error("Connection closed")]
    ConnectionClosed,
    /// Error writing a WebSocket frame to the underlying I/O.
    #[error("Write frame error: {0}")]
    WriteFrame(
        #[source]
        #[from]
        framez::WriteError<I, FrameEncodeError>,
    ),
    /// Error writing an HTTP request/response to the underlying I/O.
    #[error("Write http error: {0}")]
    WriteHttp(
        #[source]
        #[from]
        framez::WriteError<I, HttpEncodeError>,
    ),
}

/// Error establishing a WebSocket handshake.
///
/// # Generic Parameter
///
/// `E`: User-defined error type for custom errors during the handshake.
#[derive(Debug, thiserror::Error)]
pub enum HandshakeError<E = Infallible> {
    /// Use of the wrong HTTP method (the WebSocket protocol requires the GET method to be used).
    #[error("Unsupported HTTP method used - only GET is allowed")]
    WrongHttpMethod,
    /// Wrong HTTP version used (the WebSocket protocol requires version 1.1 or higher).
    #[error("HTTP version must be 1.1 or higher")]
    WrongHttpVersion,
    /// Connection was closed during the handshake.
    #[error("Connection closed during handshake")]
    ConnectionClosed,
    /// Invalid status code. (Should be 101 for switching protocols.)
    #[error("Invalid status code")]
    InvalidStatusCode,
    /// Missing or invalid (`Upgrade`: `websocket`) header.
    #[error("Missing or invalid upgrade header")]
    MissingOrInvalidUpgrade,
    /// Missing or invalid (`Connection`: `upgrade`) header.
    #[error("Missing or invalid connection header")]
    MissingOrInvalidConnection,
    /// Missing or invalid (`Sec-WebSocket-Accept`) header.
    #[error("Missing or invalid sec websocket accept header")]
    MissingOrInvalidAccept,
    /// Missing or invalid (`Sec-WebSocket-Version`) header.
    #[error("Missing or invalid sec websocket version header")]
    MissingOrInvalidSecVersion,
    /// Missing (`Sec-WebSocket-Key`) header.
    #[error("Missing sec websocket key header")]
    MissingSecKey,
    /// Other error.
    ///
    /// User-defined error type.
    #[error("Other: {0}")]
    Other(E),
}

/// Fragmentation error.
#[derive(Debug, thiserror::Error)]
pub enum FragmentationError {
    /// Fragment size is zero.
    #[error("Fragment size must be greater than 0")]
    InvalidFragmentSize,
    /// Error indicating that a message type that cannot be fragmented was attempted to be fragmented.
    ///
    /// Only text and binary messages can be fragmented.
    #[error("Only text and binary messages can be fragmented")]
    CanNotBeFragmented,
}

/// General WebSocket error type.
#[derive(Debug, thiserror::Error)]
pub enum Error<I, E = Infallible> {
    /// Error reading from the WebSocket connection.
    #[error("Read error: {0}")]
    Read(
        #[from]
        #[source]
        ReadError<I>,
    ),
    /// Error writing to the WebSocket connection.
    #[error("Write error: {0}")]
    Write(
        #[from]
        #[source]
        WriteError<I>,
    ),
    /// Handshake error.
    #[error("Handshake error: {0}")]
    Handshake(
        #[from]
        #[source]
        HandshakeError<E>,
    ),
    /// Fragmentation error.
    #[error("Fragment error: {0}")]
    Fragmentation(
        #[from]
        #[source]
        FragmentationError,
    ),
}
