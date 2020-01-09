use heapless::{ArrayLength, String};

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

    pub fn remove_line<A: ArrayLength<u8>>(&mut self, line: &String<A>) {
        let mut result = String::new();
        let mut last_end = 0;
        for (start, part) in self.buffer.match_indices(line.as_str()) {
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

    pub fn push(&mut self, data: u8) -> Result<(), ()> {
        self.buffer.push(data as char)
    }
}
