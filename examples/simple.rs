use embedded_io_adapters::tokio_1::FromTokio;
use httparse::Header;
use rand::{SeedableRng, rngs::StdRng};
use tokio::net::TcpStream;
use websocketz::{Message, Options, Websockets, next};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = TcpStream::connect("127.0.0.1:8080").await?;

    let read_buf = &mut [0u8; 8192 * 2];
    let write_buf = &mut [0u8; 8192 * 2];
    let fragments_buf = &mut [0u8; 8192 * 2];
    let rng = StdRng::from_os_rng();

    let options = Options::new(
        "/",
        &[
            Header {
                name: "Host",
                value: b"127.0.0.1:8080",
            },
            Header {
                name: "Origin",
                value: b"http://127.0.0.1:8080",
            },
        ],
    );

    let websocketz = Websockets::connect::<16>(
        FromTokio::new(stream),
        rng,
        read_buf,
        write_buf,
        fragments_buf,
        options,
    )
    .await
    .map_err(|_| "Handshake failed")?;

    let (mut websocketz_read, mut websocketz_write) = websocketz.split_with(|stream| {
        let (read, write) = tokio::io::split(stream.into_inner());
        (FromTokio::new(read), FromTokio::new(write))
    });

    websocketz_write
        .send(Message::Text("Hello, WebSocket!"))
        .await?;

    websocketz_write
        .send_fragmented(Message::Text("Hello, Fragmented WebSocket!"), 4)
        .await?;

    while let Some(message) = next!(websocketz_read).transpose()? {
        println!("Received message: {message:?}");
    }

    Ok(())
}
