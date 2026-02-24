use crate::proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, Type};

use crate::parse::{parse_field_attr, ArgAttributes, FieldAttributes, ParseInput, Variant};

/// Calculate the serialized length of a struct
///
/// Use `#[at_arg(len = xxx)]`, with a fallback to
/// types `AtatLen` implementation, allowing overwriting the max length of all
/// types, including borrowed data
///
/// When `escaped` is true, uses `ESCAPED_LEN` for types and `3 * len + 2` for
/// `&str` fields with `#[at_arg(len)]`, accounting for worst-case escape expansion.
pub fn struct_len(
    variants: Vec<Variant>,
    init_len: usize,
    escaped: bool,
) -> proc_macro2::TokenStream {
    let mut struct_len = quote! { #init_len };
    for field in variants {
        let len = if let Some(ArgAttributes { len: Some(len), .. }) = field.attrs.at_arg {
            let ty = field.ty.unwrap();
            if is_ref_str(ty) {
                if escaped {
                    quote! { 3 * #len + 2 }
                } else {
                    quote! { 1 + #len + 1 }
                }
            } else {
                quote! { #len }
            }
        } else {
            let ty = field.ty.unwrap();
            if escaped {
                quote! { <#ty as atat::AtatLen>::ESCAPED_LEN }
            } else {
                quote! { <#ty as atat::AtatLen>::LEN }
            }
        };
        struct_len = quote! {
            #len + #struct_len
        };
    }
    struct_len
}

fn is_ref_str(ty: Type) -> bool {
    match ty {
        Type::Reference(r) => match r.elem.as_ref() {
            Type::Path(p) => p.path.segments.len() == 1 && p.path.segments[0].ident == "str",
            _ => false,
        },
        _ => false,
    }
}

/// Calculate the serialized length of an enum, as the longest of all variants
///
/// Use `#[at_arg(len = xxx)]`, with a fallback to
/// types `AtatLen` implementation, allowing overwriting the max length of all
/// types, including borrowed data
///
/// When `escaped` is true, uses `ESCAPED_LEN` for types, accounting for
/// worst-case escape expansion.
pub fn enum_len(
    variants: &[Variant],
    repr: &Ident,
    _generics: &mut syn::Generics,
    escaped: bool,
) -> proc_macro2::TokenStream {
    let mut enum_len = quote! { 0 };
    for variant in variants {
        if let Some(ref fields) = variant.fields {
            let mut fields_len = quote! { 0 };
            for field in fields {
                let field_len = if let Ok(FieldAttributes {
                    at_arg:
                        Some(ArgAttributes {
                            len: Some(len),
                            value,
                            position,
                            ..
                        }),
                    ..
                }) = parse_field_attr(&field.attrs)
                {
                    assert!(value.is_none(), "value is not allowed in this position");
                    assert!(
                        position.is_none(),
                        "position is not allowed in this position"
                    );
                    if escaped && is_ref_str(field.ty.clone()) {
                        quote! { 3 * #len + 2 }
                    } else {
                        quote! { #len }
                    }
                } else {
                    let ty = &field.ty;
                    if escaped {
                        quote! { <#ty as atat::AtatLen>::ESCAPED_LEN }
                    } else {
                        quote! { <#ty as atat::AtatLen>::LEN }
                    }
                };
                fields_len = quote! {
                    #fields_len + #field_len + 1
                };
            }

            enum_len = quote! {
                {
                    const E_LEN: usize = #enum_len;
                    if #fields_len < E_LEN { E_LEN } else { #fields_len }
                }
            };
        }
    }
    if escaped {
        quote! { <#repr as atat::AtatLen>::ESCAPED_LEN + #enum_len }
    } else {
        quote! { <#repr as atat::AtatLen>::LEN + #enum_len }
    }
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

    let init_len = n_fields.checked_sub(1).unwrap_or(n_fields);
    let len = struct_len(variants.clone(), init_len, false);
    let escaped_len = struct_len(variants, init_len, true);

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::AtatLen for #ident #ty_generics #where_clause {
            const LEN: usize = #len;
            const ESCAPED_LEN: usize = #escaped_len;
        }
    })
}
