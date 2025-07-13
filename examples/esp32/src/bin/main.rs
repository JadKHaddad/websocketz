#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_executor::Spawner;
use embassy_net::{tcp::TcpSocket, Runner, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::{clock::CpuClock, rng::Trng, timer::timg::TimerGroup};
use esp_wifi::{
    wifi::{ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiState},
    EspWifiController,
};
use httparse::Header;
use log::{error, info};
use smoltcp::wire::DnsQueryType;
use websocketz::{next, options::ConnectOptions, Message, WebSocket};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.4.0

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer0 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    info!("Embassy initialized!");

    let mut trng = Trng::new(peripherals.RNG, peripherals.ADC1);

    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let wifi_init = &*mk_static!(
        EspWifiController<'static>,
        esp_wifi::init(timer1.timer0, trng.rng, peripherals.RADIO_CLK)
            .expect("Failed to initialize WIFI/BLE controller")
    );
    let (wifi_controller, interfaces) = esp_wifi::wifi::new(wifi_init, peripherals.WIFI)
        .expect("Failed to initialize WIFI controller");

    let wifi_interface = interfaces.sta;

    let config = embassy_net::Config::dhcpv4(Default::default());
    let seed = (trng.random() as u64) << 32 | trng.random() as u64;
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    spawner
        .spawn(connection(wifi_controller))
        .expect("Failed to spawn connection task");
    spawner
        .spawn(net_task(runner))
        .expect("Failed to spawn net task");

    loop {
        if stack.is_link_up() {
            break;
        }

        Timer::after(Duration::from_millis(500)).await;
    }

    info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            info!("Got IP: {}", config.address);
            break;
        }

        Timer::after(Duration::from_millis(500)).await;
    }

    let mut rx_buffer = [0; 1024];
    let mut tx_buffer = [0; 1024];

    let domain = "websockets.chilkat.io";
    let ip = *stack
        .dns_query(domain, DnsQueryType::A)
        .await
        .expect("DNS query failed")
        .first()
        .expect("No IP address returned");

    info!("Resolved {domain} to {ip}");

    loop {
        Timer::after(Duration::from_millis(1_000)).await;

        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

        socket.set_timeout(Some(embassy_time::Duration::from_secs(2)));

        info!("Connecting...");

        let r = socket.connect((ip, 80)).await;

        if let Err(e) = r {
            error!("Connect error: {:?}", e);

            continue;
        }

        info!("Connected!");

        let mut read_buf = [0u8; 1024];
        let mut write_buf = [0u8; 1024];
        let mut fragments_buf = [0u8; 1024];

        let mut websocketz = WebSocket::connect::<16>(
            ConnectOptions::default()
                .with_path_unchecked("/wsChilkatEcho.ashx")
                .with_headers(&[Header {
                    name: "Host",
                    value: domain.as_bytes(),
                }]),
            &mut socket,
            &mut trng,
            &mut read_buf,
            &mut write_buf,
            &mut fragments_buf,
        )
        .await
        .expect("Failed to create WebSocket connection");

        // split the WebSocket into read and write halves
        // let (mut websocketz_read, mut websocketz_write) = websocketz.split_with(|socket| socket.split());

        'ws: loop {
            websocketz
                .send(Message::Text("Hello, WebSocket!"))
                .await
                .expect("Failed to send message");

            match next!(websocketz) {
                None => {
                    info!("EOF");

                    break 'ws;
                }
                Some(Ok(msg)) => {
                    info!("Received message: {:?}", msg);
                }
                Some(Err(e)) => {
                    error!("Error receiving message: {:?}", e);

                    break 'ws;
                }
            }

            Timer::after(Duration::from_millis(1000)).await;
        }

        info!("Closing connection...");
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    info!("Device capabilities: {:?}", controller.capabilities());

    loop {
        if esp_wifi::wifi::wifi_state() == WifiState::StaConnected {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;

            Timer::after(Duration::from_millis(5000)).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(),
                ..Default::default()
            });

            controller.set_configuration(&client_config).unwrap();

            info!("Starting wifi");

            controller.start_async().await.unwrap();

            info!("Wifi started!");

            info!("Scan");

            let result = controller.scan_n_async(10).await.unwrap();

            for ap in result {
                info!("{:?}", ap);
            }
        }
        info!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => info!("Wifi connected!"),
            Err(err) => {
                error!("Failed to connect to wifi: {err:?}");

                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
