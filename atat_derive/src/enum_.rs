use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse_macro_input;

use crate::parse::{ArgAttributes, EnumAttributes, ParseInput, Variant};

pub fn atat_enum(input: TokenStream) -> TokenStream {
    let ParseInput {
        ident,
        at_enum,
        variants,
        ..
    } = parse_macro_input!(input as ParseInput);

    let repr = at_enum
        .unwrap_or_else(|| EnumAttributes {
            repr: format_ident!("u8"),
        })
        .repr;

    let variant_idents = variants.iter().map(|variant| &variant.ident);

    let match_variants = variants.iter().map(|variant| {
        let variant_ident = variant.ident.clone();
        let val = if let Some(ArgAttributes { value: Some(v), .. }) = variant.attrs.at_arg {
            quote! { #v }
        } else {
            quote! { #ident::#variant_ident }
        };

        quote! {
            #ident::#variant_ident => #val as #repr,
        }
    });

    let declare_discriminants = variants.iter().map(|variant| {
        let variant_ident = variant.ident.clone();
        let val = if let Some(ArgAttributes { value: Some(v), .. }) = variant.attrs.at_arg {
            quote! { #v }
        } else {
            quote! { #ident::#variant_ident }
        };

        quote! {
            #[allow(non_upper_case_globals)]
            const #variant_ident: #repr = #val as #repr;
        }
    });

    let match_discriminants = variants.iter().map(
        |Variant {
             ident: variant_ident,
             ..
         }| {
            quote! {
                discriminant::#variant_ident => core::result::Result::Ok(#ident::#variant_ident),

            }
        },
    );

    let error_format = match variants.len() {
        1 => "invalid value: {}, expected {}".to_owned(),
        2 => "invalid value: {}, expected {} or {}".to_owned(),
        n => {
            "invalid value: {}, expected one of: {}".to_owned()
                + &std::iter::repeat(", {}").take(n - 1).collect::<String>()
        }
    };

    let other_arm = quote! {
        core::result::Result::Err(serde::de::Error::custom(
            format_args!(#error_format, other #(, discriminant::#variant_idents)*)
        ))
    };

    TokenStream::from(quote! {
        impl atat::AtatLen for #ident {
            type Len = <#repr as atat::AtatLen>::Len;
        }

        impl serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer
            {
                let value: #repr = match *self {
                    #(#match_variants)*
                };
                serde::Serialize::serialize(&value, serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct discriminant;

                impl discriminant {
                    #(#declare_discriminants)*
                }

                match <#repr as serde::Deserialize>::deserialize(deserializer)? {
                    #(#match_discriminants)*
                    other => #other_arm,
                }
            }
        }
    })
}
