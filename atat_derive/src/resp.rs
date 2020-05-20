use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

use crate::{helpers, parse::ParseInput};

pub fn atat_resp(input: TokenStream) -> TokenStream {
    let ParseInput {
        ident,
        generics,
        variants,
        ..
    } = parse_macro_input!(input as ParseInput);

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut serde_generics = generics.clone();
    helpers::add_lifetime(&mut serde_generics, "'de");
    let (serde_impl_generics, _, _) = serde_generics.split_for_impl();

    let deserialize_struct = helpers::deserialize_struct(ident.clone(), variants, &generics);

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::AtatResp for #ident #ty_generics #where_clause {}

        #[automatically_derived]
        impl #serde_impl_generics serde::Deserialize<'de> for #ident #ty_generics #where_clause {
            #[inline]
            fn deserialize<D>(deserializer: D) -> serde::export::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                #deserialize_struct
            }
        }
    })
}
