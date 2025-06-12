#[derive(Debug)]
pub enum DecodeError {
    ReservedBitsNotZero,
    InvalidOpCode,
    ControlFrameFragmented,
    PingFrameTooLarge,
}

#[derive(Debug)]
pub enum EncodeError {
    BufferTooSmall,
}
