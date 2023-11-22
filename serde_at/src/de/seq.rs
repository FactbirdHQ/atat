use serde::de;

use crate::de::{Deserializer, Error, Result};

#[allow(clippy::module_name_repetitions)]
pub struct SeqAccess<'a, 'b> {
    first: bool,
    count: usize,
    len: Option<usize>,
    de: &'a mut Deserializer<'b>,
}

impl<'a, 'b> SeqAccess<'a, 'b> {
    pub(crate) fn new(de: &'a mut Deserializer<'b>) -> Self {
        let len = de.struct_size_hint();
        SeqAccess {
            de,
            first: true,
            len,
            count: 0,
        }
    }
}

impl<'a, 'de> de::SeqAccess<'de> for SeqAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.de.parse_whitespace() {
            Some(b',') => {
                self.de.eat_char();
                self.de
                    .parse_whitespace()
                    .ok_or(Error::EofWhileParsingValue)?;
            }
            Some(c) => {
                if self.first {
                    self.first = false;
                } else if c != b'+' {
                    if let Some(len) = self.len {
                        if self.count == len - 1 {
                            self.de.set_is_trailing_parsing();
                        } else {
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    }
                }
            }
            None => {
                // No more characters!
                // Fall-through to deserialize any `Option<..>` to `None`
            }
        };

        match seed.deserialize(&mut *self.de) {
            // Misuse EofWhileParsingObject here to indicate finished object in vec cases.
            // See matching TODO in `de::mod`..
            Err(Error::EofWhileParsingObject) => {
                self.count += 1;
                Ok(None)
            }
            Err(e) => Err(e),
            Ok(v) => {
                self.count += 1;
                Ok(Some(v))
            }
        }
    }
}
