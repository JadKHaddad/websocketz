use crate::OpCode;

/// A received frame.
#[derive(Debug)]
pub struct Frame<'a> {
    /// Indicates if this is the final frame in a message.
    fin: bool,
    /// The opcode of the frame.
    opcode: OpCode,
    /// The payload of the frame.
    payload: &'a [u8],
}

impl<'a> Frame<'a> {
    /// Creates a new `Frame` instance.
    pub fn new(fin: bool, opcode: OpCode, payload: &'a [u8]) -> Self {
        Self {
            fin,
            opcode,
            payload,
        }
    }

    /// Returns whether this is the final frame in a message.
    pub fn is_final(&self) -> bool {
        self.fin
    }

    /// Returns the opcode of the frame.
    pub fn opcode(&self) -> OpCode {
        self.opcode
    }

    /// Returns the payload of the frame.
    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }

    pub fn write_payload(&self, dst: &mut [u8]) -> Option<usize> {
        if dst.len() < self.payload.len() {
            return None;
        }

        dst[..self.payload.len()].copy_from_slice(self.payload);

        Some(self.payload.len())
    }
}

/// A mutable received frame.
#[derive(Debug)]
pub struct FrameMut<'a> {
    /// Indicates if this is the final frame in a message.
    fin: bool,
    /// The opcode of the frame.
    opcode: OpCode,
    /// The masking key of the frame, if any.
    mask: Option<[u8; 4]>,
    /// The payload of the frame.
    payload: &'a mut [u8],
}

impl<'a> FrameMut<'a> {
    /// Creates a new `FrameMut` instance.
    pub fn new(fin: bool, opcode: OpCode, mask: Option<[u8; 4]>, payload: &'a mut [u8]) -> Self {
        Self {
            fin,
            opcode,
            mask,
            payload,
        }
    }

    pub const fn into_frame(self) -> Frame<'a> {
        Frame {
            fin: self.fin,
            opcode: self.opcode,
            payload: self.payload,
        }
    }

    pub fn unmask(&mut self) {
        if let Some(mask) = self.mask {
            crate::mask::unmask(self.payload, mask);
        }
    }
}

#[derive(Debug)]
pub struct Header {
    /// Indicates if this is the final frame in a message.
    fin: bool,
    /// The opcode of the frame.
    opcode: OpCode,
    /// The length of the payload.
    payload_len: usize,
}

impl Header {
    pub fn new(fin: bool, opcode: OpCode, payload_len: usize) -> Self {
        Self {
            fin,
            opcode,
            payload_len,
        }
    }

    /// writes the header into the dst buffer.
    pub fn write(&self, dst: &mut [u8]) -> Option<usize> {
        if dst.len() < 2 {
            return None;
        }

        dst[0] = (self.fin as u8) << 7 | (self.opcode as u8);

        let len = self.payload_len;

        let size = if len < 126 {
            dst[1] = len as u8;
            2
        } else if len < 65536 {
            if dst.len() < 4 {
                return None;
            }

            dst[1] = 126;
            dst[2..4].copy_from_slice(&(len as u16).to_be_bytes());

            4
        } else {
            if dst.len() < 10 {
                return None;
            }

            dst[1] = 127;
            dst[2..10].copy_from_slice(&(len as u64).to_be_bytes());

            10
        };

        Some(size)
    }
}
