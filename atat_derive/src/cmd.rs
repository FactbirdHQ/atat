use crate::proc_macro::TokenStream;
use crate::proc_macro2::Literal;

use quote::{format_ident, quote};
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
            panic!("AtatCmd can only be applied to structs!");
        }
    }
}

#[derive(Debug)]
struct AtCmdAttr {
    cmd: Literal,
    resp: Ident,
    timeout_ms: Option<u32>,
    abortable: Option<bool>,
    force_receive_state: Option<bool>,
    value_sep: bool,
    cmd_prefix: String,
    termination: String,
}

fn get_parsed_ident<T: core::str::FromStr>(attr: &Attribute, needle: &str) -> Option<T> {
    match get_name_ident_lit(&attr.tokens, needle) {
        Ok(lit) => match lit.parse::<T>() {
            Ok(t) => Some(t),
            _ => None,
        },
        Err(_) => None,
    }
}

fn get_cmd_response(attrs: &[Attribute]) -> Result<AtCmdAttr> {
    if let Some(attr) = attrs.iter().find(|attr| attr.path.is_ident("at_cmd")) {
        Ok(AtCmdAttr {
            cmd: get_lit(&attr.tokens)?,
            resp: get_ident(&attr.tokens)?,
            timeout_ms: get_parsed_ident(&attr, "timeout_ms"),
            abortable: get_parsed_ident(&attr, "abortable"),
            force_receive_state: get_parsed_ident(&attr, "force_receive_state"),
            value_sep: get_parsed_ident(&attr, "value_sep").unwrap_or_else(|| true),
            cmd_prefix: get_parsed_ident(&attr, "cmd_prefix")
                .unwrap_or_else(|| String::from("AT"))
                .replace("\"", ""),
            termination: get_parsed_ident(&attr, "termination")
                .unwrap_or_else(|| String::from("\r\n"))
                .replace("\"", ""),
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

    let force_receive = if let Some(force_receive_state) = &attr.force_receive_state {
        quote! {
            fn force_receive_state(&self) -> bool {
                #force_receive_state
            }
        }
    } else {
        quote! {}
    };

    let termination = &attr.termination;

    let value_sep = &attr.value_sep;
    let cmd_prefix = &attr.cmd_prefix;
    let sub_len = cmd.to_string().replace("\"", "").len();
    let subcmd_len = format_ident!("U{}", sub_len);
    // let cmd_len = format_ident!("U{}", calculate_cmd_len(sub_len, fields, termination.len()));

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::AtatCmd for #name #ty_generics #where_clause {
            type Response = #response;
            type CommandLen = ::heapless::consts::U2048;

            fn as_bytes(&self) -> ::heapless::Vec<u8, Self::CommandLen> {
                let s: ::heapless::String<::heapless::consts::#subcmd_len> = ::heapless::String::from(#cmd);
                match serde_at::to_vec(self, s, serde_at::SerializeOptions {
                    value_sep: #value_sep,
                    cmd_prefix: #cmd_prefix,
                    termination: #termination
                }) {
                    Ok(s) => s,
                    Err(_) => panic!("Failed to serialize command")
                }
            }

            fn parse(&self, resp: &[u8]) -> core::result::Result<#response, atat::Error> {
                serde_at::from_slice::<#response>(resp).map_err(|e| {
                    atat::Error::ParseString
                })
            }

            #timeout

            #abortable

            #force_receive
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
                use atat::AtatCmd as _;
                f.write_str(&self.as_string())
            }
        }

        #[automatically_derived]
        #[cfg(feature = "use_ufmt")]
        impl #impl_generics ufmt::uDisplay for #name #ty_generics #where_clause {
            fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> core::result::Result<(), W::Error>
            where
                W: ufmt::uWrite + ?Sized,
            {
                use atat::AtatCmd as _;
                let c = self.as_string();
                f.write_str(&c[0..c.len() - 2])
            }
        }
    })
}
