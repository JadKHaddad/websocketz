use embedded_io_adapters::tokio_1::FromTokio;
use rand::{SeedableRng, rngs::StdRng};
use tokio::net::TcpStream;
use websocketz::{Message, Websockets, next};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("framez=trace")
        .init();

    let stream = TcpStream::connect("127.0.0.1:9002").await?;

    let read_buf = &mut [0u8; 8192 * 2];
    let write_buf = &mut [0u8; 8192 * 2];
    let fragments_buf = &mut [0u8; 8192 * 2];
    let rng = StdRng::from_os_rng();

    let mut websocketz = Websockets::connect::<16>(
        "/",
        &[],
        FromTokio::new(stream),
        rng,
        read_buf,
        write_buf,
        fragments_buf,
    )
    .await?;

    println!(
        "Number of framable bytes after handshake: {}",
        websocketz.framable()
    );

    websocketz.send(Message::Text("Hello, WebSocket!")).await?;

    websocketz
        .send_fragmented(Message::Text("Hello, Fragmented WebSocket!"), 4)
        .await?;

    while let Some(message) = next!(websocketz).transpose()? {
        println!("Received message: {message:?}");
    }

    Ok(())
}
