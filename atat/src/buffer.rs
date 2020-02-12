use heapless::{ArrayLength, String, Vec};

// Serial receive buffer
#[derive(Default)]
pub struct Buffer<N: ArrayLength<u8>> {
    pub buffer: String<N>,
}

impl<N> Buffer<N>
where
    N: ArrayLength<u8>,
{
    pub fn new() -> Buffer<N> {
        Buffer {
            buffer: String::new(),
        }
    }

    pub fn from(data: &[u8]) -> Buffer<N> {
        Buffer {
            buffer: String::from(core::str::from_utf8(&data).unwrap()),
        }
    }

    pub fn take<L: ArrayLength<u8>>(&mut self, index: usize) -> String<L> {
        let mut result = String::new();
        let mut return_string: String<L> = String::new();
        return_string
            .push_str(unsafe { self.buffer.get_unchecked(0..index) })
            .ok();
        result
            .push_str(unsafe { self.buffer.get_unchecked(index..self.buffer.len()) })
            .ok();
        self.buffer = result;
        return_string
    }

    pub fn remove_first(&mut self) {
        let mut result = String::new();
        if self.buffer.len() > 0 {
            result
                .push_str(unsafe { self.buffer.get_unchecked(1..self.buffer.len()) })
                .ok();
        }
        self.buffer = result;
    }

    pub fn remove_line<A: ArrayLength<u8>>(&mut self, line: &String<A>) {
        let mut result = String::new();
        let mut last_end = 0;
        if let Some((start, part)) = self.buffer.match_indices(line.as_str()).next() {
            result
                .push_str(unsafe { self.buffer.get_unchecked(last_end..start) })
                .ok();
            last_end = start + part.len();
        }
        result
            .push_str(unsafe { self.buffer.get_unchecked(last_end..self.buffer.len()) })
            .ok();
        self.buffer = result;
    }

    pub fn at_lines<S: ArrayLength<u8>, L: ArrayLength<String<S>>>(
        &self,
        term_char: char,
        format_char: char,
    ) -> Vec<String<S>, L> {
        self.buffer
            .split_terminator(term_char)
            .map(|l| l.trim_matches(|c: char| c == format_char))
            .map(String::from)
            .filter(|p| !p.is_empty())
            .collect()
    }

    pub fn push(&mut self, data: u8) -> Result<(), ()> {
        self.buffer.push(data as char)
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::tests::setup_log;
//     use heapless::consts;

//     #[test]
//     fn simple_at() {
//         setup_log();

//         let mut rx: Buffer<consts::U60> = Buffer::from(b"AT\r\n\r\nOK\r\n");
//         let lines: Vec<String<consts::U60>, consts::U8> = rx.at_lines('\n', '\r');

//         let full_response = lines
//             .iter()
//             .take_while(|&line| line.as_str() != "OK")
//             .inspect(|line| rx.remove_line(&line))
//             .filter(|p| !p.is_empty())
//             .collect::<Vec<_, consts::U8>>();

//         rx.remove_line::<consts::U2>(&String::from("OK"));

//         if rx.buffer.len() > 0 {
//             while rx.buffer.chars().nth(0) == Some('\n') || rx.buffer.chars().nth(0) == Some('\r') {
//                 rx.remove_first();
//             }
//         }

//         assert_eq!(lines.len(), 2);
//         assert_eq!(lines, ["AT", "OK"]);
//         assert_eq!(full_response.len(), 1);
//         assert_eq!(full_response, ["AT"]);
//         assert!(
//             rx.buffer.is_empty(),
//             "rx buffer still contains data: {:?}",
//             rx.buffer
//         );
//     }

//     #[test]
//     fn simple_at_no_newline() {
//         setup_log();

//         let mut rx: Buffer<consts::U60> = Buffer::from(b"AT\r\r\nOK\r\n");
//         let lines: Vec<String<consts::U60>, consts::U8> = rx.at_lines('\n', '\r');

//         let full_response = lines
//             .iter()
//             .take_while(|&line| line.as_str() != "OK")
//             .inspect(|line| rx.remove_line(&line))
//             .filter(|p| !p.is_empty())
//             .collect::<Vec<_, consts::U8>>();

//         rx.remove_line::<consts::U2>(&String::from("OK"));

//         if rx.buffer.len() > 0 {
//             while rx.buffer.chars().nth(0) == Some('\n') || rx.buffer.chars().nth(0) == Some('\r') {
//                 rx.remove_first();
//             }
//         }

//         assert_eq!(lines.len(), 2);
//         assert_eq!(lines, ["AT", "OK"]);
//         assert_eq!(full_response.len(), 1);
//         assert_eq!(full_response, ["AT"]);
//         assert!(
//             rx.buffer.is_empty(),
//             "rx buffer still contains data: {:?}",
//             rx.buffer
//         );
//     }
// }
