use heapless::{String, Vec};
use crate::{MaxCommandLen, MaxResponseLines};

pub trait ATCommandInterface<R> {
    fn get_cmd(&self) -> String<MaxCommandLen>;
    fn parse_resp(&self, response_lines: &mut Vec<String<MaxCommandLen>, MaxResponseLines>) -> R;
    fn parse_unsolicited(response_line: &str) -> R;
}
