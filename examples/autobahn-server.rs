use embedded_io_adapters::tokio_1::FromTokio;
use rand::{SeedableRng, rngs::StdRng};
use tokio::net::{TcpListener, TcpStream};
use websocketz::{CloseCode, CloseFrame, Message, Websockets, error::Error, next};

const SIZE: usize = 24 * 1024 * 1024;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:9002").await?;
    println!("Server started, listening on 127.0.0.1:9002");

    loop {
        let (stream, _) = listener.accept().await?;

        let future = async move {
            let split = |stream: FromTokio<TcpStream>| {
                let (read, write) = tokio::io::split(stream.into_inner());

                (FromTokio::new(read), FromTokio::new(write))
            };

            let mut read_buf = vec![0u8; SIZE];
            let mut write_buf = vec![0u8; SIZE];
            let mut fragments_buf = vec![0u8; SIZE];

            let websocketz = Websockets::accept::<16>(
                &[],
                FromTokio::new(stream),
                StdRng::from_os_rng(),
                &mut read_buf,
                &mut write_buf,
                &mut fragments_buf,
            )
            .await?;

            let (mut websocketz_read, mut websocketz_write) = websocketz.split_with(split);

            loop {
                match next!(websocketz_read) {
                    None => {
                        break;
                    }
                    Some(Ok(msg)) => match msg {
                        Message::Text(payload) => {
                            websocketz_write.send(Message::Text(payload)).await?;
                        }
                        Message::Binary(payload) => {
                            websocketz_write.send(Message::Binary(payload)).await?;
                        }
                        Message::Close(Some(frame)) => {
                            // TODO: remove and run the server test. we handle ControlFrameTooLarge in the codec.
                            if frame.reason().len() >= 124 {
                                break;
                            }

                            websocketz_write.send(Message::Close(Some(frame))).await?;

                            break;
                        }
                        Message::Close(None) => {
                            websocketz_write
                                .send(Message::Close(Some(CloseFrame::no_reason(
                                    CloseCode::Normal,
                                ))))
                                .await?;

                            break;
                        }
                        Message::Ping(payload) => {
                            websocketz_write.send(Message::Pong(payload)).await?;
                        }
                        Message::Pong(_) => {}
                    },
                    Some(Err(err)) => {
                        println!("Error reading message: {}", err);

                        websocketz_write.send(Message::Close(None)).await?;

                        break;
                    }
                }
            }

            Ok::<(), Error<std::io::Error>>(())
        };

        tokio::spawn(async move {
            if let Err(err) = future.await {
                eprintln!("Error handling connection: {}", err);
            }
        });
    }
}
