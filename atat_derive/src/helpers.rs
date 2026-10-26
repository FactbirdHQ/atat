use std::iter::zip;

use crate::parse::Variant;
use proc_macro2::{Literal, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_quote, GenericParam, Generics, Ident, Lifetime, LifetimeParam, TypeParamBound};

/// Adds a single lifetime symbol eg. <'a>
#[inline]
pub fn add_lifetime(generics: &mut Generics, lifetime_symbol: &str) {
    generics
        .params
        .push(GenericParam::Lifetime(LifetimeParam::new(Lifetime::new(
            lifetime_symbol,
            Span::call_site(),
        ))));
}

/// Adds a type generic symbol eg. <T>
#[inline]
pub fn add_type_generic(generics: &mut Generics, type_param: syn::TypeParam) {
    generics.params.push(GenericParam::Type(type_param));
}

/// Takes a `syn::Generics` ('a), a lifetime bound ('b) and returns a new
/// `syn::Generics` with <'a: 'b, 'b>
#[inline]
pub fn add_lifetime_bound(generics: &mut Generics, bound: &Lifetime) {
    for mut param in &mut generics.params {
        match &mut param {
            GenericParam::Lifetime(param) => {
                param.bounds.push(bound.clone());
            }
            GenericParam::Type(param) => {
                param.bounds.push(TypeParamBound::Lifetime(bound.clone()));
            }
            GenericParam::Const(_) => {}
        }
    }
    add_lifetime(generics, &quote! {#bound}.to_string());
}

/// Adds a type parameter bound given an input Type, and a bound eg.
/// T: Clone
#[inline]
pub fn add_type_parameter_bound(
    generics: &mut Generics,
    bound: syn::TypeParam,
    trait_bound: TypeParamBound,
) {
    add_type_generic(generics, bound.clone());
    let ident = bound.ident;

    let where_type = syn::PredicateType {
        bounded_ty: parse_quote!(#ident),
        colon_token: <syn::Token![:]>::default(),
        bounds: [trait_bound].iter().cloned().collect(),
        lifetimes: None,
    };
    generics
        .make_where_clause()
        .predicates
        .push(where_type.into());
}

pub fn deserialize_struct(ident: &Ident, variants: &[Variant], generics: &Generics) -> TokenStream {
    let ident_str = ident.to_string();

    let (field_names, field_names_str): (Vec<_>, Vec<_>) = variants
        .iter()
        .map(|f| {
            let ident = f.ident.clone().unwrap();
            (ident.clone(), ident.to_string())
        })
        .unzip();
    let field_types: Vec<_> = variants.iter().map(|f| f.ty.clone()).collect();

    let (anon_field_ind, anon_field): (Vec<usize>, Vec<Ident>) = field_names
        .iter()
        .enumerate()
        .map(|(i, _)| (i, format_ident!("__field{}", i)))
        .unzip();

    let anon_field_ind64: Vec<u64> = anon_field_ind.iter().map(|i| *i as u64).collect();
    let anon_field_ind128: Vec<u128> = anon_field_ind.iter().map(|i| *i as u128).collect();
    let len = variants.len();
    let enum_field = format_ident!("{}Field", ident);
    let invalid_len_err = format!("struct {ident} with {len} elements");

    let deserialize_with = variants.iter().map(|f| {
        f.attrs
            .at_arg
            .as_ref()
            .and_then(|attrs| attrs.deserialize_with.as_ref())
    });

    let wrappers: Vec<Option<Ident>> = deserialize_with
        .clone()
        .enumerate()
        .map(|(i, d)| d.map(|_| format_ident!("__DeserializeWith{}", i)))
        .collect();

    let (_, ty_generics, where_clause) = generics.split_for_impl();
    let mut serde_generics = generics.clone();
    add_lifetime(&mut serde_generics, "'de");
    let (serde_impl_generics, serde_ty_generics, _) = serde_generics.split_for_impl();

    let wrapper_defs = zip(&wrappers, deserialize_with)
        .zip(&field_types)
        .filter_map(|((wrapper, path), ty)| {
            let (wrapper, path) = (wrapper.as_ref()?, path?);
            Some(quote! {
                struct #wrapper #serde_impl_generics #where_clause {
                    value: #ty,
                    phantom: core::marker::PhantomData<#ident #ty_generics>,
                    lifetime: core::marker::PhantomData<&'de ()>,
                }
                impl #serde_impl_generics atat::serde_at::serde::Deserialize<'de> for #wrapper #serde_ty_generics #where_clause {
                    fn deserialize<D: atat::serde_at::serde::Deserializer<'de>>(
                        d: D,
                    ) -> core::result::Result<Self, D::Error> {
                        #path(d).map(|value| #wrapper {
                            value,
                            phantom: core::marker::PhantomData,
                            lifetime: core::marker::PhantomData,
                        })
                    }
                }
            })
        });

    let (accessors, serde_types): (Vec<_>, Vec<_>) = zip(&wrappers, &anon_field)
        .zip(&field_types)
        .map(|((w, f), ty)| match w {
            Some(w) => (quote!(#f.value), quote!(#w #serde_ty_generics)),
            None => (quote!(#f), quote!(#ty)),
        })
        .unzip();

    let visitor = format_ident!("{}Visitor", ident);
    let field_visitor = format_ident!("{}FieldVisitor", ident);
    let field_names_bytestr = field_names_str
        .iter()
        .map(|a| Literal::byte_string(a.as_bytes()));
    let invalid_val_err = format!("field index 0 <= i < {len}");
    let struct_name = format!("struct {ident}");

    quote! {
        #(#wrapper_defs)*

        #[allow(non_camel_case_types)]
        enum #enum_field {
            #(#anon_field,)*
            ignore,
        }
        struct #field_visitor;
        impl<'de> atat::serde_at::serde::de::Visitor<'de> for #field_visitor {
            type Value = #enum_field;
            fn expecting(
                &self,
                formatter: &mut core::fmt::Formatter,
            ) -> core::fmt::Result {
                core::fmt::Formatter::write_str(formatter, "field identifier")
            }
            fn visit_u64<E>(
                self,
                value: u64,
            ) -> core::result::Result<Self::Value, E>
            where
                E: atat::serde_at::serde::de::Error,
            {
                match value {
                    #(#anon_field_ind64 => Ok(#enum_field::#anon_field),)*
                    _ => Err(<E as atat::serde_at::serde::de::Error>::invalid_value(
                        atat::serde_at::serde::de::Unexpected::Unsigned(value),
                        &#invalid_val_err,
                    )),
                }
            }

            fn visit_u128<E>(
                self,
                value: u128,
            ) -> core::result::Result<Self::Value, E>
            where
                E: atat::serde_at::serde::de::Error,
            {
                match value {
                    #(#anon_field_ind128 => Ok(#enum_field::#anon_field),)*
                    _ => Err(atat::serde_at::serde::de::Error::invalid_value(
                        atat::serde_at::serde::de::Unexpected::Other("u128"),
                        &#invalid_val_err,
                    )),
                }
            }
            fn visit_str<E>(
                self,
                value: &str,
            ) -> core::result::Result<Self::Value, E>
            where
                E: atat::serde_at::serde::de::Error,
            {
                Ok(match value {
                    #(
                        #field_names_str => #enum_field::#anon_field,
                    )*
                    _ => #enum_field::ignore,
                })
            }
            fn visit_bytes<E>(
                self,
                value: &[u8],
            ) -> core::result::Result<Self::Value, E>
            where
                E: atat::serde_at::serde::de::Error,
            {
                Ok(match value {
                    #(
                        #field_names_bytestr => #enum_field::#anon_field,
                    )*
                    _ => #enum_field::ignore,
                })
            }
        }


        impl<'de> atat::serde_at::serde::Deserialize<'de> for #enum_field {
            #[inline]
            fn deserialize<D>(
                deserializer: D,
            ) -> core::result::Result<Self, D::Error>
            where
                D: atat::serde_at::serde::Deserializer<'de>,
            {
                atat::serde_at::serde::Deserializer::deserialize_identifier(deserializer, #field_visitor)
            }
        }
        struct #visitor #serde_impl_generics {
            marker: core::marker::PhantomData<#ident #ty_generics>,
            lifetime: core::marker::PhantomData<&'de ()>,
        }
        impl #serde_impl_generics atat::serde_at::serde::de::Visitor<'de> for #visitor #serde_ty_generics {
            type Value = #ident #ty_generics;
            fn expecting(
                &self,
                formatter: &mut core::fmt::Formatter,
            ) -> core::fmt::Result {
                core::fmt::Formatter::write_str(formatter, #struct_name)
            }
            #[inline]
            fn visit_seq<A>(
                self,
                mut seq: A,
            ) -> core::result::Result<Self::Value, A::Error>
            where
                A: atat::serde_at::serde::de::SeqAccess<'de>,
            {
                #(
                    let #anon_field =
                        atat::serde_at::serde::de::SeqAccess::next_element::<#serde_types>(&mut seq)?.ok_or_else(||atat::serde_at::serde::de::Error::invalid_length(
                            #anon_field_ind,
                            &#invalid_len_err,
                        ))?;
                )*
                Ok(#ident {
                    #(
                        #field_names: #accessors
                    ),*
                })
            }
            #[inline]
            fn visit_map<A>(
                self,
                mut map: A,
            ) -> core::result::Result<Self::Value, A::Error>
            where
                A: atat::serde_at::serde::de::MapAccess<'de>,
            {
                #(
                    let mut #anon_field: Option<#serde_types> = None;
                )*
                while let Some(key) =
                    atat::serde_at::serde::de::MapAccess::next_key::<#enum_field>(&mut map)?
                {
                    match key {
                        #(
                            #enum_field::#anon_field => {
                                if Option::is_some(&#anon_field) {
                                    return Err(
                                        <A::Error as atat::serde_at::serde::de::Error>::duplicate_field(
                                            #field_names_str,
                                        ),
                                    );
                                }
                                #anon_field = Some(
                                    atat::serde_at::serde::de::MapAccess::next_value::<#serde_types>(&mut map)?
                                );
                            }
                        )*
                        _ => {
                            atat::serde_at::serde::de::MapAccess::next_value::<
                                atat::serde_at::serde::de::IgnoredAny,
                            >(&mut map)?;
                        }
                    }
                }
                #(
                    let #anon_field = #anon_field.ok_or_else(|| <A::Error as atat::serde_at::serde::de::Error>::missing_field(#field_names_str))?;
                )*
                Ok(#ident {
                    #(
                        #field_names: #accessors
                    ),*
                })
            }
        }
        const FIELDS: &'static [&'static str] = &[#(#field_names_str),*];
        atat::serde_at::serde::Deserializer::deserialize_struct(
            deserializer,
            #ident_str,
            FIELDS,
            #visitor {
                marker: core::marker::PhantomData::<#ident #ty_generics>,
                lifetime: core::marker::PhantomData,
            },
        )
    }
}
