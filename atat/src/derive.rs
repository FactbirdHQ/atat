use heapless::{String, Vec};
use serde_at::HexStr;

/// Trait used by [`atat_derive`] to estimate lengths of the serialized commands, at compile time.
///
/// [`atat_derive`]: https://crates.io/crates/atat_derive
pub trait AtatLen {
    const LEN: usize;
}

#[cfg(feature = "bytes")]
impl<const N: usize> AtatLen for heapless_bytes::Bytes<N> {
    const LEN: usize = N;
}

macro_rules! impl_length {
    ($type:ty, $len:expr) => {
        #[allow(clippy::use_self)]
        impl AtatLen for $type {
            const LEN: usize = $len;
        }
    };
}

impl_length!(char, 1);
impl_length!(bool, 5);
impl_length!(isize, 19);
impl_length!(usize, 20);
impl_length!(u8, 3);
impl_length!(u16, 5);
impl_length!(u32, 10);
impl_length!(u64, 20);
impl_length!(u128, 39);
impl_length!(i8, 4);
impl_length!(i16, 6);
impl_length!(i32, 11);
impl_length!(i64, 20);
impl_length!(i128, 40);
impl_length!(f32, 42);
impl_length!(f64, 312);

//       0x   F:F:F:F
// uN = (2 + (N/2) - 1) * 2 bytes
impl_length!(HexStr<u8>, 10);
impl_length!(HexStr<u16>, 18);
impl_length!(HexStr<u32>, 30);
impl_length!(HexStr<u64>, 66);
impl_length!(HexStr<u128>, 130);

impl<const T: usize> AtatLen for String<T> {
    const LEN: usize = 1 + T + 1;
}

impl<T: AtatLen> AtatLen for Option<T> {
    const LEN: usize = T::LEN;
}

impl<T: AtatLen> AtatLen for &T {
    const LEN: usize = T::LEN;
}

impl<T, const L: usize> AtatLen for Vec<T, L>
where
    T: AtatLen,
{
    const LEN: usize = L * <T as AtatLen>::LEN;
}

//       0x   F:F:F:F
// uN = (2 + (N*4) - 1) * 2 bytes
impl<const L: usize> AtatLen for HexStr<[u8; L]> {
    const LEN: usize = (2 + L * 4 - 1) * 2;
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use crate as atat;
    use atat::{derive::AtatLen, AtatCmd};
    use atat_derive::{AtatCmd, AtatEnum, AtatResp};
    use heapless::{String, Vec};
    use serde_at::{from_str, to_string, HexStr, SerializeOptions};

    macro_rules! assert_not_impl {
        ($x:ty, $($t:path),+ $(,)*) => {
            const _: fn() -> () = || {
                struct Check<T: ?Sized>(T);
                trait AmbiguousIfImpl<A> { fn some_item() { } }

                impl<T: ?Sized> AmbiguousIfImpl<()> for Check<T> { }
                impl<T: ?Sized $(+ $t)*> AmbiguousIfImpl<u8> for Check<T> { }

                <Check::<$x> as AmbiguousIfImpl<_>>::some_item()
            };
        };
    }

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
        AdvancedTuple(u8, String<10>, i64, SimpleEnumU32),
        #[at_arg(value = 3)]
        SingleSimpleStruct { x: u8 },
        #[at_arg(value = 4)]
        AdvancedStruct {
            a: u8,
            b: String<10>,
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
        y: String<128>,
        #[at_arg(len = 2)]
        z: u16,
        #[at_arg(len = 150)]
        w: &'a str,
        a: SimpleEnum,
        b: SimpleEnumU32,
        #[at_arg(len = 3)]
        c: SimpleEnumU32,
        // d: Vec<SimpleEnumU32, 5>,
    }

    #[test]
    fn test_atat_len() {
        assert_eq!(<char as AtatLen>::LEN, 1);
        assert_eq!(<bool as AtatLen>::LEN, 5);
        assert_eq!(<isize as AtatLen>::LEN, 19);
        assert_eq!(<usize as AtatLen>::LEN, 20);
        assert_eq!(<u8 as AtatLen>::LEN, 3);
        assert_eq!(<u16 as AtatLen>::LEN, 5);
        assert_eq!(<u32 as AtatLen>::LEN, 10);
        assert_eq!(<u64 as AtatLen>::LEN, 20);
        assert_eq!(<u128 as AtatLen>::LEN, 39);
        assert_eq!(<i8 as AtatLen>::LEN, 4);
        assert_eq!(<i16 as AtatLen>::LEN, 6);
        assert_eq!(<i32 as AtatLen>::LEN, 11);
        assert_eq!(<i64 as AtatLen>::LEN, 20);
        assert_eq!(<i128 as AtatLen>::LEN, 40);
        assert_eq!(<f32 as AtatLen>::LEN, 42);
        assert_eq!(<f64 as AtatLen>::LEN, 312);

        assert_eq!(<SimpleEnum as AtatLen>::LEN, 3);
        assert_eq!(<SimpleEnumU32 as AtatLen>::LEN, 10);

        assert_eq!(<HexStr<u8> as AtatLen>::LEN, 10);
        assert_eq!(<HexStr<u16> as AtatLen>::LEN, 18);
        assert_eq!(<HexStr<u32> as AtatLen>::LEN, 30);
        assert_eq!(<HexStr<u64> as AtatLen>::LEN, 66);
        assert_eq!(<HexStr<u128> as AtatLen>::LEN, 130);

        #[cfg(feature = "hex_str_arrays")]
        {
            assert_eq!(<HexStr<[u8; 16]> as AtatLen>::LEN, 130);
        }

        // (fields) + (n_fields - 1)
        // (3 + (1 + 128 + 1) + 2 + (1 + 150 + 1) + 3 + 10 + 3 + (10*5)) + 7
        assert_eq!(
            <LengthTester<'_> as AtatLen>::LEN,
            (3 + (1 + 128 + 1) + 2 + (1 + 150 + 1) + 3 + 10 + 3) + 6
        );
        assert_eq!(
            <MixedEnum<'_> as AtatLen>::LEN,
            (3 + 3 + (1 + 10 + 1) + 20 + 10) + 4
        );
    }

