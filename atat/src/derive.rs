use heapless::{consts, ArrayLength, String, Vec};

pub trait AtatLen {
    type Len: ArrayLength<u8>;
}

macro_rules! impl_length {
    ($type:ty,$len:ident) => {
        impl AtatLen for $type {
            type Len = consts::$len;
        }
    };
}

impl_length!(char, U1);
impl_length!(bool, U5);
impl_length!(isize, U19);
impl_length!(usize, U20);
impl_length!(u8, U3);
impl_length!(u16, U5);
impl_length!(u32, U10);
impl_length!(u64, U20);
impl_length!(u128, U39);
impl_length!(i8, U4);
impl_length!(i16, U6);
impl_length!(i32, U11);
impl_length!(i64, U20);
impl_length!(i128, U40);
impl_length!(f32, U42);
impl_length!(f64, U312);

impl<T: ArrayLength<u8>> AtatLen for String<T> {
    type Len = T;
}

impl<T: AtatLen> AtatLen for Option<T> {
    type Len = T::Len;
}

impl<T: ArrayLength<u8>> AtatLen for Vec<u8, T> {
    type Len = T;
}

// TODO: Replace above Vec<_> impl with below generic one, as soon as i figure
// out how to obtain the length from `ArrayLength` trait
//
// impl<T: AtatLen, L: ArrayLength<T>> AtatLen for Vec<T, L> {
//     type Len = <<T as AtatLen>::Len as core::ops::Mul<<L as typenum::Len>::Output>>::Output;
// }
