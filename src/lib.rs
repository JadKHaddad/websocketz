#![no_std]
#![deny(missing_debug_implementations)]
// #![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod next;

pub mod error;

mod close_code;
pub use close_code::CloseCode;

mod close_frame;
pub use close_frame::CloseFrame;

mod codec;
use codec::FramesCodec;

mod frame;
use frame::{Frame, FrameMut, Header};

mod opcode;
use opcode::OpCode;

mod mask;

mod fragments;

mod message;
pub use message::Message;

mod websockets_core;
use websockets_core::WebsocketsCore;

mod websockets;
pub use websockets::{Websockets, WebsocketsRead, WebsocketsWrite};

#[doc(hidden)]
pub mod mock;
