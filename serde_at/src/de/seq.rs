use serde::de;

use crate::de::{Deserializer, Error, Result};

#[allow(clippy::module_name_repetitions)]
pub struct SeqAccess<'a, 'b> {
    first: bool,
    de: &'a mut Deserializer<'b>,
}

impl<'a, 'b> SeqAccess<'a, 'b> {
    pub(crate) fn new(de: &'a mut Deserializer<'b>) -> Self {
        SeqAccess { de, first: true }
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
            Some(_) => {
                if self.first {
                    self.first = false;
                } else {
                    return Ok(None);
                }
            }
            None => {}
        };

        Ok(Some(seed.deserialize(&mut *self.de)?))
    }
}

pub struct SeqByteAccess<'a, 'b> {
    de: &'a mut Deserializer<'b>,
}

impl<'a, 'b> SeqByteAccess<'a, 'b> {
    pub(crate) fn new(de: &'a mut Deserializer<'b>) -> Self {
        SeqByteAccess { de }
    }
}

impl<'a, 'de> de::SeqAccess<'de> for SeqByteAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.de.parse_whitespace() {
            Some(b',') | None => {
                return Ok(None);
            }
            Some(_) => {}
        };

        Ok(Some(seed.deserialize(&mut *self.de)?))
    }
}
