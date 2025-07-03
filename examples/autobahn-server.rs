use std::convert::Infallible;

use embedded_io_adapters::tokio_1::FromTokio;
use rand::{SeedableRng, rngs::StdRng};
use tokio::net::TcpListener;
use websocketz::{Message, OnMessage, WebSocket, error::Error, next, options::AcceptOptions};

const SIZE: usize = 24 * 1024 * 1024;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:9002").await?;
    println!("Server started, listening on 127.0.0.1:9002");

    loop {
        let (stream, _) = listener.accept().await?;

        let future = async move {
            let mut read_buf = vec![0u8; SIZE];
            let mut write_buf = vec![0u8; SIZE];
            let mut fragments_buf = vec![0u8; SIZE];

            let mut websocketz = WebSocket::accept::<16>(
                AcceptOptions::default(),
                FromTokio::new(stream),
                StdRng::from_os_rng(),
                &mut read_buf,
                &mut write_buf,
                &mut fragments_buf,
            )
            .await?;

            loop {
                match next!(websocketz, |msg| {
                    let on_message = match msg {
                        Message::Text(payload) => OnMessage::Send(Message::Text(payload)),
                        Message::Binary(payload) => OnMessage::Send(Message::Binary(payload)),
                        _ => OnMessage::Noop(msg),
                    };

                    Ok::<_, Infallible>(on_message)
                }) {
                    None => {
                        break;
                    }

                    Some(Err(err)) => {
                        println!("Error reading message: {err}");

                        websocketz.send(Message::Close(None)).await?;

                        break;
                    }
                    _ => {}
                }
            }

            Ok::<(), Error<std::io::Error>>(())
        };

        tokio::spawn(async move {
            if let Err(err) = future.await {
                eprintln!("Error handling connection: {err}");
            }
        });
    }
}
