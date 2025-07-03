use crate::Message;

#[derive(Debug)]
pub enum OnMessage<'a> {
    Send(Message<'a>),
    Noop(Message<'a>),
}
