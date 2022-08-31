use crate::ser::{Error, Result, Serializer};
use serde::ser;

#[allow(clippy::module_name_repetitions)]
pub struct SerializeStruct<'a, 'b, const B: usize> {
    ser: &'a mut Serializer<'b, B>,
    first: bool,
}

impl<'a, 'b, const B: usize> SerializeStruct<'a, 'b, B> {
    pub(crate) fn new(ser: &'a mut Serializer<'b, B>) -> Self {
        SerializeStruct { ser, first: true }
    }
}

impl<'a, 'b, const B: usize> ser::SerializeStruct for SerializeStruct<'a, 'b, B> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
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
        self.ser
            .buf
            .extend_from_slice(self.ser.options.termination.as_bytes())?;
        Ok(())
    }
}
