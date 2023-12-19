#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]
use atat_examples::common;

use atat::{
    asynch::{AtatClient, Client},
    AtatIngress, Config, DefaultDigester, Ingress, ResponseChannel, UrcChannel,
};
use embedded_io_adapters::tokio_1::FromTokio;
use std::process::exit;
use tokio_serial::SerialStream;

const INGRESS_BUF_SIZE: usize = 1024;
const URC_CAPACITY: usize = 128;
const URC_SUBSCRIBERS: usize = 3;

#[tokio::main]
async fn main() -> ! {
    env_logger::init();

    let (reader, writer) = SerialStream::pair().expect("Failed to create serial pair");

    static RES_CHANNEL: ResponseChannel<INGRESS_BUF_SIZE> = ResponseChannel::new();
    static URC_CHANNEL: UrcChannel<common::Urc, URC_CAPACITY, URC_SUBSCRIBERS> = UrcChannel::new();
    let ingress = Ingress::new(
        DefaultDigester::<common::Urc>::default(),
        &RES_CHANNEL,
        &URC_CHANNEL,
    );
    let buf = StaticCell::make_static!([0; 1024]);
    let mut client = Client::new(FromTokio::new(writer), &RES_CHANNEL, buf, Config::default());

    tokio::spawn(ingress_task(ingress, FromTokio::new(reader)));

    let mut state: u8 = 0;
    loop {
        // These will all timeout after 1 sec, as there is no response
        match state {
            0 => {
                client.send(&common::general::GetManufacturerId).await.ok();
            }
            1 => {
                client.send(&common::general::GetModelId).await.ok();
            }
            2 => {
                client.send(&common::general::GetSoftwareVersion).await.ok();
            }
            3 => {
                client.send(&common::general::GetWifiMac).await.ok();
            }
            _ => exit(0),
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        state += 1;
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
    mut reader: FromTokio<SerialStream>,
) -> ! {
    ingress.read_from(&mut reader).await
}
