use crate::proc_macro::TokenStream;
use crate::proc_macro2::Literal;

use quote::quote;
use syn::{Attribute, Data, DataStruct, DeriveInput, Fields, FieldsNamed, Ident, Result};

use crate::helpers::{get_field_names, get_ident, get_lit, get_name_ident_lit};

pub fn atat_cmd(item: DeriveInput) -> TokenStream {
    match item.data {
        Data::Struct(struct_) => {
            let at_cmd_attr = get_cmd_response(&item.attrs).unwrap();
            match struct_ {
                DataStruct {
                    fields: Fields::Named(fields),
                    ..
                } => generate_cmd_output(&item.ident, &item.generics, &at_cmd_attr, Some(&fields)),
                DataStruct {
                    fields: Fields::Unit,
                    ..
                } => {
                    let at_cmd_attr = get_cmd_response(&item.attrs).unwrap();
                    generate_cmd_output(&item.ident, &item.generics, &at_cmd_attr, None)
                }
                _ => panic!("Cannot handle unnamed struct fields"),
            }
        }
        _ => {
            panic!("ATATCmd can only be applied to structs!");
        }
    }
}

#[derive(Debug)]
struct AtCmdAttr {
    cmd: Literal,
    resp: Ident,
    timeout_ms: Option<u32>,
    abortable: Option<bool>,
    value_sep: bool,
}

fn get_cmd_response(attrs: &[Attribute]) -> Result<AtCmdAttr> {
    if let Some(attr) = attrs.iter().find(|attr| attr.path.is_ident("at_cmd")) {
        let timeout_ms = match get_name_ident_lit(&attr.tokens, "timeout_ms") {
            Ok(lit) => match lit.parse::<u32>() {
                Ok(t) => Some(t),
                _ => None,
            },
            Err(_) => None,
        };
        let abortable = match get_name_ident_lit(&attr.tokens, "abortable") {
            Ok(lit) => match lit.parse::<bool>() {
                Ok(t) => Some(t),
                _ => None,
            },
            Err(_) => None,
        };
        // println!("{:?}", attr.tokens);
        let value_sep = match get_name_ident_lit(&attr.tokens, "value_sep") {
            Ok(lit) => match lit.parse::<bool>() {
                Ok(c) => c,
                _ => true,
            },
            Err(_) => true,
        };
        Ok(AtCmdAttr {
            cmd: get_lit(&attr.tokens)?,
            resp: get_ident(&attr.tokens)?,
            timeout_ms,
            abortable,
            value_sep,
        })
    } else {
        panic!("Failed to find non-optional at_cmd attribute!",)
    }
}

fn generate_cmd_output(
    name: &Ident,
    generics: &syn::Generics,
    attr: &AtCmdAttr,
    fields: Option<&FieldsNamed>,
) -> TokenStream {
    let name_str = &name.to_string();
    let cmd = &attr.cmd;
    let response = &attr.resp;

    let (field_names, _, field_names_str) = get_field_names(fields);
    let len = field_names.len();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let timeout = if let Some(timeout_ms) = &attr.timeout_ms {
        quote! {
            fn max_timeout_ms(&self) -> u32 {
                #timeout_ms
            }
        }
    } else {
        quote! {}
    };

    let abortable = if let Some(abortable) = &attr.abortable {
        quote! {
            fn can_abort(&self) -> bool {
                #abortable
            }
        }
    } else {
        quote! {}
    };

    let value_sep = &attr.value_sep;

    #[cfg(feature = "error-message")]
    let invalid_resp_err = quote! { atat::Error::InvalidResponseWithMessage(String::from(resp)) };
    #[cfg(not(feature = "error-message"))]
    let invalid_resp_err = quote! { atat::Error::InvalidResponse };

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::ATATCmd for #name #ty_generics #where_clause {
            type Response = #response;
            type CommandLen = heapless::consts::U64;

            fn as_str(&self) -> heapless::String<Self::CommandLen> {
                let s: heapless::String<Self::CommandLen> = heapless::String::from(#cmd);
                match serde_at::to_string(self, s, #value_sep) {
                    Ok(s) => s,
                    Err(_) => String::new()
                }
            }

            fn parse(&self, resp: &str) -> core::result::Result<#response, atat::Error> {
                serde_at::from_str::<#response>(resp).map_err(|e| {
                    #invalid_resp_err
                })
            }

            #timeout

            #abortable
        }

        #[automatically_derived]
        impl #impl_generics serde::Serialize for #name #ty_generics #where_clause {
            fn serialize<S>(
                &self,
                serializer: S,
            ) -> serde::export::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut serde_state = match serde::Serializer::serialize_struct(
                    serializer,
                    #name_str,
                    #len,
                ) {
                    serde::export::Ok(val) => val,
                    serde::export::Err(err) => {
                        return serde::export::Err(err);
                    }
                };

                #(
                    match serde::ser::SerializeStruct::serialize_field(
                        &mut serde_state,
                        #field_names_str,
                        &self.#field_names,
                    ) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                )*

                serde::ser::SerializeStruct::end(serde_state)
            }
        }

        #[automatically_derived]
        #[cfg(feature = "use_ufmt")]
        impl #impl_generics ufmt::uDebug for #name #ty_generics #where_clause {
            fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> core::result::Result<(), W::Error>
            where
                W: ufmt::uWrite + ?Sized,
            {
                f.write_str(&self.as_str())
            }
        }

        #[automatically_derived]
        #[cfg(feature = "use_ufmt")]
        impl #impl_generics ufmt::uDisplay for #name #ty_generics #where_clause {
            fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> core::result::Result<(), W::Error>
            where
                W: ufmt::uWrite + ?Sized,
            {
                let c = self.as_str();
                f.write_str(&c[0..c.len() - 2])
            }
        }
    })
}
