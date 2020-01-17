use heapless::{
    spsc::{Consumer, Producer},
    ArrayLength,
};

use embedded_hal::timer::CountDown;

use crate::error::Error;
use crate::traits::{ATCommandInterface, ATInterface, ATRequestType};
use crate::Response;

type ReqProducer<Req, N> = Producer<'static, Req, N, u8>;
type ResConsumer<Res, N> = Consumer<'static, Result<Res, Error>, N, u8>;

pub struct ATClient<T, Req, ReqQueueLen, ResQueueLen>
where
    Req: ATRequestType,
    Req::Command: ATCommandInterface,
    T: CountDown,
    ReqQueueLen: ArrayLength<Req>,
    ResQueueLen: ArrayLength<Result<Response<Req>, Error>>,
{
    req_p: ReqProducer<Req, ReqQueueLen>,
    res_c: ResConsumer<Response<Req>, ResQueueLen>,
    default_timeout: T::Time,
    timer: T,
}

impl<T, Req, ReqQueueLen, ResQueueLen> ATClient<T, Req, ReqQueueLen, ResQueueLen>
where
    Req: ATRequestType,
    Req::Command: ATCommandInterface,
    T: CountDown,
    T::Time: Copy,
    ReqQueueLen: ArrayLength<Req>,
    ResQueueLen: ArrayLength<Result<Response<Req>, Error>>,
{
    pub fn new(
        queues: (
            ReqProducer<Req, ReqQueueLen>,
            ResConsumer<Response<Req>, ResQueueLen>,
        ),
        default_timeout: T::Time,
        timer: T,
    ) -> Self {
        let (req_p, res_c) = queues;
        Self {
            req_p,
            res_c,
            default_timeout,
            timer,
        }
    }

    pub fn release(
        self,
    ) -> (
        ReqProducer<Req, ReqQueueLen>,
        ResConsumer<Response<Req>, ResQueueLen>,
    ) {
        (self.req_p, self.res_c)
    }
}

impl<T, Req, ReqQueueLen, ResQueueLen> ATInterface<T, Req, Response<Req>>
    for ATClient<T, Req, ReqQueueLen, ResQueueLen>
where
    Req: ATRequestType,
    Req::Command: ATCommandInterface,
    T: CountDown,
    T::Time: Copy,
    Response<Req>: core::fmt::Debug,
    ReqQueueLen: ArrayLength<Req>,
    ResQueueLen: ArrayLength<Result<Response<Req>, Error>>,
{
    fn send(&mut self, req: Req) -> Result<Response<Req>, Error> {
        self.send_timeout(req, self.default_timeout)
    }

    fn send_timeout(&mut self, req: Req, timeout: T::Time) -> Result<Response<Req>, Error> {
        match self.req_p.enqueue(req) {
            Ok(_) => self.wait_response_timeout(timeout),
            Err(_e) => Err(Error::Overflow),
        }
    }

    fn wait_response_timeout(&mut self, timeout: T::Time) -> Result<Response<Req>, Error> {
        self.timer.start(timeout);
        loop {
            if let Some(result) = self.res_c.dequeue() {
                // self.timer.cancel().map_err(|_| Error::Timeout)?;
                return result.map_err(|_e| Error::InvalidResponse);
            }
            if self.timer.wait().is_ok() {
                return Err(Error::Timeout);
            }
        }
    }

    fn wait_response(&mut self) -> Result<Response<Req>, Error> {
        self.wait_response_timeout(self.default_timeout)
    }

    fn peek_response(&mut self) -> &Result<Response<Req>, Error> {
        self.timer.start(self.default_timeout);
        loop {
            if let Some(result) = self.res_c.peek() {
                return result;
            }
            if self.timer.wait().is_ok() {
                return &Err(Error::Timeout);
            }
        }
    }
}
