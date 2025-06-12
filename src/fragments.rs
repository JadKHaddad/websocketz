use crate::{Frame, OpCode};

pub enum FragmentsIterator<'a> {
    Once(core::iter::Once<Frame<'a>>),
    Iter(Iter<'a>),
}

impl<'a> FragmentsIterator<'a> {
    pub fn new(data: &'a [u8], opcode: OpCode, fragment_size: usize) -> Self {
        match data.len() {
            0 => Self::Once(core::iter::once(Frame::new(true, opcode, &[]))),
            _ => Self::Iter(Iter::new(data, opcode, fragment_size)),
        }
    }
}

impl<'a> Iterator for FragmentsIterator<'a> {
    type Item = Frame<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Once(iter) => iter.next(),
            Self::Iter(iter) => iter.next(),
        }
    }
}

pub struct Iter<'a> {
    data: &'a [u8],
    opcode: OpCode,
    fragment_size: usize,
    pos: usize,
}

impl<'a> Iter<'a> {
    pub fn new(data: &'a [u8], opcode: OpCode, fragment_size: usize) -> Self {
        Self {
            data,
            opcode,
            fragment_size,
            pos: 0,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Frame<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }

        let start = self.pos;
        let end = (self.pos + self.fragment_size).min(self.data.len());
        self.pos = end;

        let fin = self.pos == self.data.len();
        let opcode = if start == 0 {
            self.opcode
        } else {
            OpCode::Continuation
        };

        let payload = &self.data[start..end];

        Some(Frame::new(fin, opcode, payload))
    }
}
