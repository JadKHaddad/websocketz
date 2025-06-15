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
use http::{Request, RequestCodec, ResponseCodec};

mod mask;

mod message;
pub use message::Message;

#[doc(hidden)]
pub mod mock;

mod next;

mod opcode;
use opcode::OpCode;

mod options;
pub use options::Options;

mod websockets_core;
use websockets_core::WebsocketsCore;

mod websockets;
pub use websockets::{Websockets, WebsocketsRead, WebsocketsWrite};

#[cfg(test)]
mod tests;

#[cfg(test)]
extern crate std;
