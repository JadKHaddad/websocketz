const SIZE: usize = 128;

// cSpell:disable
const BINARY_MESSAGES: &[&[u8]] = &[
    b"Hello, world!",
    b"Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
    b"Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium.",
    b"Ut enim ad minima veniam, quis nostrum exercitationem ullam corporis suscipit.",
    b"Curabitur pretium tincidunt lacus. Nulla gravida orci a odio.",
    b"Aenean nec eros. Vestibulum ante ipsum primis in faucibus orci luctus et.",
    b"Integer tincidunt. Cras dapibus. Vivamus elementum semper nisi.",
    b"Donec pede justo, fringilla vel, aliquet nec, vulputate eget, arcu.",
    b"In enim justo, rhoncus ut, imperdiet a, venenatis vitae, justo.",
];

const STR_MESSAGES: &[&str] = &[
    "Hello, world!",
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
    "Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium.",
    "Ut enim ad minima veniam, quis nostrum exercitationem ullam corporis suscipit.",
    "Curabitur pretium tincidunt lacus. Nulla gravida orci a odio.",
    "Aenean nec eros. Vestibulum ante ipsum primis in faucibus orci luctus et.",
    "Integer tincidunt. Cras dapibus. Vivamus elementum semper nisi.",
    "Donec pede justo, fringilla vel, aliquet nec, vulputate eget, arcu.",
    "In enim justo, rhoncus ut, imperdiet a, venenatis vitae, justo.",
];
// cSpell:enable

mod client {
    use embedded_io_adapters::tokio_1::FromTokio;
    use rand::{SeedableRng, rngs::StdRng};

    use crate::{Message, Websockets, next};

    use super::*;

    #[tokio::test]
    async fn send() {
        let (client, server) = tokio::io::duplex(16);

        let read_buf = &mut [0u8; SIZE];
        let write_buf = &mut [0u8; SIZE];
        let fragments_buf = &mut [0u8; SIZE];

        let server = async move {
            let mut fastwebsockets =
                fastwebsockets::WebSocket::after_handshake(server, fastwebsockets::Role::Server);

            let mut bin_index = 0;
            let mut str_index = 0;

            loop {
                match fastwebsockets.read_frame().await {
                    Ok(frame) => match frame.opcode {
                        fastwebsockets::OpCode::Binary => {
                            assert_eq!(frame.payload, BINARY_MESSAGES[bin_index]);
                            bin_index += 1;
                        }
                        fastwebsockets::OpCode::Text => {
                            let text = core::str::from_utf8(&frame.payload).unwrap();
                            assert_eq!(text, STR_MESSAGES[str_index]);
                            str_index += 1;
                        }
                        _ => panic!("Unexpected frame opcode"),
                    },
                    Err(fastwebsockets::WebSocketError::UnexpectedEOF) => break,
                    _ => panic!("Unexpected frame"),
                }
            }
        };

        let client = async move {
            let mut websocketz = Websockets::client(
                FromTokio::new(client),
                StdRng::from_os_rng(),
                read_buf,
                write_buf,
                fragments_buf,
            );

            for binary in BINARY_MESSAGES {
                websocketz
                    .send(Message::Binary(binary))
                    .await
                    .expect("Failed to send binary message");
            }

            for text in STR_MESSAGES {
                websocketz
                    .send(Message::Text(text))
                    .await
                    .expect("Failed to send text message");
            }
        };

        tokio::join!(server, client);
    }

    #[tokio::test]
    async fn send_fragmented() {
        let (client, server) = tokio::io::duplex(16);

        let read_buf = &mut [0u8; SIZE];
        let write_buf = &mut [0u8; SIZE];
        let fragments_buf = &mut [0u8; SIZE];

        let server = async move {
            let mut fastwebsockets = fastwebsockets::FragmentCollector::new(
                fastwebsockets::WebSocket::after_handshake(server, fastwebsockets::Role::Server),
            );

            let mut bin_index = 0;
            let mut str_index = 0;

            loop {
                match fastwebsockets.read_frame().await {
                    Ok(frame) => match frame.opcode {
                        fastwebsockets::OpCode::Binary => {
                            assert_eq!(frame.payload, BINARY_MESSAGES[bin_index]);
                            bin_index += 1;
                        }
                        fastwebsockets::OpCode::Text => {
                            let text = core::str::from_utf8(&frame.payload).unwrap();
                            assert_eq!(text, STR_MESSAGES[str_index]);
                            str_index += 1;
                        }
                        _ => panic!("Unexpected frame opcode"),
                    },
                    Err(fastwebsockets::WebSocketError::UnexpectedEOF) => break,
                    _ => panic!("Unexpected frame"),
                }
            }
        };

        let client = async move {
            let mut websocketz = Websockets::client(
                FromTokio::new(client),
                StdRng::from_os_rng(),
                read_buf,
                write_buf,
                fragments_buf,
            );

            for binary in BINARY_MESSAGES {
                websocketz
                    .send_fragmented(Message::Binary(binary), 16)
                    .await
                    .expect("Failed to send binary message");
            }

            for text in STR_MESSAGES {
                websocketz
                    .send_fragmented(Message::Text(text), 16)
                    .await
                    .expect("Failed to send text message");
            }
        };

        tokio::join!(server, client);
    }

    #[tokio::test]
    async fn receive() {
        let (client, server) = tokio::io::duplex(16);

        let read_buf = &mut [0u8; SIZE];
        let write_buf = &mut [0u8; SIZE];
        let fragments_buf = &mut [0u8; SIZE];

        let server = async move {
            let mut fastwebsockets =
                fastwebsockets::WebSocket::after_handshake(server, fastwebsockets::Role::Server);

            for binary in BINARY_MESSAGES {
                fastwebsockets
                    .write_frame(fastwebsockets::Frame::binary(
                        fastwebsockets::Payload::Borrowed(binary),
                    ))
                    .await
                    .expect("Failed to send binary message");
            }

            for text in STR_MESSAGES {
                fastwebsockets
                    .write_frame(fastwebsockets::Frame::text(
                        fastwebsockets::Payload::Borrowed(text.as_bytes()),
                    ))
                    .await
                    .expect("Failed to send text message");
            }
        };

        let client = async move {
            let mut websocketz = Websockets::client(
                FromTokio::new(client),
                StdRng::from_os_rng(),
                read_buf,
                write_buf,
                fragments_buf,
            );

            let mut bin_index = 0;
            let mut str_index = 0;

            loop {
                match next!(websocketz) {
                    Some(Ok(Message::Binary(payload))) => {
                        assert_eq!(payload, BINARY_MESSAGES[bin_index]);
                        bin_index += 1;
                    }
                    Some(Ok(Message::Text(payload))) => {
                        assert_eq!(payload, STR_MESSAGES[str_index]);
                        str_index += 1;
                    }
                    None => break,
                    message => panic!("Unexpected message: {message:?}"),
                }
            }
        };

        tokio::join!(server, client);
    }
}

// TODO
mod server {
    use super::*;
}

// TODO
mod handshake {
    use super::*;
}
