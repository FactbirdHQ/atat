use crate::proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, Fields};

use crate::parse::{ParseInput, UrcAttributes};

pub fn atat_urc(input: TokenStream) -> TokenStream {
    let ParseInput {
        ident,
        generics,
        variants,
        ..
    } = parse_macro_input!(input as ParseInput);

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    assert!(!variants.is_empty(), "there must be at least one variant");

    let (match_arms, digest_arms): (Vec<_>, Vec<_>) = variants.iter().map(|variant| {
        let UrcAttributes {
            code
        } = variant.attrs.at_urc.clone().unwrap_or_else(|| {
            panic!(
                "missing #[at_urc(...)] attribute",
            )
        });

        let variant_ident = variant.ident.clone();
        let parse_arm = match variant.fields.clone() {
            Some(Fields::Named(_)) => {
                panic!("cannot handle named enum variants")
            }
            Some(Fields::Unnamed(f)) => {
                let mut field_iter = f.unnamed.iter();
                let first_field = field_iter.next().expect("variant must have exactly one field");
                assert!(field_iter.next().is_none(), "cannot handle variants with more than one field");
                quote! {
                    #code => #ident::#variant_ident(atat::serde_at::from_slice::<#first_field>(&resp).ok()?),
                }
            }
            Some(Fields::Unit) => {
                quote! {
                    #code => #ident::#variant_ident,
                }
            }
            None => {
                panic!()
            }
        };

        let digest_arm = quote! {
            atat::digest::parser::urc_helper(&#code[..]),
        };

        (parse_arm, digest_arm)
    }).unzip();

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::AtatUrc for #ident #ty_generics #where_clause {
            type Response = #ident;

            #[inline]
            fn parse(resp: &[u8]) -> Option<Self::Response> {
                // FIXME: this should be more generic than ':' (Split using #code?)
                let index = resp.iter().position(|&x| x == b':').unwrap_or(resp.len());
                Some(match &resp[..index] {
                    #(
                        #match_arms
                    )*
                    _ => return None
                })
            }
        }

        #[automatically_derived]
        impl #impl_generics atat::Parser for #ident #ty_generics #where_clause {
            fn parse<'a>(
                buf: &'a [u8],
            ) -> Result<(&'a [u8], usize), atat::digest::ParseError> {
                let (_, r) = atat::nom::branch::alt((
                    #(
                        #digest_arms
                    )*
                ))(buf)?;

                Ok(r)
            }
        }
    })
}
