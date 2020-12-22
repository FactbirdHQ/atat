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

    if variants.is_empty() {
        panic!("there must be at least one variant");
    }

    let match_arms: Vec<_> = variants.iter().map(|variant| {
        let UrcAttributes {
            code
        } = variant.attrs.at_urc.clone().unwrap_or_else(|| {
            panic!(
                "missing #[at_urc(...)] attribute",
            )
        });

        let variant_ident = variant.ident.clone();
        match variant.fields.clone() {
            Some(Fields::Named(_)) => {
                panic!("cannot handle named enum variants")
            }
            Some(Fields::Unnamed(f)) => {
                let mut field_iter = f.unnamed.iter();
                let first_field = field_iter.next().unwrap_or_else(|| panic!(""));
                if field_iter.next().is_some() {
                    panic!("cannot handle variants with more than one field")
                }
                quote! {
                    #code => #ident::#variant_ident(atat::serde_at::from_slice::<#first_field>(&resp[index..]).ok()?),
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
        }
    }).collect();

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::AtatUrc for #ident #ty_generics #where_clause {
            type Response = #ident;

            #[inline]
            fn parse(resp: &[u8]) -> Option<Self::Response> {
                if let Some(index) = resp.iter().position(|&x| x == b':') {
                    Some(match &resp[..index] {
                        #(
                            #match_arms
                        )*
                        _ => return None
                    })
                } else {
                    None
                }
            }
        }
    })
}
