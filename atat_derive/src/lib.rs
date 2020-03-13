// #![feature(proc_macro_diagnostic)]

extern crate proc_macro;
extern crate proc_macro2;

mod cmd;
mod helpers;
mod resp;
mod urc;

use crate::proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(AtatResp, attributes(at_arg))]
pub fn derive_atat_resp(input: TokenStream) -> TokenStream {
    resp::atat_resp(syn::parse(input).expect("Failed to parse input stream!"))
}

#[proc_macro_derive(AtatUrc, attributes(at_urc))]
pub fn derive_atat_urc(input: TokenStream) -> TokenStream {
    urc::atat_urc(syn::parse(input).expect("Failed to parse input stream!"))
}

#[proc_macro_derive(AtatEnum, attributes(at_arg))]
pub fn derive_atat_enum(_input: TokenStream) -> TokenStream {
    // let item: DeriveInput = syn::parse(input).expect("Failed to parse input stream!";)
    TokenStream::from(quote! {})
}

#[proc_macro_derive(AtatCmd, attributes(at_cmd, at_arg))]
pub fn derive_atat_cmd(input: TokenStream) -> TokenStream {
    cmd::atat_cmd(syn::parse(input).expect("Failed to parse input stream!"))
}
