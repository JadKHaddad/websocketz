use embedded_io_adapters::tokio_1::FromTokio;
use rand::{SeedableRng, rngs::StdRng};
use tokio::net::TcpStream;
use websocketz::{Message, Response, Websockets, next};

#[derive(Debug, thiserror::Error)]
#[error("Oh no!")]
struct CustomError {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = TcpStream::connect("127.0.0.1:9002").await?;

    let read_buf = &mut [0u8; 8192];
    let write_buf = &mut [0u8; 8192];
    let fragments_buf = &mut [0u8; 8192];
    let rng = StdRng::from_os_rng();

    let mut websocketz = Websockets::connect_with(
        "/",
        &[],
        FromTokio::new(stream),
        rng,
        read_buf,
        write_buf,
        fragments_buf,
        |response: &Response<'_, 16>| {
            // Fail the handshake if `Custom-Header: Custom-Value` header does not exist in the server response.

            response
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

    println!(
        "Number of framable bytes after handshake: {}",
        websocketz.framable()
    );

    websocketz.send(Message::Text("Hello, WebSocket!")).await?;

    while let Some(message) = next!(websocketz).transpose()? {
        println!("Received message: {message:?}");
    }

    Ok(())
}
