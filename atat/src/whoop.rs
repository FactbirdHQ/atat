use crate as atat;
use atat_derive::AtatEnum;
use atat::AtatLen;

#[derive(AtatEnum)]
enum MixedEnum<T> {
    SingleSimpleTupleLifetime(u8, T),
}

impl<T> AtatLen for MixedEnum<T>
where
    T: AtatLen,
    T::Len: core::ops::Add<typenum::U0>,
    <T::Len as core::ops::Add<typenum::U0>>::Output: heapless::ArrayLength<u8>,
{
    type Len = <<T as atat::AtatLen>::Len as core::ops::Add<heapless::consts::U0>>::Output;
}
