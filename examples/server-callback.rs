use embedded_io_adapters::tokio_1::FromTokio;
use rand::{SeedableRng, rngs::StdRng};
use tokio::net::{TcpListener, TcpStream};
use websocketz::{Message, Request, Websockets, next};

#[derive(Debug, thiserror::Error)]
#[error("Oh no!")]
struct CustomError {}

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

            let websocketz = Websockets::accept_with(
                &[],
                FromTokio::new(stream),
                StdRng::from_os_rng(),
                &mut read_buf,
                &mut write_buf,
                &mut fragments_buf,
                |request: &Request<'_, 16>| {
                    // Fail the handshake if `Custom-Header: Custom-Value` header does not exist in the client request.

                    request
                        .headers()
                        .iter()
                        .find(|h| h.name.eq_ignore_ascii_case("Custom-Header"))
                        .and_then(|h| core::str::from_utf8(h.value).ok())
                        .filter(|v| v.eq_ignore_ascii_case("Custom-Value"))
                        .map(|_| ())
                        .ok_or(CustomError {})
                },
            )
            .await?;

            let (mut websocketz_read, mut websocketz_write) = websocketz.split_with(split);

            while let Some(_) = next!(websocketz_read).transpose()? {
                websocketz_write.send(Message::Text("Ok Boomer 👍")).await?
            }

            Ok::<(), Box<dyn std::error::Error>>(())
        };

        tokio::spawn(async move {
            if let Err(err) = future.await {
                eprintln!("Error handling connection: {}", err);
            }
        });
    }
}
