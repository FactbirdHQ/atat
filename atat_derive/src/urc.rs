use crate::proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, parse_quote, Fields};

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

    let mut match_arms = Vec::with_capacity(variants.len());
    let mut parsers = Vec::with_capacity(variants.len());

    for variant in &variants {
        let Some(UrcAttributes { code, parse }) = variant.attrs.at_urc.clone() else {
            panic!("missing #[at_urc(...)] attribute")
        };

        let parse_fn = parse.unwrap_or_else(|| parse_quote! { atat::digest::parser::urc_helper });
        parsers.push(quote! { #parse_fn(&#code[..]) });

        let variant_ident = variant.ident.clone();
        match variant.fields.clone() {
            Some(Fields::Named(_)) => panic!("cannot handle named enum variants"),
            Some(Fields::Unnamed(f)) => {
                let mut field_iter = f.unnamed.iter();
                let first_field = field_iter
                    .next()
                    .expect("variant must have exactly one field");
                assert!(
                    field_iter.next().is_none(),
                    "cannot handle variants with more than one field"
                );
                match_arms.push(quote! {
                    #code => #ident::#variant_ident(atat::serde_at::from_slice::<#first_field>(&resp).ok()?),
                });
            }
            Some(Fields::Unit) => {
                match_arms.push(quote! {
                    #code => #ident::#variant_ident,
                });
            }
            None => {
                panic!()
            }
        }
    }

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::AtatUrc for #ident #ty_generics #where_clause {
            #[inline]
            fn parse(resp: &[u8]) -> Option<Self> {
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
            fn parse<'a>(buf: &'a [u8]) -> Result<(&'a [u8], usize), atat::digest::ParseError> {
                #(
                    if let Some(r) = atat::digest::parser::try_urc(#parsers, buf) { return r };
                )*
                Err(atat::digest::ParseError::NoMatch)
            }
        }
    })
}
