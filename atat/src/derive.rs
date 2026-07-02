#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use crate as atat;
    use atat::AtatCmd;
    use atat_derive::{AtatCmd, AtatEnum, AtatResp};
    use heapless::{String, Vec};
    use serde_at::{from_str, to_string, SerializeOptions};

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
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[at_enum(u32)]
    enum SimpleEnumU32 {
        #[at_arg(default)]
        A,
        B,
        C,
        D,
    }

    #[derive(Debug, PartialEq, AtatEnum)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
        SingleSimpleTupleLifetime(&'a str),
    }

    #[derive(Debug, PartialEq, AtatCmd)]
    #[at_cmd("+CFUN", NoResponse)]
    struct LengthTester<'a> {
        x: u8,
        y: String<128>,
        z: u16,
        w: &'a str,
        a: SimpleEnum,
        b: SimpleEnumU32,
        c: SimpleEnumU32,
        // d: Vec<SimpleEnumU32, 5>,
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
            Vec::<u8, 360>::from_slice(b"AT+CFUN=8,\"SomeString\",2,\"whatup\",0,0,1\r").unwrap()
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
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
