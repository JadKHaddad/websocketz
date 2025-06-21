use embedded_io_adapters::tokio_1::FromTokio;
use rand::{SeedableRng, rngs::StdRng};

use crate::{CloseCode, Message, WebSocket, next};

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
    use super::*;

    #[tokio::test]
    async fn handshake() {
        let (client, server) = tokio::io::duplex(16);

        // Handshake requires larger buffers than SIZE
        let read_buf = &mut [0u8; SIZE * 2];
        let write_buf = &mut [0u8; SIZE * 2];
        let fragments_buf = &mut [];

        let server = async move {
            let io = hyper_util::rt::TokioIo::new(server);
            hyper::server::conn::http1::Builder::new()
                .serve_connection(
                    io,
                    hyper::service::service_fn(|mut req| async move {
                        let (response, fut) = fastwebsockets::upgrade::upgrade(&mut req).unwrap();

                        tokio::spawn(async move {
                            let mut ws = fut.await.unwrap();

                            ws.write_frame(fastwebsockets::Frame::close(1000, b"close"))
                                .await
                                .unwrap();
                        });

                        Ok::<_, fastwebsockets::WebSocketError>(response)
                    }),
                )
                .with_upgrades()
                .await
                .unwrap();
        };

        let client = async move {
            let mut websocketz = WebSocket::connect::<16>(
                "/",
                &[],
                FromTokio::new(client),
                StdRng::from_os_rng(),
                read_buf,
                write_buf,
                fragments_buf,
            )
            .await
            .unwrap();

            match next!(websocketz) {
                Some(Ok(Message::Close(Some(frame)))) => {
                    assert_eq!(frame.code(), CloseCode::Normal);
                    assert_eq!(frame.reason(), "close");
                }
                message => {
                    panic!("Unexpected message: {message:?}");
                }
            }
        };

        tokio::join!(server, client);
    }

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
            let mut websocketz = WebSocket::client(
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
            let mut websocketz = WebSocket::client(
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
            let mut websocketz = WebSocket::client(
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

mod server {
    use std::println;

    use bytes::Bytes;
    use http::{
        Request,
        header::{CONNECTION, UPGRADE},
    };
    use http_body_util::Empty;
    use tokio::io::AsyncWriteExt;

    use crate::{
        CloseFrame,
        error::{Error, HandshakeError},
    };

    use super::*;

    struct SpawnExecutor;

    impl<Fut> hyper::rt::Executor<Fut> for SpawnExecutor
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        fn execute(&self, fut: Fut) {
            tokio::task::spawn(fut);
        }
    }

    #[tokio::test]
    async fn wrong_http_method() {
        const REQUEST: &str = "POST / HTTP/1.1\r\n\
            Host: localhost\r\n\
            Upgrade: websocket\r\n\
            Connection: upgrade\r\n\
            Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
            Sec-WebSocket-Version: 13\r\n\
            \r\n";

        let (server, mut client) = tokio::io::duplex(16);

        let read_buf = &mut [0u8; SIZE * 2];
        let write_buf = &mut [0u8; SIZE * 2];
        let fragments_buf = &mut [];

        let server = async move {
            match WebSocket::accept::<16>(
                &[],
                FromTokio::new(server),
                StdRng::from_os_rng(),
                read_buf,
                write_buf,
                fragments_buf,
            )
            .await
            {
                Ok(_) => panic!("Expected error, but got Ok"),
                Err(error) => {
                    assert!(matches!(
                        error,
                        Error::Handshake(HandshakeError::WrongHttpMethod)
                    ));
                }
            }
        };

        let client = async move {
            client.write_all(REQUEST.as_bytes()).await.unwrap();
        };

        tokio::join!(server, client);
    }

    #[tokio::test]
    async fn wrong_http_version() {
        const REQUEST: &str = "GET / HTTP/1.0\r\n\
            Host: localhost\r\n\
            Upgrade: websocket\r\n\
            Connection: upgrade\r\n\
            Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
            Sec-WebSocket-Version: 13\r\n\
            \r\n";

        let (server, mut client) = tokio::io::duplex(16);

        let read_buf = &mut [0u8; SIZE * 2];
        let write_buf = &mut [0u8; SIZE * 2];
        let fragments_buf = &mut [];

        let server = async move {
            match WebSocket::accept::<16>(
                &[],
                FromTokio::new(server),
                StdRng::from_os_rng(),
                read_buf,
                write_buf,
                fragments_buf,
            )
            .await
            {
                Ok(_) => panic!("Expected error, but got Ok"),
                Err(error) => {
                    println!("Error: {:?}", error);
                    assert!(matches!(
                        error,
                        Error::Handshake(HandshakeError::WrongHttpVersion)
                    ));
                }
            }
        };

        let client = async move {
            client.write_all(REQUEST.as_bytes()).await.unwrap();
        };

        tokio::join!(server, client);
    }

    #[tokio::test]
    async fn handshake() {
        let (server, client) = tokio::io::duplex(16);

        // Handshake requires larger buffers than SIZE
        let read_buf = &mut [0u8; SIZE * 2];
        let write_buf = &mut [0u8; SIZE * 2];
        let fragments_buf = &mut [];

        let server = async move {
            let mut websocketz = WebSocket::accept::<16>(
                &[],
                FromTokio::new(server),
                StdRng::from_os_rng(),
                read_buf,
                write_buf,
                fragments_buf,
            )
            .await
            .unwrap();

            websocketz
                .send(Message::Close(Some(CloseFrame::new(
                    CloseCode::Normal,
                    "close",
                ))))
                .await
                .unwrap();

            websocketz.into_inner()
        };

        let client = async move {
            let req = Request::builder()
                .method("GET")
                .uri("/")
                .header(UPGRADE, "websocket")
                .header(CONNECTION, "upgrade")
                .header(
                    "Sec-WebSocket-Key",
                    fastwebsockets::handshake::generate_key(),
                )
                .header("Sec-WebSocket-Version", "13")
                .body(Empty::<Bytes>::new())
                .unwrap();

            let (mut fastwebsockets, _) =
                fastwebsockets::handshake::client(&SpawnExecutor, req, client)
                    .await
                    .unwrap();

            match fastwebsockets.read_frame().await {
                Ok(frame) => match frame.opcode {
                    fastwebsockets::OpCode::Close => {
                        let payload: &[u8] = frame.payload.as_ref();
                        let code = u16::from_be_bytes([payload[0], payload[1]]);
                        let reason = core::str::from_utf8(&payload[2..]).unwrap();

                        assert_eq!(code, 1000);
                        assert_eq!(reason, "close");
                    }
                    _ => panic!("Unexpected frame opcode"),
                },
                Err(fastwebsockets::WebSocketError::UnexpectedEOF) => {}
                _ => panic!("Unexpected frame"),
            }
        };

        // Keep io to prevent BrokenPipe error
        let (_io, _) = tokio::join!(server, client);
    }

    #[tokio::test]
    async fn send() {
        let (server, client) = tokio::io::duplex(16);

        let read_buf = &mut [0u8; SIZE];
        let write_buf = &mut [0u8; SIZE];
        let fragments_buf = &mut [0u8; SIZE];

        let server = async move {
            let mut websocketz = WebSocket::server(
                FromTokio::new(server),
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

        let client = async move {
            let mut fastwebsockets =
                fastwebsockets::WebSocket::after_handshake(client, fastwebsockets::Role::Client);

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

        tokio::join!(server, client);
    }

    #[tokio::test]
    async fn send_fragmented() {
        let (server, client) = tokio::io::duplex(16);

        let read_buf = &mut [0u8; SIZE];
        let write_buf = &mut [0u8; SIZE];
        let fragments_buf = &mut [0u8; SIZE];

        let server = async move {
            let mut websocketz = WebSocket::server(
                FromTokio::new(server),
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

        let client = async move {
            let mut fastwebsockets = fastwebsockets::FragmentCollector::new(
                fastwebsockets::WebSocket::after_handshake(client, fastwebsockets::Role::Client),
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

        tokio::join!(server, client);
    }

    #[tokio::test]
    async fn receive() {
        let (server, client) = tokio::io::duplex(16);

        let read_buf = &mut [0u8; SIZE];
        let write_buf = &mut [0u8; SIZE];
        let fragments_buf = &mut [0u8; SIZE];

        let client = async move {
            let mut fastwebsockets =
                fastwebsockets::WebSocket::after_handshake(client, fastwebsockets::Role::Client);

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

        let server = async move {
            let mut websocketz = WebSocket::server(
                FromTokio::new(server),
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

// TODO: test every possible error variant
