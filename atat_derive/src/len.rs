use crate::proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse_macro_input;

use crate::parse::{ArgAttributes, ParseInput, Variant};

/// Calculate the serialized length of a struct
///
/// Use #[at_arg(len = 128)], with a fallback to
/// types AtatLen implementation, allowing overwriting the max length of all
/// types, including borrowed data
pub fn struct_len(variants: Vec<Variant>) -> proc_macro2::TokenStream {
    let mut struct_len = quote! { ::heapless::consts::U0 };
    for field in variants {
        let len = if let Some(ArgAttributes { len: Some(len), .. }) = field.attrs.at_arg {
            let len_ident = format_ident!("U{}", len);
            quote! { ::heapless::consts::#len_ident }
        } else {
            let ty = field.ty.unwrap();
            quote! { <#ty as atat::AtatLen>::Len }
        };
        struct_len = quote! {
            <#len as core::ops::Add<#struct_len>>::Output
        };
    }
    struct_len
}

pub fn atat_len(input: TokenStream) -> TokenStream {
    let ParseInput {
        ident,
        generics,
        variants,
        ..
    } = parse_macro_input!(input as ParseInput);

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let struct_len = struct_len(variants);

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::AtatLen for #ident #ty_generics #where_clause {
            type Len = #struct_len;
        }
    })
}
