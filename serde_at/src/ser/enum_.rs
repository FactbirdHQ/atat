use serde::ser;

use heapless::ArrayLength;

use crate::ser::{Error, Result, Serializer};

pub struct SerializeTupleVariant<'a, 'b, B, C>
where
    B: ArrayLength<u8>,
    C: ArrayLength<u8>,
{
    ser: &'a mut Serializer<'b, B, C>,
    first: bool,
}

impl<'a, 'b, B, C> SerializeTupleVariant<'a, 'b, B, C>
where
    B: ArrayLength<u8>,
    C: ArrayLength<u8>,
{
    pub(crate) fn new(ser: &'a mut Serializer<'b, B, C>) -> Self {
        SerializeTupleVariant { ser, first: true }
    }
}


impl<'a, 'b, B, C> ser::SerializeTupleVariant for SerializeTupleVariant<'a, 'b, B, C>
where
    B: ArrayLength<u8>,
    C: ArrayLength<u8>,
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


pub struct SerializeStructVariant<'a, 'b, B, C>
where
    B: ArrayLength<u8>,
    C: ArrayLength<u8>,
{
    ser: &'a mut Serializer<'b, B, C>,
    first: bool,
}

impl<'a, 'b, B, C> SerializeStructVariant<'a, 'b, B, C>
where
    B: ArrayLength<u8>,
    C: ArrayLength<u8>,
{
    pub(crate) fn new(ser: &'a mut Serializer<'b, B, C>) -> Self {
        SerializeStructVariant { ser, first: true }
    }
}

impl<'a, 'b, B, C> ser::SerializeStructVariant for SerializeStructVariant<'a, 'b, B, C>
where
    B: ArrayLength<u8>,
    C: ArrayLength<u8>,
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
