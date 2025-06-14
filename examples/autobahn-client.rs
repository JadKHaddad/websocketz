use embedded_io_adapters::tokio_1::FromTokio;
use httparse::Header;
use rand::{SeedableRng, rngs::StdRng};
use tokio::net::TcpStream;
use websocketz::{
    CloseCode, CloseFrame, Message, Options, Websockets, WebsocketsRead, WebsocketsWrite, next,
};

async fn connect(path: &str) -> Result<TcpStream, Box<dyn std::error::Error>> {
    let path = format!("http://localhost:9001/{}", path);

    let stream = TcpStream::connect("localhost:9001").await?;

    let read_buf = &mut [0u8; 1024];
    let write_buf = &mut [0u8; 1024];
    let rng = StdRng::from_os_rng();

    let options = Options::new(
        &path,
        &[
            Header {
                name: "Host",
                value: b"localhost:9001",
            },
            Header {
                name: "Origin",
                value: b"http://localhost:9001",
            },
        ],
    );

    Ok(
        Websockets::handshake::<16>(FromTokio::new(stream), rng, read_buf, write_buf, options)
            .await
            .map_err(|_| "Handshake failed")?
            .into_inner(),
    )
}

async fn get_case_count() -> Result<u32, Box<dyn std::error::Error>> {
    let stream = connect("getCaseCount").await?;

    let read_buf = &mut [0u8; 1024];
    let write_buf = &mut [0u8; 1024];
    let fragments_buf = &mut [0u8; 1024];
    let rng = StdRng::from_os_rng();

    let mut websocketz = Websockets::client(
        FromTokio::new(stream),
        rng,
        read_buf,
        write_buf,
        fragments_buf,
    );

    let message = {
        let Message::Text(payload) = next!(websocketz)
            .transpose()?
            .ok_or("No message received")?
        else {
            return Err("Expected a text message".into());
        };
        payload.parse()?
    };

    websocketz
        .send(Message::Close(Some(CloseFrame::new(CloseCode::Normal, ""))))
        .await?;

    Ok(message)
}

const SIZE: usize = 24 * 1024 * 1024;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count = get_case_count().await?;

    for case in 1..=count {
        println!("Running case {case} of {count}");

        let stream = connect(&format!("runCase?case={}&agent=websocketz", case)).await?;
        let (read, write) = tokio::io::split(stream);

        let mut read_buf = vec![0u8; SIZE];
        let mut write_buf = vec![0u8; SIZE];
        let mut fragments_buf = vec![0u8; SIZE];
        let rng = StdRng::from_os_rng();

        let mut websocketz_read =
            WebsocketsRead::client(FromTokio::new(read), &mut read_buf, &mut fragments_buf);

        let mut websocketz_write =
            WebsocketsWrite::client(FromTokio::new(write), rng, &mut write_buf);

        loop {
            match next!(websocketz_read) {
                Some(Ok(msg)) => {
                    match msg {
                        Message::Text(payload) => {
                            websocketz_write.send(Message::Text(payload)).await?;
                        }
                        Message::Binary(payload) => {
                            // we can also fragment messages
                            websocketz_write
                                .send_fragmented(Message::Binary(payload), SIZE / 4)
                                .await?;
                        }
                        Message::Close(Some(frame)) => {
                            websocketz_write.send(Message::Close(Some(frame))).await?;

                            break;
                        }
                        Message::Close(None) => {
                            websocketz_write
                                .send(Message::Close(Some(CloseFrame::new(CloseCode::Normal, ""))))
                                .await?;

                            break;
                        }
                        Message::Ping(payload) => {
                            websocketz_write.send(Message::Pong(payload)).await?;
                        }
                        _ => {}
                    }
                }
                None => {
                    break;
                }
                Some(Err(err)) => {
                    println!("Error reading message: {}", err);

                    websocketz_write.send(Message::Close(None)).await?;

                    break;
                }
            }
        }
    }

    let stream = connect("updateReports?agent=websocketz").await?;

    let write_buf = &mut [0u8; 1024];
    let rng = StdRng::from_os_rng();

    let mut websocketz = WebsocketsWrite::client(FromTokio::new(stream), rng, write_buf);

    websocketz
        .send(Message::Close(Some(CloseFrame::new(CloseCode::Normal, ""))))
        .await?;

    Ok(())
}
