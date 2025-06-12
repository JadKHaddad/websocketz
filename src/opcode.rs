use crate::error::DecodeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Continuation,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
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
            0x0 => Ok(OpCode::Continuation),
            0x1 => Ok(OpCode::Text),
            0x2 => Ok(OpCode::Binary),
            0x8 => Ok(OpCode::Close),
            0x9 => Ok(OpCode::Ping),
            0xA => Ok(OpCode::Pong),
            _ => Err(DecodeError::InvalidOpCode),
        }
    }
}
