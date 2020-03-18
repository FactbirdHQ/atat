use serde::ser;

use heapless::ArrayLength;

use crate::ser::{Error, Result, Serializer};

pub struct SerializeStruct<'a, 'b, B, C>
where
    B: ArrayLength<u8>,
    C: ArrayLength<u8>,
{
    ser: &'a mut Serializer<'b, B, C>,
    first: bool,
}

impl<'a, 'b, B, C> SerializeStruct<'a, 'b, B, C>
where
    B: ArrayLength<u8>,
    C: ArrayLength<u8>,
{
    pub(crate) fn new(ser: &'a mut Serializer<'b, B, C>) -> Self {
        SerializeStruct { ser, first: true }
    }
}

impl<'a, 'b, B, C> ser::SerializeStruct for SerializeStruct<'a, 'b, B, C>
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
        if self.first {
            if self.ser.options.value_sep {
                self.ser.buf.push(b'=')?;
            }
        } else {
            self.ser.buf.push(b',')?;
        }
        self.first = false;

        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        self.ser.buf.extend_from_slice(self.ser.options.termination.as_bytes())?;
        Ok(())
    }
}
