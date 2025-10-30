//! A collection of examples to be used in the documentation.

mod lib {

    #[tokio::test]
    async fn client() {
        use crate::mock::Noop;
        use crate::{Message, WebSocket, http::Header, next, options::ConnectOptions};

        // An already connected stream.
        // Impl embedded_io_async Read + Write.
        let stream = Noop;

        let read_buffer = &mut [0u8; 1024];
        let write_buffer = &mut [0u8; 1024];
        let fragments_buffer = &mut [0u8; 1024];

        // Impl rand_core RngCore.
        let rng = Noop;

        // Perform a WebSocket handshake as a client.
        // 16 is the max number of headers to allocate space for.
        let mut websocketz = WebSocket::connect::<16>(
            // Set the connection options.
            // The path for the WebSocket endpoint as well as any additional HTTP headers.
            ConnectOptions::default()
                .with_path("/ws")
                .expect("Valid path")
                .with_headers(&[
                    Header {
                        name: "Host",
                        value: b"example.com",
                    },
                    Header {
                        name: "User-Agent",
                        value: b"WebSocketz",
                    },
                ]),
            stream,
            rng,
            read_buffer,
            write_buffer,
            fragments_buffer,
        )
        .await
        .expect("Handshake failed");

        // Send a text message.
        websocketz
            .send(Message::Text("Hello, WebSocket!"))
            .await
            .expect("Failed to send message");

        // Receive messages in a loop.
        loop {
            match next!(websocketz) {
                None => {
                    // Connection closed.
                    break;
                }
                Some(Ok(msg)) => {
                    // Handle received message.
                    let _ = msg;
                }
                Some(Err(err)) => {
                    // Handle error.
                    let _ = err;

                    break;
                }
            }
        }
    }

    #[tokio::test]
    async fn server() {
        use crate::mock::Noop;
        use crate::{Message, WebSocket, http::Header, next, options::AcceptOptions};

        // An already connected stream.
        // Impl embedded_io_async Read + Write.
        let stream = Noop;

        let read_buffer = &mut [0u8; 1024];
        let write_buffer = &mut [0u8; 1024];
        let fragments_buffer = &mut [0u8; 1024];

        // Impl rand_core RngCore.
        let rng: Noop = Noop;

        // Perform a WebSocket handshake as a server.
        // 16 is the max number of headers to allocate space for.
        let mut websocketz = WebSocket::accept::<16>(
            // Set the acceptance options.
            // Any additional HTTP headers.
            AcceptOptions::default().with_headers(&[Header {
                name: "Server",
                value: b"WebSocketz",
            }]),
            stream,
            rng,
            read_buffer,
            write_buffer,
            fragments_buffer,
        )
        .await
        .expect("Handshake failed");

        // Receive messages in a loop.
        loop {
            match next!(websocketz) {
                None => {
                    // Connection closed.
                    break;
                }
                Some(Ok(msg)) => {
                    // Handle received message.
                    let _ = msg;

                    // Send a binary message.
                    if let Err(err) = websocketz.send(Message::Binary(b"Hello, WebSocket!")).await {
                        let _ = err;

                        break;
                    }
                }
                Some(Err(err)) => {
                    // Handle error.
                    let _ = err;

                    break;
                }
            }
        }
    }
}
