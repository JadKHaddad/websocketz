#![no_std]
#![deny(missing_debug_implementations)]
// #![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

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

mod http;
pub use http::{Request, Response};

mod mask;

mod message;
pub use message::Message;

#[doc(hidden)]
pub mod mock;

mod next;

mod opcode;
use opcode::OpCode;

pub mod options;

mod websocket_core;
use websocket_core::WebSocketCore;

mod websocket;
pub use websocket::{WebSocket, WebSocketRead, WebSocketWrite};

#[cfg(test)]
mod tests;

#[cfg(test)]
extern crate std;
