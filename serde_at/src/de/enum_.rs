use serde::de;

use crate::de::{Deserializer, Error, Result};

pub(crate) struct VariantAccess<'a, 'b> {
    de: &'a mut Deserializer<'b>,
}

impl<'a, 'b> VariantAccess<'a, 'b> {
    pub(crate) fn new(de: &'a mut Deserializer<'b>) -> Self {
        VariantAccess { de }
    }
}

impl<'a, 'de> de::EnumAccess<'de> for VariantAccess<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(&mut *self.de)?;
        Ok((variant, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for VariantAccess<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self
            .de
            .parse_whitespace()
            .ok_or(Error::EofWhileParsingObject)?
            == b','
        {
            self.de.eat_char();
            self.de.parse_whitespace();
        }
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }
}
