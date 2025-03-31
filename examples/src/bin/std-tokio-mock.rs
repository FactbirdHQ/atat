use atat_examples::common;

use atat::{
    asynch::{AtatClient, Client},
    AtatIngress, Config, DefaultDigester, Ingress, ResponseSlot, UrcChannel,
};
use embedded_io_adapters::tokio_1::FromTokio;
use static_cell::StaticCell;
use std::process::exit;
use tokio::io::{AsyncReadExt, DuplexStream};

const INGRESS_BUF_SIZE: usize = 1024;
const URC_CAPACITY: usize = 128;
const URC_SUBSCRIBERS: usize = 1;

static INGRESS_BUF: StaticCell<[u8; INGRESS_BUF_SIZE]> = StaticCell::new();
static RES_SLOT: ResponseSlot<INGRESS_BUF_SIZE> = ResponseSlot::new();
static URC_CHANNEL: UrcChannel<common::Urc, URC_CAPACITY, URC_SUBSCRIBERS> = UrcChannel::new();

// Responses: Trigger
#[allow(dead_code)]
const RESPONSE_ERROR: &str = "\r\nERROR\r\n";
#[allow(dead_code)]
const RESPONSE_CME_ERROR: &str = "\r\n+CME ERROR: 122\r\n";

#[tokio::main]
async fn main() -> ! {
    env_logger::init();

    let (host, device) = tokio::io::duplex(1024);

    let (host_rx, host_tx) = tokio::io::split(host);
    let (device_rx, device_tx) = tokio::io::split(device);

    let ingress = Ingress::new(
        DefaultDigester::<common::Urc>::default(),
        INGRESS_BUF.init([0; INGRESS_BUF_SIZE]),
        &RES_SLOT,
        &URC_CHANNEL,
    );

    tokio::spawn(ingress_task(ingress, host_rx));
    tokio::spawn(device_task(
        FromTokio::new(device_rx),
        FromTokio::new(device_tx),
        RESPONSE_CME_ERROR.to_string(),
    ));

    static BUF: StaticCell<[u8; 1024]> = StaticCell::new();
    let buf = BUF.init([0; 1024]);
    let mut client = Client::new(FromTokio::new(host_tx), &RES_SLOT, buf, Config::default());

    let response = client.send(&common::general::GetManufacturerId).await;

    match response {
        Ok(_) => {
            log::info!("Response: OK");
        }
        Err(e) => {
            log::error!("Error: {:?}", e);
        }
    }

    exit(0);
}

async fn device_task(
    mut reader: impl embedded_io_async::Read,
    mut writer: impl embedded_io_async::Write,
    response: String,
) -> ! {
    let mut buf = [0; 1024];
    loop {
        let n = reader.read(&mut buf).await.unwrap();
        let received = core::str::from_utf8(&buf[..n]).unwrap();

        log::debug!("Received from host: {:?}", received);

        for byte in response.as_bytes() {
            writer.write(&[*byte]).await.unwrap();
        }
    }
}

async fn ingress_task<'a>(
    mut ingress: Ingress<
        'a,
        DefaultDigester<common::Urc>,
        common::Urc,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
        URC_SUBSCRIBERS,
    >,
    mut read: tokio::io::ReadHalf<DuplexStream>,
) -> ! {
    let mut buf = [0; 1024];

    while let Ok(n) = read.read(&mut buf).await {
        let received = core::str::from_utf8(&buf[..n]).unwrap();
        log::debug!("Received from device: {:?}", received);

        ingress
            .try_write(&buf[..n])
            .expect("Failed to write to ingress");
    }

    panic!("Failed to read data");
}
