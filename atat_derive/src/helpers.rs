use crate::parse::Variant;
use proc_macro2::{Literal, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_quote, GenericParam, Generics, Ident, Lifetime, LifetimeDef, TypeParamBound};

/// Adds a single lifetime symbol eg. <'a>
#[inline]
pub fn add_lifetime(generics: &mut Generics, lifetime_symbol: &str) {
    generics
        .params
        .push(GenericParam::Lifetime(LifetimeDef::new(Lifetime::new(
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
        bounds: vec![trait_bound].iter().cloned().collect(),
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
    let visitor = format_ident!("{}Visitor", ident);
    let field_visitor = format_ident!("{}FieldVisitor", ident);
    let enum_field = format_ident!("{}Field", ident);
    let field_names_bytestr = field_names_str
        .iter()
        .map(|a| Literal::byte_string(a.as_bytes()));
    let invalid_len_err = format!("struct {} with {} elements", ident, len);
    let invalid_val_err = format!("field index {} <= i < {}", 0, len);
    let struct_name = format!("struct {}", ident);

    let (_, ty_generics, _) = generics.split_for_impl();
    let mut serde_generics = generics.clone();
    add_lifetime(&mut serde_generics, "'de");
    let (serde_impl_generics, serde_ty_generics, _) = serde_generics.split_for_impl();

    quote! {
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
                formatter: &mut atat::serde_at::serde::export::Formatter,
            ) -> atat::serde_at::serde::export::fmt::Result {
                atat::serde_at::serde::export::Formatter::write_str(formatter, "field identifier")
            }
            fn visit_u64<E>(
                self,
                value: u64,
            ) -> atat::serde_at::serde::export::Result<Self::Value, E>
            where
                E: atat::serde_at::serde::de::Error,
            {
                match value {
                    #(#anon_field_ind64 => atat::serde_at::serde::export::Ok(#enum_field::#anon_field),)*
                    _ => atat::serde_at::serde::export::Err(atat::serde_at::serde::de::Error::invalid_value(
                        atat::serde_at::serde::de::Unexpected::Unsigned(value),
                        &#invalid_val_err,
                    )),
                }
            }

            fn visit_u128<E>(
                self,
                value: u128,
            ) -> atat::serde_at::serde::export::Result<Self::Value, E>
            where
                E: atat::serde_at::serde::de::Error,
            {
                match value {
                    #(#anon_field_ind128 => atat::serde_at::serde::export::Ok(#enum_field::#anon_field),)*
                    _ => atat::serde_at::serde::export::Err(atat::serde_at::serde::de::Error::invalid_value(
                        atat::serde_at::serde::de::Unexpected::Other("u128"),
                        &#invalid_val_err,
                    )),
                }
            }
            fn visit_str<E>(
                self,
                value: &str,
            ) -> atat::serde_at::serde::export::Result<Self::Value, E>
            where
                E: atat::serde_at::serde::de::Error,
            {
                match value {
                    #(
                        #field_names_str => atat::serde_at::serde::export::Ok(#enum_field::#anon_field),
                    )*
                    _ => atat::serde_at::serde::export::Ok(#enum_field::ignore),
                }
            }
            fn visit_bytes<E>(
                self,
                value: &[u8],
            ) -> atat::serde_at::serde::export::Result<Self::Value, E>
            where
                E: atat::serde_at::serde::de::Error,
            {
                match value {
                    #(
                        #field_names_bytestr => atat::serde_at::serde::export::Ok(#enum_field::#anon_field),
                    )*
                    _ => atat::serde_at::serde::export::Ok(#enum_field::ignore),
                }
            }
        }


        impl<'de> atat::serde_at::serde::Deserialize<'de> for #enum_field {
            #[inline]
            fn deserialize<D>(
                deserializer: D,
            ) -> atat::serde_at::serde::export::Result<Self, D::Error>
            where
                D: atat::serde_at::serde::Deserializer<'de>,
            {
                atat::serde_at::serde::Deserializer::deserialize_identifier(deserializer, #field_visitor)
            }
        }
        struct #visitor #serde_impl_generics {
            marker: atat::serde_at::serde::export::PhantomData<#ident #ty_generics>,
            lifetime: atat::serde_at::serde::export::PhantomData<&'de ()>,
        }
        impl #serde_impl_generics atat::serde_at::serde::de::Visitor<'de> for #visitor #serde_ty_generics {
            type Value = #ident #ty_generics;
            fn expecting(
                &self,
                formatter: &mut atat::serde_at::serde::export::Formatter,
            ) -> atat::serde_at::serde::export::fmt::Result {
                atat::serde_at::serde::export::Formatter::write_str(formatter, #struct_name)
            }
            #[inline]
            fn visit_seq<A>(
                self,
                mut seq: A,
            ) -> atat::serde_at::serde::export::Result<Self::Value, A::Error>
            where
                A: atat::serde_at::serde::de::SeqAccess<'de>,
            {
                #(
                    let #anon_field =
                        match match atat::serde_at::serde::de::SeqAccess::next_element::<#field_types>(&mut seq) {
                            atat::serde_at::serde::export::Ok(val) => val,
                            atat::serde_at::serde::export::Err(err) => {
                                return atat::serde_at::serde::export::Err(err);
                            }
                        } {
                            atat::serde_at::serde::export::Some(value) => value,
                            atat::serde_at::serde::export::None => {
                                return atat::serde_at::serde::export::Err(atat::serde_at::serde::de::Error::invalid_length(
                                    #anon_field_ind,
                                    &#invalid_len_err,
                                ));
                            }
                        };
                )*
                atat::serde_at::serde::export::Ok(#ident {
                    #(
                        #field_names: #anon_field
                    ),*
                })
            }
            #[inline]
            fn visit_map<A>(
                self,
                mut map: A,
            ) -> atat::serde_at::serde::export::Result<Self::Value, A::Error>
            where
                A: atat::serde_at::serde::de::MapAccess<'de>,
            {
                #(
                    let mut #anon_field: atat::serde_at::serde::export::Option<#field_types> = atat::serde_at::serde::export::None;
                )*
                while let atat::serde_at::serde::export::Some(key) =
                    match atat::serde_at::serde::de::MapAccess::next_key::<#enum_field>(&mut map) {
                        atat::serde_at::serde::export::Ok(val) => val,
                        atat::serde_at::serde::export::Err(err) => {
                            return atat::serde_at::serde::export::Err(err);
                        }
                    }
                {
                    match key {
                        #(
                            #enum_field::#anon_field => {
                                if atat::serde_at::serde::export::Option::is_some(&#anon_field) {
                                    return atat::serde_at::serde::export::Err(
                                        <A::Error as atat::serde_at::serde::de::Error>::duplicate_field(
                                            #field_names_str,
                                        ),
                                    );
                                }
                                #anon_field = atat::serde_at::serde::export::Some(
                                    match atat::serde_at::serde::de::MapAccess::next_value::<#field_types>(&mut map) {
                                        atat::serde_at::serde::export::Ok(val) => val,
                                        atat::serde_at::serde::export::Err(err) => {
                                            return atat::serde_at::serde::export::Err(err);
                                        }
                                    },
                                );
                            }
                        )*
                        _ => {
                            let _ = match atat::serde_at::serde::de::MapAccess::next_value::<
                                atat::serde_at::serde::de::IgnoredAny,
                            >(&mut map)
                            {
                                atat::serde_at::serde::export::Ok(val) => val,
                                atat::serde_at::serde::export::Err(err) => {
                                    return atat::serde_at::serde::export::Err(err);
                                }
                            };
                        }
                    }
                }
                #(
                    let #anon_field = match #anon_field {
                        atat::serde_at::serde::export::Some(#anon_field) => #anon_field,
                        atat::serde_at::serde::export::None => {
                            match atat::serde_at::serde::private::de::missing_field(#field_names_str) {
                                atat::serde_at::serde::export::Ok(val) => val,
                                atat::serde_at::serde::export::Err(err) => {
                                    return atat::serde_at::serde::export::Err(err);
                                }
                            }
                        }
                    };
                )*
                atat::serde_at::serde::export::Ok(#ident {
                    #(
                        #field_names: #anon_field
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
                marker: atat::serde_at::serde::export::PhantomData::<#ident #ty_generics>,
                lifetime: atat::serde_at::serde::export::PhantomData,
            },
        )
    }
}
