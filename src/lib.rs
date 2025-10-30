//! `zerocopy`, `async`, `no_std` and [`autobahn`](https://github.com/crossbario/autobahn-testsuite) compliant `websockets` implementation.

// TODO: examples

#![no_std]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! # Examples
//!
//! In the following examples, `Noop` is a mock type that implements the required traits for using a [`WebSocket`].
//! - A `stream` is anything that implements [`embedded_io_async::Read`] + [`embedded_io_async::Write`].
//! - A `rng` is anything that implements [`rand_core::RngCore`].
//!
//! ## Client
//! ```no_run
//! # async fn client() {
//! # use crate::mock::Noop;
//! use crate::{Message, WebSocket, http::Header, next, options::ConnectOptions};
//!
//! // An already connected stream.
//! // Impl embedded_io_async Read + Write.
//! let stream = Noop;
//!
//! let read_buffer = &mut [0u8; 1024];
//! let write_buffer = &mut [0u8; 1024];
//! let fragments_buffer = &mut [0u8; 1024];
//!
//! // Impl rand_core RngCore.
//! let rng = Noop;
//!
//! // Perform a WebSocket handshake as a client.
//! // 16 is the max number of headers to allocate space for.
//! let mut websocketz = WebSocket::connect::<16>(
//!     // Set the connection options.
//!     // The path for the WebSocket endpoint as well as any additional HTTP headers.
//!     ConnectOptions::default()
//!         .with_path("/ws")
//!         .expect("Valid path")
//!         .with_headers(&[
//!             Header {
//!                 name: "Host",
//!                 value: b"example.com",
//!             },
//!             Header {
//!                 name: "User-Agent",
//!                 value: b"WebSocketz",
//!             },
//!         ]),
//!     stream,
//!     rng,
//!     read_buffer,
//!     write_buffer,
//!     fragments_buffer,
//! )
//! .await
//! .expect("Handshake failed");
//!
//! // Send a text message.
//! websocketz
//!     .send(Message::Text("Hello, WebSocket!"))
//!     .await
//!     .expect("Failed to send message");
//!
//! // Receive messages in a loop.
//! loop {
//!     match next!(websocketz) {
//!         None => {
//!             // Connection closed.
//!             break;
//!         }
//!         Some(Ok(msg)) => {
//!             // Handle received message.
//!             let _ = msg;
//!         }
//!         Some(Err(err)) => {
//!             // Handle error.
//!             let _ = err;
//!
//!             break;
//!         }
//!     }
//! }
//! # }
//! ```
//!
//! ## Server
//! ```no_run
//! # async fn server() {
//! # use crate::mock::Noop;
//! use crate::{Message, WebSocket, http::Header, next, options::AcceptOptions};
//!
//! // An already connected stream.
//! // Impl embedded_io_async Read + Write.
//! let stream = Noop;
//!
//! let read_buffer = &mut [0u8; 1024];
//! let write_buffer = &mut [0u8; 1024];
//! let fragments_buffer = &mut [0u8; 1024];
//!
//! // Impl rand_core RngCore.
//! let rng: Noop = Noop;
//!
//! // Perform a WebSocket handshake as a server.
//! // 16 is the max number of headers to allocate space for.
//! let mut websocketz = WebSocket::accept::<16>(
//!     // Set the acceptance options.
//!     // Any additional HTTP headers.
//!     AcceptOptions::default().with_headers(&[Header {
//!         name: "Server",
//!         value: b"WebSocketz",
//!     }]),
//!     stream,
//!     rng,
//!     read_buffer,
//!     write_buffer,
//!     fragments_buffer,
//! )
//! .await
//! .expect("Handshake failed");
//!
//! // Receive messages in a loop.
//! loop {
//!     match next!(websocketz) {
//!         None => {
//!             // Connection closed.
//!             break;
//!         }
//!         Some(Ok(msg)) => {
//!             // Handle received message.
//!             let _ = msg;
//!
//!             // Send a binary message.
//!             if let Err(err) = websocketz.send(Message::Binary(b"Hello, WebSocket!")).await {
//!                 let _ = err;
//!
//!                 break;
//!             }
//!         }
//!         Some(Err(err)) => {
//!             // Handle error.
//!             let _ = err;
//!
//!             break;
//!         }
//!     }
//! }
//! # }
//! ```
//!
//! # Laziness
//!
//! This library is `lazy`, meaning that the WebSocket connection is managed as long as you read from the connection.
//!
//! Managing the connection consists of two parts:
//! - Sending [Message::Pong] messages in response to [Message::Ping] messages.
//! - Responding to [Message::Close] messages by sending the appropriate [Message::Close] response and closing the connection.
//!
//! `auto_pong` and `auto_close` are enabled by default, but can be set using [`WebSocket::with_auto_pong`] and [`WebSocket::with_auto_close`] respectively.
//!
//! # Reading from the connection
//!
//! This library allocates nothing. It only uses exclusive references and stack memory. It is quite challenging to offer a clean API while adhering to rust's borrowing rules.
//! That's why a [`WebSocket`] does not offer any method to read messages directly.
//!
//! Instead, you can use the [`next!`] macro to read messages from the connection.
//!
//! [`next!`] unpacks the internal `private` structure of the [`WebSocket`] to obtain mutable references and perform reads.
//!
//! ```no_run
//! # async fn next_macro() {
//! # use crate::mock::Noop;
//! # use crate::{WebSocket, next, options::ConnectOptions};
//! #
//! # let stream = Noop;
//! # let read_buffer = &mut [0u8; 1024];
//! # let write_buffer = &mut [0u8; 1024];
//! # let fragments_buffer = &mut [0u8; 1024];
//! # let rng = Noop;
//! #
//! # let websocketz = WebSocket::connect::<16>(
//! #     ConnectOptions::default()
//! #         .with_path("/ws")
//! #         .expect("Valid path"),
//! #     stream,
//! #     rng,
//! #     read_buffer,
//! #     write_buffer,
//! #     fragments_buffer,
//! # )
//! # .await
//! # .expect("Handshake failed");
//! #
//! # let existing_websocket = || websocketz;
//! let mut websocketz = existing_websocket();
//!
//! while let Some(Ok(msg)) = next!(websocketz) {
//!     // Messages hold references to the websocket buffers.
//!     let _ = msg;
//! }
//! # }
mod close_code;
pub use close_code::CloseCode;

mod close_frame;
pub use close_frame::CloseFrame;

mod codec;
use codec::FramesCodec;

pub mod error;

mod fragments;

mod frame;
use frame::{Frame, FrameMut, Header};

#[doc(hidden)]
pub mod functions;

pub mod http;

mod mask;

mod message;
pub use message::Message;

#[doc(hidden)]
pub mod mock;

mod macros;

mod opcode;
use opcode::OpCode;

pub mod options;

mod websocket_core;
use websocket_core::{ConnectionState, FragmentsState, OnFrame, WebSocketCore};

mod websocket;
pub use websocket::{WebSocket, WebSocketRead, WebSocketWrite};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod examples;

#[cfg(test)]
extern crate std;
