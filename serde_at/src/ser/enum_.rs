use crate::ser::{Error, Result, Serializer};
use serde::ser;

pub struct SerializeTupleVariant<'a, 'b> {
    ser: &'a mut Serializer<'b>,
    first: bool,
}

impl<'a, 'b> SerializeTupleVariant<'a, 'b> {
    pub(crate) fn new(ser: &'a mut Serializer<'b>) -> Self {
        SerializeTupleVariant { ser, first: true }
    }
}

impl<'a, 'b> ser::SerializeTupleVariant for SerializeTupleVariant<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        if !self.first {
            self.ser.push(b',')?;
        }
        self.first = false;

        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

pub struct SerializeStructVariant<'a, 'b> {
    ser: &'a mut Serializer<'b>,
    first: bool,
}

impl<'a, 'b> SerializeStructVariant<'a, 'b> {
    pub(crate) fn new(ser: &'a mut Serializer<'b>) -> Self {
        SerializeStructVariant { ser, first: true }
    }
}

impl<'a, 'b> ser::SerializeStructVariant for SerializeStructVariant<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        if !self.first {
            self.ser.push(b',')?;
        }
        self.first = false;

        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}
