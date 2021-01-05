use core::ops::Mul;
use heapless::{ArrayLength, String, Vec};
use typenum::Unsigned;

/// Trait used by [`atat_derive`] to estimate lengths of the serialized commands, at compile time.
///
/// [`atat_derive`]: https://crates.io/crates/atat_derive
pub trait AtatLen {
    type Len: ArrayLength<u8>;
}

macro_rules! impl_length {
    ($type:ty, $len:ident) => {
        #[allow(clippy::use_self)]
        impl AtatLen for $type {
            type Len = heapless::consts::$len;
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

impl<T: AtatLen> AtatLen for &T {
    type Len = T::Len;
}

impl<T, L> AtatLen for Vec<T, L>
where
    T: AtatLen,
    L: ArrayLength<T> + Unsigned + Mul<<T as AtatLen>::Len>,
    <L as Mul<<T as AtatLen>::Len>>::Output: ArrayLength<u8>,
{
    type Len = <L as Mul<<T as AtatLen>::Len>>::Output;
}

#[cfg(test)]
mod tests {
    use crate as atat;
    use atat::{derive::AtatLen, AtatCmd};
    use atat_derive::{AtatCmd, AtatEnum, AtatResp};
    use heapless::{consts, String, Vec};
    use serde_at::{from_str, to_string, SerializeOptions};
    use typenum::marker_traits::Unsigned;

    #[derive(Debug, PartialEq, AtatResp)]
    struct NoResponse {}

    #[derive(Debug, PartialEq, AtatEnum)]
    enum SimpleEnum {
        #[at_arg(default, value = 0)]
        A,
        #[at_arg(value = 1)]
        B,
        #[at_arg(value = 2)]
        C,
        #[at_arg(value = 3)]
        D,
    }
    #[derive(Debug, PartialEq, AtatEnum)]
    #[at_enum(u32)]
    enum SimpleEnumU32 {
        #[at_arg(default)]
        A,
        B,
        C,
        D,
    }

    #[derive(Debug, PartialEq, AtatEnum)]
    enum MixedEnum<'a> {
        #[at_arg(value = 0)]
        UnitVariant,
        #[at_arg(value = 1)]
        SingleSimpleTuple(u8),
        #[at_arg(default, value = 2)]
        AdvancedTuple(u8, String<consts::U10>, i64, SimpleEnumU32),
        #[at_arg(value = 3)]
        SingleSimpleStruct { x: u8 },
        #[at_arg(value = 4)]
        AdvancedStruct {
            a: u8,
            b: String<consts::U10>,
            c: i64,
            d: SimpleEnum,
        },
        #[at_arg(value = 6)]
        SingleSimpleTupleLifetime(#[at_arg(len = 10)] &'a str),
    }

    #[derive(Debug, PartialEq, AtatCmd)]
    #[at_cmd("+CFUN", NoResponse)]
    struct LengthTester<'a> {
        x: u8,
        y: String<consts::U128>,
        #[at_arg(len = 2)]
        z: u16,
        #[at_arg(len = 150)]
        w: &'a str,
        a: SimpleEnum,
        b: SimpleEnumU32,
        #[at_arg(len = 3)]
        c: SimpleEnumU32,
        // d: Vec<SimpleEnumU32, consts::U5>,
    }

    #[test]
    fn test_atat_len() {
        assert_eq!(<char as AtatLen>::Len::to_usize(), 1);
        assert_eq!(<bool as AtatLen>::Len::to_usize(), 5);
        assert_eq!(<isize as AtatLen>::Len::to_usize(), 19);
        assert_eq!(<usize as AtatLen>::Len::to_usize(), 20);
        assert_eq!(<u8 as AtatLen>::Len::to_usize(), 3);
        assert_eq!(<u16 as AtatLen>::Len::to_usize(), 5);
        assert_eq!(<u32 as AtatLen>::Len::to_usize(), 10);
        assert_eq!(<u64 as AtatLen>::Len::to_usize(), 20);
        assert_eq!(<u128 as AtatLen>::Len::to_usize(), 39);
        assert_eq!(<i8 as AtatLen>::Len::to_usize(), 4);
        assert_eq!(<i16 as AtatLen>::Len::to_usize(), 6);
        assert_eq!(<i32 as AtatLen>::Len::to_usize(), 11);
        assert_eq!(<i64 as AtatLen>::Len::to_usize(), 20);
        assert_eq!(<i128 as AtatLen>::Len::to_usize(), 40);
        assert_eq!(<f32 as AtatLen>::Len::to_usize(), 42);
        assert_eq!(<f64 as AtatLen>::Len::to_usize(), 312);

        assert_eq!(<SimpleEnum as AtatLen>::Len::to_usize(), 3);
        assert_eq!(<SimpleEnumU32 as AtatLen>::Len::to_usize(), 10);
        // (fields) + (n_fields - 1)
        // (3 + 128 + 2 + 150 + 3 + 10 + 3 + (10*5)) + 7
        assert_eq!(
            <LengthTester<'_> as AtatLen>::Len::to_usize(),
            (3 + 128 + 2 + 150 + 3 + 10 + 3) + 6
        );
        assert_eq!(
            <MixedEnum<'_> as AtatLen>::Len::to_usize(),
            (3 + 3 + 10 + 20 + 10) + 4
        );
    }

    #[test]
    fn test_length_serialize() {
        assert_eq!(
            LengthTester {
                x: 8,
                y: String::from("SomeString"),
                z: 2,
                w: &"whatup",
                a: SimpleEnum::A,
                b: SimpleEnumU32::A,
                c: SimpleEnumU32::B,
                // d: Vec::new()
            }
            .as_bytes(),
            Vec::<u8, consts::U360>::from_slice(b"AT+CFUN=8,\"SomeString\",2,\"whatup\",0,0,1\r\n")
                .unwrap()
        );
    }

    #[test]
    fn test_mixed_enum() {
        assert_eq!(
            to_string::<consts::U1, consts::U3, _>(
                &MixedEnum::UnitVariant,
                String::from("CMD"),
                SerializeOptions::default()
            )
            .unwrap(),
            String::<consts::U1>::from("0")
        );
        assert_eq!(
            to_string::<consts::U10, consts::U3, _>(
                &MixedEnum::SingleSimpleTuple(15),
                String::from("CMD"),
                SerializeOptions::default()
            )
            .unwrap(),
            String::<consts::U10>::from("1,15")
        );
        assert_eq!(
            to_string::<consts::U50, consts::U3, _>(
                &MixedEnum::AdvancedTuple(25, String::from("testing"), -54, SimpleEnumU32::A),
                String::from("CMD"),
                SerializeOptions::default()
            )
            .unwrap(),
            String::<consts::U50>::from("2,25,\"testing\",-54,0")
        );
        assert_eq!(
            to_string::<consts::U10, consts::U3, _>(
                &MixedEnum::SingleSimpleStruct { x: 35 },
                String::from("CMD"),
                SerializeOptions::default()
            )
            .unwrap(),
            String::<consts::U10>::from("3,35")
        );

        assert_eq!(
            to_string::<consts::U50, consts::U3, _>(
                &MixedEnum::AdvancedStruct {
                    a: 77,
                    b: String::from("whaat"),
                    c: 88,
                    d: SimpleEnum::B
                },
                String::from("CMD"),
                SerializeOptions::default()
            )
            .unwrap(),
            String::<consts::U50>::from("4,77,\"whaat\",88,1")
        );

        assert_eq!(Ok(MixedEnum::UnitVariant), from_str::<MixedEnum<'_>>("0"));
        assert_eq!(
            Ok(MixedEnum::SingleSimpleTuple(67)),
            from_str::<MixedEnum<'_>>("1,67")
        );
        assert_eq!(
            Ok(MixedEnum::AdvancedTuple(
                251,
                String::from("deser"),
                -43,
                SimpleEnumU32::C
            )),
            from_str::<MixedEnum<'_>>("2,251,\"deser\",-43,2")
        );

        assert_eq!(
            Ok(MixedEnum::SingleSimpleTupleLifetime("abc")),
            from_str::<MixedEnum<'_>>("6,\"abc\"")
        );
    }
}
