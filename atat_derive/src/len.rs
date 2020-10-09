use crate::proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Ident};

use crate::parse::{parse_field_attr, ArgAttributes, FieldAttributes, ParseInput, Variant};

/// Calculate the serialized length of a struct
///
/// Use `#[at_arg(len = xxx)]`, with a fallback to
/// types `AtatLen` implementation, allowing overwriting the max length of all
/// types, including borrowed data
pub fn struct_len(variants: Vec<Variant>, init_len: usize) -> proc_macro2::TokenStream {
    let init_len_ident = format_ident!("U{}", init_len);
    let mut struct_len = quote! { ::heapless::consts::#init_len_ident };
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

/// Calculate the serialized length of an enum, as the longest of all variants
///
/// Use `#[at_arg(len = xxx)]`, with a fallback to
/// types `AtatLen` implementation, allowing overwriting the max length of all
/// types, including borrowed data
pub fn enum_len(
    variants: Vec<Variant>,
    repr: &Ident,
    _generics: &mut syn::Generics,
) -> proc_macro2::TokenStream {
    let mut enum_len = quote! { ::heapless::consts::U0 };
    for variant in variants {
        if let Some(fields) = variant.fields {
            let mut fields_len = quote! { ::heapless::consts::U0 };
            for field in fields {
                let field_len = if let Ok(FieldAttributes {
                    at_arg:
                        Some(ArgAttributes {
                            len: Some(len),
                            value,
                            position,
                        }),
                    ..
                }) = parse_field_attr(&field.attrs)
                {
                    if value.is_some() {
                        panic!("value is not allowed in this position");
                    }
                    if position.is_some() {
                        panic!("position is not allowed in this position");
                    }
                    let len_ident = format_ident!("U{}", len);
                    quote! { ::heapless::consts::#len_ident }
                } else {
                    let ty = field.ty;
                    quote! { <#ty as atat::AtatLen>::Len }
                };
                fields_len = quote! {
                    <<#field_len as core::ops::Add<#fields_len>>::Output as core::ops::Add<::heapless::consts::U1>>::Output
                };
            }
            enum_len = quote! {
                <#fields_len as atat::typenum::type_operators::Max<#enum_len>>::Output
            };
        }
    }
    quote! { <<#repr as atat::AtatLen>::Len as core::ops::Add<#enum_len>>::Output }
}

pub fn atat_len(input: TokenStream) -> TokenStream {
    let ParseInput {
        ident,
        generics,
        variants,
        ..
    } = parse_macro_input!(input as ParseInput);

    let n_fields = variants.len();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let struct_len = struct_len(variants, n_fields.checked_sub(1).unwrap_or(n_fields));

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::AtatLen for #ident #ty_generics #where_clause {
            type Len = #struct_len;
        }
    })
}
