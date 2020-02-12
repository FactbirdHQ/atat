use crate::error::{NBResult, Result};
use heapless::{ArrayLength, String};

pub trait ATATResp {}

pub trait ATATCmd {
    type Response: ATATResp;
    type CommandLen: ArrayLength<u8>;

    fn as_str(&self) -> String<Self::CommandLen>;

    fn parse(&self, resp: &str) -> Result<Self::Response>;

    fn can_abort(&self) -> bool {
        false
    }

    fn max_timeout_ms(&self) -> u32 {
        1000
    }
}

pub trait ATATInterface {
    fn send<A: ATATCmd>(&mut self, cmd: &A) -> NBResult<A::Response>;

    fn check_response<A: ATATCmd>(&mut self, cmd: &A) -> NBResult<A::Response>;
}
