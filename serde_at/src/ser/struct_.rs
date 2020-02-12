use serde::ser;

use heapless::ArrayLength;

use crate::ser::{Error, Result, Serializer};
// use at_rs::ATATPosition;

pub struct SerializeStruct<'a, B, C>
where
    B: ArrayLength<u8>,
    C: ArrayLength<u8>,
{
    ser: &'a mut Serializer<B, C>,
    arg_n: usize,
}

impl<'a, B, C> SerializeStruct<'a, B, C>
where
    B: ArrayLength<u8>,
    C: ArrayLength<u8>,
{
    pub(crate) fn new(ser: &'a mut Serializer<B, C>) -> Self {
        SerializeStruct { ser, arg_n: 0 }
    }
}

impl<'a, B, C> ser::SerializeStruct for SerializeStruct<'a, B, C>
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
        if self.arg_n == 0 {
            self.ser.buf.push(b'=')?;
        } else {
            self.ser.buf.push(b',')?;
        }

        // if let Some((k, v)) = self.saved.pop() {

        // }
        value.serialize(&mut *self.ser)?;
        self.arg_n += 1;

        // let pos = value.get_pos(key);
        // let pos = 0;
        // if pos != self.arg_n {
        //     // Push the value for later processing
        //     self.saved.push((key, value));
        // } else {

        // }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        self.ser.buf.extend_from_slice(b"\r\n")?;
        Ok(())
    }
}
