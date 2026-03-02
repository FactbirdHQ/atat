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
        parse,
        timeout_ms,
        attempts,
        reattempt_on_parse_err,
        abortable,
        response_code,
        value_sep,
        cmd_prefix,
        termination,
        escape_strings,
    } = at_cmd.expect("missing #[at_cmd(...)] attribute");

    let ident_str = ident.to_string();

    let n_fields = variants.len();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let timeout = match timeout_ms {
        Some(timeout_ms) => {
            quote! {
                const MAX_TIMEOUT_MS: u32 = #timeout_ms;
            }
        }
        None => quote! {},
    };

    let abortable = match abortable {
        Some(abortable) => {
            quote! {
                const CAN_ABORT: bool = #abortable;
            }
        }
        None => quote! {},
    };

    let attempts = match attempts {
        Some(attempts) => {
            quote! {
                const ATTEMPTS: u8 = #attempts;
            }
        }
        None => quote! {},
    };

    let response = match response_code {
        Some(is_resp) => {
            quote! {
                const EXPECTS_RESPONSE_CODE: bool = #is_resp;
            }
        }
        None => quote! {},
    };

    let reattempt_on_parse_err = match reattempt_on_parse_err {
        Some(reattempt_on_parse_err) => {
            quote! {
                const REATTEMPT_ON_PARSE_ERR: bool = #reattempt_on_parse_err;
            }
        }
        None => quote! {},
    };

    let mut cmd_len = cmd_prefix.len() + cmd.len() + termination.len();
    if value_sep {
        cmd_len += 1;
    }

    let (field_names, field_names_str): (Vec<_>, Vec<_>) = variants
        .iter()
        .map(|f| {
            let ident = f.ident.clone().unwrap();
            (ident.clone(), ident.to_string())
        })
        .unzip();

    let init_len = n_fields.checked_sub(1).unwrap_or(n_fields);
    let unescaped_struct_len = crate::len::struct_len(variants.clone(), init_len, false);
    let escaped_struct_len = crate::len::struct_len(variants, init_len, true);

    let max_len_struct = if escape_strings {
        &escaped_struct_len
    } else {
        &unescaped_struct_len
    };

    let ident_len = format_ident!("ATAT_{}_LEN", ident.to_string().to_uppercase());

    let parse = if let Some(parse) = parse {
        quote! {
            #[inline]
            fn parse(&self, res: Result<&[u8], atat::InternalError>) -> core::result::Result<Self::Response, atat::Error> {
                match res {
                    Ok(resp) => #parse(resp).map_err(|e| {
                        atat::Error::Parse
                    }),
                    Err(e) => Err(e.into())
                }
            }
        }
    } else {
        quote! {
            #[inline]
           fn parse(&self, res: Result<&[u8], atat::InternalError>) -> core::result::Result<Self::Response, atat::Error> {
               match res {
                   Ok(resp) => atat::serde_at::from_slice::<#resp>(resp).map_err(|e| {
                       atat::Error::Parse
                   }),
                   Err(e) => Err(e.into())
               }
           }
        }
    };

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::AtatLen for #ident #ty_generics #where_clause {
            const LEN: usize = #unescaped_struct_len;
            const ESCAPED_LEN: usize = #escaped_struct_len;
        }

        const #ident_len: usize = #max_len_struct;

        #[automatically_derived]
        impl #impl_generics atat::AtatCmd for #ident #ty_generics #where_clause {
            type Response = #resp;

            const MAX_LEN: usize = { #ident_len + #cmd_len };

            #timeout

            #abortable

            #attempts

            #response

            #reattempt_on_parse_err

            #[inline]
            fn write(&self, buf: &mut [u8]) -> usize {
                match atat::serde_at::to_slice(self, #cmd, buf, atat::serde_at::SerializeOptions {
                    value_sep: #value_sep,
                    cmd_prefix: #cmd_prefix,
                    termination: #termination,
                    escape_strings: #escape_strings
                }) {
                    Ok(s) => s,
                    Err(_) => panic!("Failed to serialize command")
                }
            }

            #parse
        }

        #[automatically_derived]
        impl #impl_generics atat::serde_at::serde::Serialize for #ident #ty_generics #where_clause {
            #[inline]
            fn serialize<S>(
                &self,
                serializer: S,
            ) -> core::result::Result<S::Ok, S::Error>
            where
                S: atat::serde_at::serde::Serializer,
            {
                let mut serde_state = atat::serde_at::serde::Serializer::serialize_struct(
                    serializer,
                    #ident_str,
                    #n_fields,
                )?;

                #(
                    atat::serde_at::serde::ser::SerializeStruct::serialize_field(
                        &mut serde_state,
                        #field_names_str,
                        &self.#field_names,
                    )?;
                )*

                atat::serde_at::serde::ser::SerializeStruct::end(serde_state)
            }
        }
    })
}
