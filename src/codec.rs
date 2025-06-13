use framez::{decode::Decoder, encode::Encoder};
use rand::Rng;
use rand_core::RngCore;

use crate::{
    Frame, FrameMut, Header, Message, OpCode,
    error::{DecodeError, EncodeError},
};

#[derive(Debug)]
enum DecodeState {
    Init,
    DecodedHeader {
        fin: bool,
        opcode: OpCode,
        masked: bool,
        length_code: u8,
        extra: usize,
        min_src_len: usize,
    },
    DecodedPayloadLength {
        fin: bool,
        opcode: OpCode,
        mask: Option<[u8; 4]>,
        payload_len: usize,
        min_src_len: usize,
    },
}

#[derive(Debug)]
pub struct FramesCodec<R = ()> {
    unmask: bool,
    mask: bool,
    decode_state: DecodeState,
    rng: R,
}

impl<R> FramesCodec<R> {
    pub fn new(rng: R) -> Self {
        Self {
            unmask: false,
            mask: false,
            decode_state: DecodeState::Init,
            rng,
        }
    }

    pub fn set_unmask(&mut self, unmask: bool) {
        self.unmask = unmask;
    }

    pub fn set_mask(&mut self, mask: bool) {
        self.mask = mask;
    }

    pub fn split(self) -> (FramesCodec<()>, FramesCodec<R>) {
        (
            FramesCodec {
                unmask: self.unmask,
                mask: self.mask,
                decode_state: self.decode_state,
                rng: (),
            },
            FramesCodec {
                unmask: self.unmask,
                mask: self.mask,
                decode_state: DecodeState::Init, // We don't care about the decode state in the second codec (writer)
                rng: self.rng,
            },
        )
    }
}

impl<R> framez::decode::DecodeError for FramesCodec<R> {
    type Error = DecodeError;
}

impl<'buf, R> Decoder<'buf> for FramesCodec<R> {
    type Item = Frame<'buf>;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        const MIN_HEADER_SIZE: usize = 2;

        loop {
            match self.decode_state {
                DecodeState::Init => {
                    if src.len() < MIN_HEADER_SIZE {
                        return Ok(None);
                    }

                    let fin = src[0] & 0b10000000 != 0;
                    let rsv1 = src[0] & 0b01000000 != 0;
                    let rsv2 = src[0] & 0b00100000 != 0;
                    let rsv3 = src[0] & 0b00010000 != 0;

                    if rsv1 || rsv2 || rsv3 {
                        return Err(DecodeError::ReservedBitsNotZero);
                    }

                    let opcode = OpCode::try_from(src[0] & 0b00001111)?;
                    let masked = src[1] & 0b10000000 != 0;

                    let length_code = src[1] & 0x7F;
                    let extra = match length_code {
                        126 => 2,
                        127 => 8,
                        _ => 0,
                    };

                    let min_src_len = MIN_HEADER_SIZE + extra + masked as usize * 4;

                    self.decode_state = DecodeState::DecodedHeader {
                        fin,
                        opcode,
                        masked,
                        length_code,
                        extra,
                        min_src_len,
                    };
                }
                DecodeState::DecodedHeader {
                    fin,
                    opcode,
                    masked,
                    length_code,
                    extra,
                    min_src_len,
                } => {
                    if src.len() < min_src_len {
                        return Ok(None);
                    }

                    let payload_len = match extra {
                        0 => length_code as usize,
                        2 => u16::from_be_bytes([src[2], src[3]]) as usize,
                        8 => u64::from_be_bytes([
                            src[2], src[3], src[4], src[5], src[6], src[7], src[8], src[9],
                        ]) as usize,
                        _ => unreachable!(),
                    };

                    let mask = masked.then(|| {
                        [
                            src[2 + extra],
                            src[3 + extra],
                            src[4 + extra],
                            src[5 + extra],
                        ]
                    });

                    if opcode.is_control() && !fin {
                        return Err(DecodeError::ControlFrameFragmented);
                    }

                    if matches!(opcode, OpCode::Ping) && payload_len > 125 {
                        return Err(DecodeError::PingFrameTooLarge);
                    }

                    let min_src_len = min_src_len + payload_len;

                    self.decode_state = DecodeState::DecodedPayloadLength {
                        fin,
                        opcode,
                        mask,
                        payload_len,
                        min_src_len,
                    };
                }
                DecodeState::DecodedPayloadLength {
                    fin,
                    opcode,
                    mask,
                    payload_len,
                    min_src_len,
                } => {
                    if src.len() < min_src_len {
                        return Ok(None);
                    }

                    let payload = &mut src[min_src_len - payload_len..min_src_len];

                    let mut frame = FrameMut::new(fin, opcode, mask, payload);

                    if self.unmask {
                        frame.unmask();
                    }

                    self.decode_state = DecodeState::Init;

                    return Ok(Some((frame.into_frame(), min_src_len)));
                }
            }
        }
    }
}

impl<R: RngCore> Encoder<Message<'_>> for FramesCodec<R> {
    type Error = EncodeError;

    fn encode(&mut self, item: Message, dst: &mut [u8]) -> Result<usize, Self::Error> {
        let header = Header::new(true, item.opcode(), item.len());

        let head_len = header
            .write(&mut dst[..])
            .ok_or(EncodeError::BufferTooSmall)?;

        let mask: Option<[u8; 4]> = self.mask.then(|| self.rng.random());

        let head_len = match mask {
            None => head_len,
            Some(mask) => {
                if head_len + 4 > dst.len() {
                    return Err(EncodeError::BufferTooSmall);
                }

                dst[1] |= 0x80;
                dst[head_len..head_len + 4].copy_from_slice(&mask);

                head_len + 4
            }
        };

        let payload_len = item
            .write(&mut dst[head_len..])
            .ok_or(EncodeError::BufferTooSmall)?;

        if let Some(mask) = mask {
            crate::mask::unmask(&mut dst[head_len..head_len + payload_len], mask);
        }

        Ok(head_len + payload_len)
    }
}

impl<R: RngCore> Encoder<Frame<'_>> for FramesCodec<R> {
    type Error = EncodeError;

    fn encode(&mut self, item: Frame, dst: &mut [u8]) -> Result<usize, Self::Error> {
        let header = Header::new(item.is_final(), item.opcode(), item.payload().len());

        let head_len = header
            .write(&mut dst[..])
            .ok_or(EncodeError::BufferTooSmall)?;

        let mask: Option<[u8; 4]> = self.mask.then(|| self.rng.random());

        let head_len = match mask {
            None => head_len,
            Some(mask) => {
                if head_len + 4 > dst.len() {
                    return Err(EncodeError::BufferTooSmall);
                }

                dst[1] |= 0x80;
                dst[head_len..head_len + 4].copy_from_slice(&mask);

                head_len + 4
            }
        };

        let payload_len = item
            .write_payload(&mut dst[head_len..])
            .ok_or(EncodeError::BufferTooSmall)?;

        if let Some(mask) = mask {
            crate::mask::unmask(&mut dst[head_len..head_len + payload_len], mask);
        }

        Ok(head_len + payload_len)
    }
}
