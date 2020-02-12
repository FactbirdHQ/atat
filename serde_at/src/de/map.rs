use serde::de;

use crate::de::{Deserializer, Error};

pub struct MapAccess<'a, 'b> {
    de: &'a mut Deserializer<'b>,
    first: bool,
}

impl<'a, 'b> MapAccess<'a, 'b> {
    pub(crate) fn new(de: &'a mut Deserializer<'b>) -> Self {
        MapAccess { de, first: true }
    }
}

impl<'a, 'de> de::MapAccess<'de> for MapAccess<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self
            .de
            .parse_whitespace()
            .ok_or(Error::EofWhileParsingObject)?
        {
            b',' if !self.first => {
                self.de.eat_char();
                self.de.parse_whitespace();
            }
            _ => {}
        }
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}
