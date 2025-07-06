use embedded_io_async::{Read, Write};
use framez::state::{ReadState, WriteState};
use rand::RngCore;

use crate::{
    Frame, Message, OnFrame, WebSocketCore,
    codec::FramesCodec,
    error::{Error, ProtocolError, ReadError, WriteError},
    websocket_core::FragmentsState,
};

#[derive(Debug)]
pub struct ReadAutoCaller;

impl ReadAutoCaller {
    pub async fn call<'this, F, RW, Rng>(
        &self,
        auto: F,
        codec: &mut FramesCodec<Rng>,
        inner: &mut RW,
        read_state: &'this mut ReadState<'_>,
        write_state: &mut WriteState<'_>,
        fragments_state: &'this mut FragmentsState<'_>,
    ) -> Option<Result<Option<Message<'this>>, Error<RW::Error>>>
    where
        RW: Read + Write,
        Rng: RngCore,
        F: FnOnce(Frame<'_>) -> Result<OnFrame<'_>, ProtocolError> + 'static,
    {
        let frame = match framez::functions::maybe_next(read_state, codec, inner).await {
            Some(Ok(Some(frame))) => frame,
            Some(Ok(None)) => return Some(Ok(None)),
            Some(Err(err)) => return Some(Err(Error::Read(ReadError::ReadFrame(err)))),
            None => return None,
        };

        let frame = match auto(frame) {
            Ok(on_frame) => match on_frame {
                OnFrame::Send(message) => {
                    let is_close = message.is_close();

                    match framez::functions::send(write_state, codec, inner, message).await {
                        Ok(_) => match is_close {
                            false => return Some(Ok(None)),
                            true => return None,
                        },
                        Err(err) => return Some(Err(Error::Write(WriteError::WriteFrame(err)))),
                    }
                }
                OnFrame::Noop(frame) => frame,
            },
            Err(err) => return Some(Err(Error::Read(ReadError::Protocol(err)))),
        };

        WebSocketCore::<RW, Rng>::on_frame(fragments_state, frame)
            .map(|result| result.map_err(Error::from))
    }
}

#[derive(Debug)]
pub struct ReadCaller;

impl ReadCaller {
    pub async fn call<'this, RW, Rng>(
        &self,
        _auto: (),
        codec: &mut FramesCodec<Rng>,
        inner: &mut RW,
        read_state: &'this mut ReadState<'_>,
        _write_state: &mut WriteState<'_>,
        fragments_state: &'this mut FragmentsState<'_>,
    ) -> Option<Result<Option<Message<'this>>, Error<RW::Error>>>
    where
        RW: Read,
    {
        let frame = match framez::functions::maybe_next(read_state, codec, inner).await {
            Some(Ok(Some(frame))) => frame,
            Some(Ok(None)) => return Some(Ok(None)),
            Some(Err(err)) => return Some(Err(Error::Read(ReadError::ReadFrame(err)))),
            None => return None,
        };

        WebSocketCore::<RW, Rng>::on_frame(fragments_state, frame)
            .map(|result| result.map_err(Error::from))
    }
}

pub async fn send<RW, Rng>(
    codec: &mut FramesCodec<Rng>,
    inner: &mut RW,
    write_state: &mut WriteState<'_>,
    message: Message<'_>,
) -> Result<(), Error<RW::Error>>
where
    RW: Write,
    Rng: RngCore,
{
    framez::functions::send(write_state, codec, inner, message)
        .await
        .map_err(|err| Error::Write(WriteError::WriteFrame(err)))?;

    Ok(())
}
