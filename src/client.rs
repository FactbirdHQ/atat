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

#[derive(PartialEq)]
enum State {
    Idle,
    AwaitingResponse,
}

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
    state: State,
    timer: T,
}

impl<T, Req, ReqQueueLen, ResQueueLen> ATClient<T, Req, ReqQueueLen, ResQueueLen>
where
    Req: ATRequestType,
    Req::Command: ATCommandInterface,
    T: CountDown,
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
            state: State::Idle,
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
    ReqQueueLen: ArrayLength<Req>,
    ResQueueLen: ArrayLength<Result<Response<Req>, Error>>,
{
    fn send(&mut self, req: Req) -> nb::Result<Response<Req>, Error> {
        self.send_timeout(req, self.default_timeout)
    }

    fn send_timeout(&mut self, req: Req, timeout: T::Time) -> nb::Result<Response<Req>, Error> {
        if self.state == State::Idle {
            match self.req_p.enqueue(req) {
                Ok(_) => {
                    self.timer.start(timeout);
                    self.state = State::AwaitingResponse;
                    self.wait_response()
                }
                Err(_e) => Err(nb::Error::Other(Error::Overflow)),
            }
        } else {
            self.wait_response()
        }
    }

    fn wait_response(&mut self) -> nb::Result<Response<Req>, Error> {
        if let Some(result) = self.res_c.dequeue() {
            self.state = State::Idle;
            return result.map_err(|_e| nb::Error::Other(Error::InvalidResponse));
        }
        if self.timer.wait().is_ok() {
            self.state = State::Idle;
            return Err(nb::Error::Other(Error::Timeout));
        }
        Err(nb::Error::WouldBlock)
    }

    // fn peek_response(&mut self) -> &nb::Result<Response<Req>, Error> {
    //     self.timer.start(self.default_timeout);
    //     if let Some(result) = self.res_c.peek() {
    //         return result.map_err(|e| nb::Error::Other(e));
    //     }
    //     if self.timer.wait().is_ok() {
    //         return &Err(nb::Error::Other(Error::Timeout));
    //     }
    //     &Err(nb::Error::WouldBlock)
    // }
}
