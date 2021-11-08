use atat::{
    atat_derive::{AtatCmd, AtatResp},
    AtatClient, Client, ComQueue, Config, InternalError, Mode, ResponseHeader,
};
use bbqueue::{framed::FrameProducer, BBBuffer};
use criterion::{criterion_group, criterion_main, Criterion};
use embedded_hal::{serial, timer::CountDown};
use heapless::spsc::Queue;

struct TxMock;

impl serial::Write<u8> for TxMock {
    type Error = ();

    fn try_write(&mut self, _c: u8) -> nb::Result<(), Self::Error> {
        Ok(())
    }

    fn try_flush(&mut self) -> nb::Result<(), Self::Error> {
        Ok(())
    }
}

struct CdMock;

impl CountDown for CdMock {
    type Error = core::convert::Infallible;
    type Time = u32;
    fn try_start<T>(&mut self, _count: T) -> Result<(), Self::Error>
    where
        T: Into<Self::Time>,
    {
        Ok(())
    }
    fn try_wait(&mut self) -> nb::Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Clone, AtatCmd)]
#[at_cmd("+CUN", TestResponse)]
pub struct TestCmd;

#[derive(Clone, AtatResp, PartialEq, Debug)]
pub struct TestResponse {
    #[at_arg(position = 0)]
    pub socket: u8,
    #[at_arg(position = 1)]
    pub length: usize,
    #[at_arg(position = 2)]
    pub data: heapless::String<4096>,
}

const TEST_RX_BUF_LEN: usize = 4096;
const TEST_RES_CAPACITY: usize = 3 * TEST_RX_BUF_LEN;
const TEST_URC_CAPACITY: usize = 3 * TEST_RX_BUF_LEN;

pub fn enqueue_res(
    producer: &mut FrameProducer<'static, TEST_RES_CAPACITY>,
    res: Result<heapless::Vec<u8, TEST_RX_BUF_LEN>, InternalError>,
) {
    let (header, bytes) = ResponseHeader::as_bytes(&res);

    if let Ok(mut grant) = producer.grant(bytes.len() + header.len()) {
        grant[0..header.len()].copy_from_slice(&header);
        grant[header.len()..header.len() + bytes.len()].copy_from_slice(bytes);
        grant.commit(bytes.len() + header.len());
    }
}

