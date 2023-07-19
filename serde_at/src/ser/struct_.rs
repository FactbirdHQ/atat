use crate::ser::{Error, Result, Serializer};
use serde::ser;

#[allow(clippy::module_name_repetitions)]
pub struct SerializeStruct<'a, 'b> {
    ser: &'a mut Serializer<'b>,
    nested: bool,
    first: bool,
}

impl<'a, 'b> SerializeStruct<'a, 'b> {
    pub(crate) fn new(ser: &'a mut Serializer<'b>, nested: bool) -> Self {
        SerializeStruct {
            ser,
            nested,
            first: true,
        }
    }
}

impl<'a, 'b> ser::SerializeStruct for SerializeStruct<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        if self.first {
            if !self.nested && self.ser.options.value_sep {
                self.ser.push(b'=')?;
            }
        } else {
            self.ser.push(b',')?;
        }
        self.first = false;

        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        if !self.nested {
            self.ser
                .extend_from_slice(self.ser.options.termination.as_bytes())?;
        }
        Ok(())
    }
}
