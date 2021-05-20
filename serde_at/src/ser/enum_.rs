use crate::ser::{Error, Result, Serializer};
use serde::ser;

pub struct SerializeTupleVariant<'a, 'b, const B: usize, const C: usize> {
    ser: &'a mut Serializer<'b, B, C>,
    first: bool,
}

impl<'a, 'b, const B: usize, const C: usize> SerializeTupleVariant<'a, 'b, B, C> {
    pub(crate) fn new(ser: &'a mut Serializer<'b, B, C>) -> Self {
        SerializeTupleVariant { ser, first: true }
    }
}

impl<'a, 'b, const B: usize, const C: usize> ser::SerializeTupleVariant
    for SerializeTupleVariant<'a, 'b, B, C>
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        if !self.first {
            self.ser.buf.push(b',')?;
        }
        self.first = false;

        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

pub struct SerializeStructVariant<'a, 'b, const B: usize, const C: usize> {
    ser: &'a mut Serializer<'b, B, C>,
    first: bool,
}

impl<'a, 'b, const B: usize, const C: usize> SerializeStructVariant<'a, 'b, B, C> {
    pub(crate) fn new(ser: &'a mut Serializer<'b, B, C>) -> Self {
        SerializeStructVariant { ser, first: true }
    }
}

impl<'a, 'b, const B: usize, const C: usize> ser::SerializeStructVariant
    for SerializeStructVariant<'a, 'b, B, C>
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        if !self.first {
            self.ser.buf.push(b',')?;
        }
        self.first = false;

        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}
