// Size of buffer for received serial data
const RX_SZ: usize = 128;

const CARRIAGE_RETURN: u8 = 13;
const NEW_LINE: u8 = 10;
const OK: [u8; 4] = [b'O', b'K', CARRIAGE_RETURN, NEW_LINE];
const ERROR: [u8; 7] = [b'E', b'R', b'R', b'O', b'R', CARRIAGE_RETURN, NEW_LINE];

// Serial receive buffer
pub struct Buffer {
    pub index: usize,
    pub buffer: [u8; RX_SZ],
}

impl Buffer {
    pub const fn new() -> Buffer {
        Buffer {
            index: 0,
            buffer: [0; RX_SZ],
        }
    }

    pub fn from(buffer: &[u8]) -> Buffer {
        let mut arr: [u8; RX_SZ] = [0; RX_SZ];
        arr[0..buffer.len()].copy_from_slice(buffer);
        Buffer {
            index: buffer.len(),
            buffer: arr,
        }
    }

    fn trim_newline(&mut self) {
        while self.index > 1
            && (self.buffer[self.index - 1] == CARRIAGE_RETURN
                || self.buffer[self.index - 1] == NEW_LINE)
        {
            self.index -= 1;
            self.buffer[self.index] = 0;
        }
    }

    pub fn push(&mut self, data: u8) -> Result<(), ()> {
        if self.index < RX_SZ {
            self.buffer[self.index] = data;
            self.index += 1;
            return Ok(());
        }
        Err(())
    }

    pub fn read(&mut self) -> Option<Buffer> {
        if self.index > 0 {
            let tmp = self.index;
            self.index = 0;
            Some(Buffer {
                index: tmp,
                buffer: self.buffer,
            })
        } else {
            None
        }
    }

    pub fn split_line(&mut self) -> Option<(Buffer, Buffer)> {
        if let Some(mut index) = self
            .buffer
            .iter()
            .position(|&x| x == CARRIAGE_RETURN || x == NEW_LINE)
        {
            // Make sure to take the last char as well in a new line,
            // and handle lines ending in both \r\n & \n\r for good measure.
            if self.buffer[index + 1] == CARRIAGE_RETURN || self.buffer[index + 1] == NEW_LINE {
                index += 1;
            }

            let mut line = Buffer::from(&self.buffer[0..=index]);
            let mut remainder = Buffer::from(&self.buffer[index + 1..self.index]);
            line.trim_newline();
            remainder.trim_newline();
            Some((line, remainder))
        } else {
            None
        }
    }

    pub fn split_response(&mut self) -> Option<(Buffer, Buffer)> {
        // Skip the iterator until we reach an O or an E
        let iter = self
            .buffer
            .iter()
            .enumerate()
            .skip_while(|(_, x)| **x != OK[0] && **x != ERROR[0])
            .skip(1);

        let mut i = 1;
        let mut index: Option<usize> = None;

        for (ind, c) in iter {
            if *c == 0 {
                break;
            }

            if i < OK.len() && *c == OK[i] {
                if i == OK.len() - 1 {
                    // A complete 'OK' is found
                    index = Some(ind);
                }
            } else if i < ERROR.len() && *c == ERROR[i] {
                if i == ERROR.len() - 1 {
                    // A complete 'ERROR' is found
                    index = Some(ind);
                }
            } else {
                // Start searching for 'O' or 'E' again
                i = 0;
            }
            i += 1;
        }

        if let Some(ind) = index {
            let mut response = Buffer::from(&self.buffer[0..=ind]);
            let mut remainder = Buffer::from(&self.buffer[ind + 1..self.index]);
            response.trim_newline();
            remainder.trim_newline();
            Some((response, remainder))
        } else {
            // No 'OK' or 'ERROR' found
            None
        }
    }
}
