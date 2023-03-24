#![feature(async_fn_in_trait)]

mod common;

use std::process::exit;

use atat::{
    asynch::AtatClient, digest::ParseError, AtatIngress, Buffers, Config, DefaultDigester, Ingress,
    Parser,
};
use embedded_io::adapters::FromTokio;
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};

#[tokio::main]
async fn main() -> ! {
    env_logger::init();

    static BUFFERS: Buffers<256, 1024, 1024> = Buffers::<256, 1024, 1024>::new();

    let (reader, writer) =
        tokio_serial::SerialStream::pair().expect("Failed to create serial pair");

    let (ingress, mut client) = BUFFERS.split(
        FromTokio::new(writer),
        DefaultDigester::<common::Urc>::default(),
        Config::default(),
    );

    tokio::spawn(ingress_task(ingress, FromTokio::new(reader)));

    let mut state: u8 = 0;
    loop {
        // These will all timeout after 1 sec, as there is no response
        match state {
            0 => {
                client.send(&common::general::GetManufacturerId).await;
            }
            1 => {
                client.send(&common::general::GetModelId).await;
            }
            2 => {
                client.send(&common::general::GetSoftwareVersion).await;
            }
            3 => {
                client.send(&common::general::GetWifiMac).await;
            }
            _ => exit(0),
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        state += 1;
    }
}

async fn ingress_task<'a>(
    mut ingress: Ingress<'a, DefaultDigester<common::Urc>, 256, 1024, 1024>,
    mut reader: FromTokio<SerialStream>,
) -> ! {
    ingress.read_from(&mut reader).await
}
