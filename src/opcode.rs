use crate::error::DecodeError;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

impl OpCode {
    pub fn is_control(&self) -> bool {
        matches!(self, OpCode::Close | OpCode::Ping | OpCode::Pong)
    }
}

impl TryFrom<u8> for OpCode {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            v if v == OpCode::Continuation as u8 => Ok(OpCode::Continuation),
            v if v == OpCode::Text as u8 => Ok(OpCode::Text),
            v if v == OpCode::Binary as u8 => Ok(OpCode::Binary),
            v if v == OpCode::Close as u8 => Ok(OpCode::Close),
            v if v == OpCode::Ping as u8 => Ok(OpCode::Ping),
            v if v == OpCode::Pong as u8 => Ok(OpCode::Pong),
            _ => Err(DecodeError::InvalidOpCode),
        }
    }
}
