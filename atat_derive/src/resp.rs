use crate::proc_macro::TokenStream;
use crate::proc_macro2::{Literal, Span};

use quote::{format_ident, quote};
use syn::{
    Data, DataStruct, DeriveInput, Fields, FieldsNamed, GenericParam, Ident, Lifetime, LifetimeDef,
};

use crate::helpers::get_field_names;

pub fn atat_resp(item: DeriveInput) -> TokenStream {
    match item.data {
        Data::Struct(struct_) => match struct_ {
            DataStruct {
                fields: Fields::Named(fields),
                ..
            } => generate_resp_output(&item.ident, &item.generics, Some(&fields)),
            DataStruct {
                fields: Fields::Unit,
                ..
            } => generate_resp_output(&item.ident, &item.generics, None),

            _ => panic!("Cannot handle unnamed struct fields"),
        },
        _ => {
            // item.span()
            //     .unstable()
            //     .error("ATATResp can only be applied to structs!")
            //     .emit();
            // TokenStream::new()
            panic!("ATATResp can only be applied to structs!");
        }
    }
}

fn generate_resp_output(
    name: &Ident,
    generics: &syn::Generics,
    fields: Option<&FieldsNamed>,
) -> TokenStream {
    let name_str = &name.to_string();
    let (field_names, field_types, field_names_str) = get_field_names(fields);
    let (anon_field_ind, anon_field): (Vec<usize>, Vec<Ident>) = field_names
        .iter()
        .enumerate()
        .map(|(i, _)| (i, format_ident!("field{}", i)))
        .unzip();
    let anon_field_ind64: Vec<u64> = anon_field_ind.iter().map(|i| *i as u64).collect();
    let len = field_names.len();
    let visitor = format_ident!("{}Visitor", name_str);
    let field_visitor = format_ident!("{}FieldVisitor", name_str);
    let enum_field = format_ident!("{}Field", name_str);
    let field_names_bytestr = field_names_str
        .iter()
        .map(|a| Literal::byte_string(a.as_bytes()));
    let invalid_len_err = format!("struct {} with {} elements", name_str, len);
    let invalid_val_err = format!("field index {} <= i < {}", 0, len);
    let struct_name = format!("struct {}", name);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut serde_generics = generics.clone();

    serde_generics
        .params
        .push(GenericParam::Lifetime(LifetimeDef::new(Lifetime::new(
            "'de",
            Span::call_site(),
        ))));
    let (serde_impl_generics, _, _) = serde_generics.split_for_impl();

    TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_generics atat::ATATResp for #name #ty_generics #where_clause {}

        #[automatically_derived]
        impl #serde_impl_generics serde::Deserialize<'de> for #name #ty_generics #where_clause {
            fn deserialize<D>(deserializer: D) -> serde::export::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum #enum_field {
                    #(#anon_field,)*
                    ignore,
                }
                struct #field_visitor;
                impl<'de> serde::de::Visitor<'de> for #field_visitor {
                    type Value = #enum_field;
                    fn expecting(
                        &self,
                        formatter: &mut serde::export::Formatter,
                    ) -> serde::export::fmt::Result {
                        serde::export::Formatter::write_str(formatter, "field identifier")
                    }
                    fn visit_u64<E>(
                        self,
                        value: u64,
                    ) -> serde::export::Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            #(#anon_field_ind64 => serde::export::Ok(#enum_field::#anon_field),)*
                            _ => serde::export::Err(::serde::de::Error::invalid_value(
                                serde::de::Unexpected::Unsigned(value),
                                &#invalid_val_err,
                            )),
                        }
                    }
                    fn visit_str<E>(
                        self,
                        value: &str,
                    ) -> serde::export::Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            #(
                                #field_names_str => serde::export::Ok(#enum_field::#anon_field),
                            )*
                            _ => serde::export::Ok(#enum_field::ignore),
                        }
                    }
                    fn visit_bytes<E>(
                        self,
                        value: &[u8],
                    ) -> serde::export::Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            #(
                                #field_names_bytestr => serde::export::Ok(#enum_field::#anon_field),
                            )*
                            _ => serde::export::Ok(#enum_field::ignore),
                        }
                    }
                }


                impl<'de> serde::Deserialize<'de> for #enum_field {
                    #[inline]
                    fn deserialize<D>(
                        deserializer: D,
                    ) -> serde::export::Result<Self, D::Error>
                    where
                        D: serde::Deserializer<'de>,
                    {
                        serde::Deserializer::deserialize_identifier(deserializer, #field_visitor)
                    }
                }
                struct #visitor<'de> {
                    marker: serde::export::PhantomData<#name>,
                    lifetime: serde::export::PhantomData<&'de ()>,
                }
                impl<'de> serde::de::Visitor<'de> for #visitor<'de> {
                    type Value = #name;
                    fn expecting(
                        &self,
                        formatter: &mut serde::export::Formatter,
                    ) -> serde::export::fmt::Result {
                        serde::export::Formatter::write_str(formatter, #struct_name)
                    }
                    #[inline]
                    fn visit_seq<A>(
                        self,
                        mut seq: A,
                    ) -> serde::export::Result<Self::Value, A::Error>
                    where
                        A: serde::de::SeqAccess<'de>,
                    {
                        #(
                            let #anon_field =
                                match match serde::de::SeqAccess::next_element::<#field_types>(&mut seq) {
                                    serde::export::Ok(val) => val,
                                    serde::export::Err(err) => {
                                        return serde::export::Err(err);
                                    }
                                } {
                                    serde::export::Some(value) => value,
                                    serde::export::None => {
                                        return serde::export::Err(::serde::de::Error::invalid_length(
                                            #anon_field_ind,
                                            &#invalid_len_err,
                                        ));
                                    }
                                };
                        )*
                        serde::export::Ok(#name {
                            #(
                                #field_names: #anon_field
                            ),*
                        })
                    }
                    #[inline]
                    fn visit_map<A>(
                        self,
                        mut map: A,
                    ) -> serde::export::Result<Self::Value, A::Error>
                    where
                        A: serde::de::MapAccess<'de>,
                    {
                        #(
                            let mut #anon_field: serde::export::Option<#field_types> = serde::export::None;
                        )*
                        while let serde::export::Some(key) =
                            match serde::de::MapAccess::next_key::<#enum_field>(&mut map) {
                                serde::export::Ok(val) => val,
                                serde::export::Err(err) => {
                                    return serde::export::Err(err);
                                }
                            }
                        {
                            match key {
                                #(
                                    #enum_field::#anon_field => {
                                        if serde::export::Option::is_some(&#anon_field) {
                                            return serde::export::Err(
                                                <A::Error as serde::de::Error>::duplicate_field(
                                                    #field_names_str,
                                                ),
                                            );
                                        }
                                        #anon_field = serde::export::Some(
                                            match serde::de::MapAccess::next_value::<#field_types>(&mut map) {
                                                serde::export::Ok(val) => val,
                                                serde::export::Err(err) => {
                                                    return serde::export::Err(err);
                                                }
                                            },
                                        );
                                    }
                                )*
                                _ => {
                                    let _ = match serde::de::MapAccess::next_value::<
                                        serde::de::IgnoredAny,
                                    >(&mut map)
                                    {
                                        serde::export::Ok(val) => val,
                                        serde::export::Err(err) => {
                                            return serde::export::Err(err);
                                        }
                                    };
                                }
                            }
                        }
                        #(
                            let #anon_field = match #anon_field {
                                serde::export::Some(#anon_field) => #anon_field,
                                serde::export::None => {
                                    match serde::private::de::missing_field(#field_names_str) {
                                        serde::export::Ok(val) => val,
                                        serde::export::Err(err) => {
                                            return serde::export::Err(err);
                                        }
                                    }
                                }
                            };
                        )*
                        serde::export::Ok(#name {
                            #(
                                #field_names: #anon_field
                            ),*
                        })
                    }
                }
                const FIELDS: &'static [&'static str] = &[#(#field_names_str),*];
                serde::Deserializer::deserialize_struct(
                    deserializer,
                    #name_str,
                    FIELDS,
                    #visitor {
                        marker: serde::export::PhantomData::<#name>,
                        lifetime: serde::export::PhantomData,
                    },
                )
            }
        }
    })
}
