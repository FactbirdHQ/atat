use crate::ser::{Error, Result, Serializer};
use serde::ser;

pub struct SerializeTupleVariant<'a, 'b, const B: usize> {
    ser: &'a mut Serializer<'b, B>,
    first: bool,
}

impl<'a, 'b, const B: usize> SerializeTupleVariant<'a, 'b, B> {
    pub(crate) fn new(ser: &'a mut Serializer<'b, B>) -> Self {
        SerializeTupleVariant { ser, first: true }
    }
}

impl<'a, 'b, const B: usize> ser::SerializeTupleVariant for SerializeTupleVariant<'a, 'b, B> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
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

pub struct SerializeStructVariant<'a, 'b, const B: usize> {
    ser: &'a mut Serializer<'b, B>,
    first: bool,
}

impl<'a, 'b, const B: usize> SerializeStructVariant<'a, 'b, B> {
    pub(crate) fn new(ser: &'a mut Serializer<'b, B>) -> Self {
        SerializeStructVariant { ser, first: true }
    }
}

impl<'a, 'b, const B: usize> ser::SerializeStructVariant for SerializeStructVariant<'a, 'b, B> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
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
