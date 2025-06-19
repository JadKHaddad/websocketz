use crate::{Frame, OpCode};

/// Iterator for fragmenting WebSocket messages.
///
/// handles empty and non-empty payloads.
///
/// See [`FragmentsIterator::new`].
pub enum FragmentsIterator<'a> {
    Once(core::iter::Once<Frame<'a>>),
    Iter(Iter<'a>),
}

impl<'a> FragmentsIterator<'a> {
    /// Must `NOT` be called with:
    ///
    /// - `fragment_size` = 0. Otherwise, it will produce an infinite iterator of non-final frames with empty payloads.
    /// - `opcode` != `OpCode::Text` or `OpCode::Binary`. Otherwise it will produce an invalid frame.
    pub fn new(opcode: OpCode, data: &'a [u8], fragment_size: usize) -> Self {
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
    pub const fn new(data: &'a [u8], opcode: OpCode, fragment_size: usize) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_payload_should_produce_one_frame() {
        let mut fragments = FragmentsIterator::new(OpCode::Text, &[], 10);

        assert_eq!(fragments.next(), Some(Frame::new(true, OpCode::Text, &[])));
        assert_eq!(fragments.next(), None);
    }

    /// See [`FragmentsIterator::new`].
    mod zero_fragment_size {
        use super::*;

        #[test]
        fn zero_fragment_size_should_produce_non_final_frames_with_empty_payload() {
            let mut fragments = FragmentsIterator::new(OpCode::Text, &[1, 2, 3], 0);

            assert_eq!(fragments.next(), Some(Frame::new(false, OpCode::Text, &[])));
            assert_eq!(fragments.next(), Some(Frame::new(false, OpCode::Text, &[])));
        }

        #[test]
        #[ignore = "TODO: how to test infinite iterators?"]
        fn zero_fragment_size_should_produce_infinite_non_final_frames_with_empty_payload() {
            // TODO
        }
    }

    mod ok_fragment_size {
        use super::*;

        #[test]
        fn less_than_payload_size() {
            let mut fragments = FragmentsIterator::new(OpCode::Binary, &[1, 2, 3, 4, 5], 1);

            assert_eq!(
                fragments.next(),
                Some(Frame::new(false, OpCode::Binary, &[1]))
            );
            assert_eq!(
                fragments.next(),
                Some(Frame::new(false, OpCode::Continuation, &[2]))
            );
            assert_eq!(
                fragments.next(),
                Some(Frame::new(false, OpCode::Continuation, &[3]))
            );
            assert_eq!(
                fragments.next(),
                Some(Frame::new(false, OpCode::Continuation, &[4]))
            );
            assert_eq!(
                fragments.next(),
                Some(Frame::new(true, OpCode::Continuation, &[5]))
            );
            assert_eq!(fragments.next(), None);

            let mut fragments = FragmentsIterator::new(OpCode::Text, &[1, 2, 3, 4, 5], 2);

            assert_eq!(
                fragments.next(),
                Some(Frame::new(false, OpCode::Text, &[1, 2]))
            );
            assert_eq!(
                fragments.next(),
                Some(Frame::new(false, OpCode::Continuation, &[3, 4]))
            );
            assert_eq!(
                fragments.next(),
                Some(Frame::new(true, OpCode::Continuation, &[5]))
            );
            assert_eq!(fragments.next(), None);
        }

        #[test]
        fn equal_to_payload_size() {
            let mut fragments = FragmentsIterator::new(OpCode::Text, &[1, 2, 3, 4, 5], 5);

            assert_eq!(
                fragments.next(),
                Some(Frame::new(true, OpCode::Text, &[1, 2, 3, 4, 5]))
            );
            assert_eq!(fragments.next(), None);
        }

        #[test]
        fn greater_than_payload_size() {
            let mut fragments = FragmentsIterator::new(OpCode::Text, &[1, 2, 3, 4, 5], 10);

            assert_eq!(
                fragments.next(),
                Some(Frame::new(true, OpCode::Text, &[1, 2, 3, 4, 5]))
            );
            assert_eq!(fragments.next(), None);
        }
    }
}