    #[test]
    fn test_length_serialize() {
        let mut buf = [0; 360];
        let len = LengthTester {
            x: 8,
            y: String::try_from("SomeString").unwrap(),
            z: 2,
            w: "whatup",
            a: SimpleEnum::A,
            b: SimpleEnumU32::A,
            c: SimpleEnumU32::B,
            // d: Vec::new()
        }
        .write(&mut buf);
        assert_eq!(
            &buf[..len],
            Vec::<u8, 360>::from_slice(b"AT+CFUN=8,\"SomeString\",2,\"whatup\",0,0,1\r\n").unwrap()
        );
    }

    #[test]
    fn test_mixed_enum() {
        assert_not_impl!(MixedEnum, TryFrom<u8>);
        assert_not_impl!(MixedEnum, TryFrom<u16>);
        assert_not_impl!(MixedEnum, TryFrom<u32>);

        assert_eq!(SimpleEnum::try_from(3), Ok(SimpleEnum::D));
        assert_eq!(SimpleEnumU32::try_from(1), Ok(SimpleEnumU32::B));
        assert_eq!(
            to_string::<_, 1>(&MixedEnum::UnitVariant, "CMD", SerializeOptions::default()).unwrap(),
            String::<1>::try_from("0").unwrap()
        );
        assert_eq!(
            to_string::<_, 10>(
                &MixedEnum::SingleSimpleTuple(15),
                "CMD",
                SerializeOptions::default()
            )
            .unwrap(),
            String::<10>::try_from("1,15").unwrap()
        );
        assert_eq!(
            to_string::<_, 50>(
                &MixedEnum::AdvancedTuple(
                    25,
                    String::try_from("testing").unwrap(),
                    -54,
                    SimpleEnumU32::A
                ),
                "CMD",
                SerializeOptions::default()
            )
            .unwrap(),
            String::<50>::try_from("2,25,\"testing\",-54,0").unwrap()
        );
        assert_eq!(
            to_string::<_, 10>(
                &MixedEnum::SingleSimpleStruct { x: 35 },
                "CMD",
                SerializeOptions::default()
            )
            .unwrap(),
            String::<10>::try_from("3,35").unwrap()
        );

        assert_eq!(
            to_string::<_, 50>(
                &MixedEnum::AdvancedStruct {
                    a: 77,
                    b: String::try_from("whaat").unwrap(),
                    c: 88,
                    d: SimpleEnum::B
                },
                "CMD",
                SerializeOptions::default()
            )
            .unwrap(),
            String::<50>::try_from("4,77,\"whaat\",88,1").unwrap()
        );

        assert_eq!(Ok(MixedEnum::UnitVariant), from_str::<MixedEnum<'_>>("0"));
        assert_eq!(
            Ok(MixedEnum::SingleSimpleTuple(67)),
            from_str::<MixedEnum<'_>>("1,67")
        );
        assert_eq!(
            Ok(MixedEnum::AdvancedTuple(
                251,
                String::try_from("deser").unwrap(),
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

    fn custom_parse(response: &[u8]) -> Result<CustomResponseParse, atat::Error> {
        Ok(CustomResponseParse {
            arg1: core::str::from_utf8(&response[6..])
                .unwrap()
                .parse()
                .unwrap(),
        })
    }

    #[derive(Debug, PartialEq, AtatResp)]
    struct CustomResponseParse {
        arg1: u8,
    }

    #[derive(Debug, PartialEq, AtatCmd)]
    #[at_cmd("+CFUN", CustomResponseParse, parse = custom_parse)]
    struct RequestWithCustomResponseParse;

    #[test]
    fn test_custom_parse() {
        assert_eq!(
            RequestWithCustomResponseParse.parse(Ok(b"ignore123")),
            Ok(CustomResponseParse { arg1: 123 })
        );
    }
}