pub fn client(c: &mut Criterion) {
    static mut RES_Q: BBBuffer<TEST_RES_CAPACITY> = BBBuffer::new();
    let (mut res_p, res_c) = unsafe { RES_Q.try_split_framed().unwrap() };

    static mut URC_Q: BBBuffer<TEST_URC_CAPACITY> = BBBuffer::new();
    let (_urc_p, urc_c) = unsafe { URC_Q.try_split_framed().unwrap() };

    static mut COM_Q: ComQueue = Queue::new();
    let (com_p, _com_c) = unsafe { COM_Q.split() };
    let resp = heapless::Vec::from_slice(b"+CUN: 0,4096,\"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam diam ipsum, consectetur at dictum sit amet, lacinia quis mi. Aliquam auctor enim lectus. Orci varius natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Suspendisse volutpat faucibus erat, vel aliquam ligula. Vivamus rutrum mollis turpis, in consequat massa dictum ut. Sed sed dictum libero. Morbi lobortis eget ante ac ultricies. Proin ornare elit efficitur justo faucibus pulvinar. In hac habitasse platea dictumst. Pellentesque ut nisi eu velit rutrum egestas. Aliquam nunc nunc, tristique hendrerit purus quis, congue luctus nibh. In venenatis fringilla augue, a cursus orci efficitur a. Vivamus vel consectetur leo, tristique mattis ipsum. Suspendisse sollicitudin felis id velit lobortis tempor. Proin dui dolor, iaculis sed euismod id, facilisis in dui. Morbi commodo, odio nec cursus hendrerit, ante enim accumsan massa, sed iaculis orci augue sed lacus. In vestibulum id augue a malesuada. In sit amet euismod risus. Mauris varius nibh in purus varius fringilla. Nunc in sagittis sem. Vivamus laoreet lectus vulputate euismod dignissim. Pellentesque non eleifend leo, sit amet molestie sapien. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae; Duis faucibus dapibus mauris, ut interdum metus rhoncus non. Sed in nisl ac sem vulputate feugiat. Proin sem tortor, pulvinar eu lorem vel, auctor dapibus dolor. Phasellus ac mollis ex, eget convallis mi. Morbi ornare dapibus nulla nec imperdiet. Cras porta ultrices auctor. Duis tempor orci ante, id hendrerit elit semper et. Phasellus lacinia justo ac ex fermentum pellentesque. Aliquam volutpat sit amet quam eget dictum. Nunc eu tristique quam, vel cursus nunc. Fusce quis lacus eget ipsum ultrices sollicitudin nec et neque. Nullam non congue risus. Praesent blandit, lorem at faucibus rutrum, felis augue aliquet ipsum, ac sodales erat mauris quis nisi. Duis nibh lectus, sagittis at erat vel, dictum consectetur neque. Aliquam erat volutpat. Donec aliquam vulputate sem, id faucibus lacus rutrum in. Ut interdum arcu turpis. Etiam convallis laoreet faucibus. Donec sit amet augue non lectus ornare efficitur volutpat at lectus. Donec id ultrices orci. Ut ultricies risus dapibus, scelerisque neque sit amet, vulputate nisl. Sed tempus blandit turpis id vestibulum. Aenean bibendum erat lacinia tellus ultrices consequat. Vivamus condimentum quam pulvinar dignissim malesuada. Suspendisse quis vestibulum risus, quis scelerisque odio. Proin consequat lorem eget orci rutrum, suscipit hendrerit dui lacinia. Maecenas et facilisis nisi, a volutpat lorem. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Aenean orci enim, commodo sit amet mattis non, posuere vitae ante. Vivamus nec vestibulum ante. Mauris id lectus consequat, tempus ipsum sed, laoreet eros. Curabitur sit amet vestibulum arcu. Suspendisse sagittis massa ante, id sodales tellus euismod ut. Donec faucibus lacus vel sapien auctor, eu tincidunt augue eleifend. Etiam aliquam, purus ut feugiat tincidunt, sem arcu vulputate ipsum, vitae laoreet dolor leo et eros. Quisque eget neque ex. In venenatis, eros et facilisis ultricies, tellus libero rhoncus diam, sed hendrerit nisl nunc eu turpis. Donec dictum, enim at laoreet scelerisque, metus risus vulputate metus, non sagittis erat tellus ac lectus. Praesent id massa risus. Sed pharetra pharetra dui vel ullamcorper. Sed diam arcu, consectetur sodales pretium sed, egestas ac massa. Vestibulum turpis leo, elementum ac vestibulum at, ultrices sed arcu. Nunc sagittis sit amet diam ut porta. Maecenas posuere orci sed risus pellentesque, vel molestie massa accumsan. Cras rhoncus sed eros et consectetur. Suspendisse nulla elit, efficitur ac urna id, viverra dictum tellus. Nunc semper et lorem eget dignissim. Morbi nec elit at ipsum venenatis lobortis. Nam risus ex, laoreet et dui tristique, facilisis sodales ex. Donec id sem lacus. Vestibulum egestas mattis fringilla. Vivamus tristique cursus quam, at interdum mi consectetur sit amet.\"").unwrap();

    let mut client = Client::new(
        TxMock,
        res_c,
        urc_c,
        com_p,
        CdMock,
        Config::new(Mode::Blocking),
    );

    c.bench_function("client", |b| {
        b.iter(|| {
            client.reset();
            enqueue_res(&mut res_p, Ok(resp.clone()));
            nb::block!(client.send(&TestCmd))
        })
    });
}

criterion_group!(benches, client);

criterion_main!(benches);
