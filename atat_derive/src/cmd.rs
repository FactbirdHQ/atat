use crate::proc_macro::TokenStream;

use quote::{format_ident, quote};
use syn::parse_macro_input;

use crate::parse::{CmdAttributes, ParseInput};

pub fn atat_cmd(input: TokenStream) -> TokenStream {
    let ParseInput {
        ident,
        at_cmd,
        generics,
        variants,
        ..
    } = parse_macro_input!(input as ParseInput);

    let CmdAttributes {
        cmd,
        resp,
        timeout_ms,
        abortable,
        force_receive_state,
        value_sep,
        cmd_prefix,
        termination,
    } = at_cmd.expect("missing #[at_cmd(...)] attribute");

    let ident_str = ident.to_string();

    let n_fields = variants.len();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let timeout = match timeout_ms {
        Some(timeout_ms) => {
            quote! {
                fn max_timeout_ms(&self) -> u32 {
                    #timeout_ms
                }
            }
        }
        None => quote! {},
    };

    let abortable = match abortable {
        Some(abortable) => {
            quote! {
                fn can_abort(&self) -> bool {
                    #abortable
                }
            }
        }
        None => quote! {},
    };

    let force_receive = match force_receive_state {
        Some(force_receive_state) => {
            quote! {
                fn force_receive_state(&self) -> bool {
                    #force_receive_state
                }
            }
        }
        None => quote! {},
    };

    let subcmd_len_ident = format_ident!("U{}", cmd.len());
    let mut cmd_len = cmd_prefix.len() + cmd.len() + termination.len();
    if value_sep {
        cmd_len += 1;
    }

    let cmd_len_ident = format_ident!("U{}", cmd_len);

    let (field_names, field_names_str): (Vec<_>, Vec<_>) = variants
        .iter()
        .map(|f| {
            let ident = f.ident.clone().unwrap();
            (ident.clone(), ident.to_string())
        })
        .unzip();

    let struct_len = crate::len::struct_len(variants, n_fields.checked_sub(1).unwrap_or(n_fields));

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::AtatLen for #ident #ty_generics #where_clause {
            type Len = #struct_len;
        }

        #[automatically_derived]
        impl #impl_generics atat::AtatCmd for #ident #ty_generics #where_clause {
            type Response = #resp;
            type CommandLen = <<Self as atat::AtatLen>::Len as core::ops::Add<::heapless::consts::#cmd_len_ident>>::Output;

            #[inline]
            fn as_bytes(&self) -> ::heapless::Vec<u8, Self::CommandLen> {
                let s: ::heapless::String<::heapless::consts::#subcmd_len_ident> = ::heapless::String::from(#cmd);
                match atat::serde_at::to_vec(self, s, atat::serde_at::SerializeOptions {
                    value_sep: #value_sep,
                    cmd_prefix: #cmd_prefix,
                    termination: #termination
                }) {
                    Ok(s) => s,
                    Err(_) => panic!("Failed to serialize command")
                }
            }

            #[inline]
            fn parse(&self, resp: &[u8]) -> core::result::Result<#resp, atat::Error> {
                atat::serde_at::from_slice::<#resp>(resp).map_err(|e| {
                    atat::Error::ParseString
                })
            }

            #timeout

            #abortable

            #force_receive
        }

        #[automatically_derived]
        impl #impl_generics serde::Serialize for #ident #ty_generics #where_clause {
            #[inline]
            fn serialize<S>(
                &self,
                serializer: S,
            ) -> serde::export::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut serde_state = match serde::Serializer::serialize_struct(
                    serializer,
                    #ident_str,
                    #n_fields,
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
    })
}
