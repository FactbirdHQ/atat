use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_quote, Fields, Ident};

use crate::{
    helpers,
    parse::{ArgAttributes, EnumAttributes, ParseInput},
};

struct AnonymousEnum {
    ident: Ident,
    fields: Vec<Ident>,
}

struct Info {
    serialize_match_arms: Vec<proc_macro2::TokenStream>,
    anonymous_enum: AnonymousEnum,
    identifier_match_arms: Vec<proc_macro2::TokenStream>,
    deserialize_match_arms: Vec<proc_macro2::TokenStream>,
}

pub fn atat_enum(input: TokenStream) -> TokenStream {
    let ParseInput {
        ident,
        at_enum,
        variants,
        generics,
        ..
    } = parse_macro_input!(input as ParseInput);

    let repr = at_enum
        .unwrap_or_else(|| EnumAttributes {
            repr: format_ident!("u8"),
        })
        .repr;
    let ident_str = ident.to_string();

    let variant_names_str: Vec<_> = variants
        .iter()
        .map(|f| f.ident.clone().unwrap().to_string())
        .collect();

    let anon_enum = format_ident!("{}Field", ident);

    let mut info = Info {
        serialize_match_arms: Vec::new(),
        anonymous_enum: AnonymousEnum {
            ident: anon_enum.clone(),
            fields: Vec::new(),
        },
        identifier_match_arms: Vec::new(),
        deserialize_match_arms: Vec::new(),
    };
    let len = variants.len();

    let visitor = format_ident!("{}Visitor", ident);
    let field_visitor = format_ident!("{}FieldVisitor", ident);
    let invalid_val_err = format!("field index {} <= i < {}", 0, len);
    let enum_name = format!("enum {}", ident);

    let mut deserialize_generics = syn::Generics::default();
    let mut serialize_generics = syn::Generics::default();
    let mut atat_len_generics = syn::Generics::default();

    helpers::add_lifetime(&mut deserialize_generics, "'de");
    for lt in generics.lifetimes() {
        helpers::add_lifetime_bound(&mut deserialize_generics, &lt.lifetime);
        helpers::add_lifetime_bound(&mut serialize_generics, &lt.lifetime);
        helpers::add_lifetime_bound(&mut atat_len_generics, &lt.lifetime);
    }
    for tp in generics.type_params() {
        helpers::add_type_parameter_bound(
            &mut deserialize_generics,
            tp.clone(),
            parse_quote!(serde::Deserialize<'de>),
        );
        helpers::add_type_parameter_bound(
            &mut serialize_generics,
            tp.clone(),
            parse_quote!(serde::Serialize),
        );
        helpers::add_type_parameter_bound(
            &mut atat_len_generics,
            tp.clone(),
            parse_quote!(atat::AtatLen),
        );
    }

    let (_, ty_generics, _) = generics.split_for_impl();
    let (deserialize_impl_generics, deserialize_ty_generics, deserialize_where_clause) =
        deserialize_generics.split_for_impl();
    let (serialize_impl_generics, serialize_ty_generics, serialize_where_clause) =
        serialize_generics.split_for_impl();

    for (i, variant) in variants.iter().enumerate() {
        let variant_ident = variant.ident.clone().unwrap();
        let variant_ident_str = variant_ident.to_string();
        let val = if let Some(ArgAttributes { value: Some(v), .. }) = variant.attrs.at_arg {
            quote! { #v }
        } else {
            quote! { #ident::#variant_ident }
        };

        let anon_ident = format_ident!("_Field{}", i);

        info.identifier_match_arms.push(quote! {
            a if a == #val as i64  => atat::serde_at::serde::export::Ok(#anon_enum::#anon_ident)
        });

        // TODO: Catch error when using struct/tuple variants, and not defining
        // `#[at_arg(value = )]`
        // TODO: Should these handle attributes, eg for AtatLen?
        match variant.fields.clone().unwrap() {
            Fields::Named(f) => {
                let (field_ident, field_str): (Vec<_>, Vec<_>) = f
                    .named
                    .iter()
                    .map(|field| {
                        let ident = field.ident.clone().unwrap();
                        (ident.clone(), ident.to_string())
                    })
                    .unzip();

                // info.deserialize_match_arms.push(quote! {(#anon_enum::anon_ident, __variant) => atat::serde_at::serde::export::Ok(#ident::#variant_ident)});
                // helpers::deserialize_struct(ident.clone(), Vec::new(), &generics);

                info.serialize_match_arms.push(quote! {
                    #ident::#variant_ident { #(ref #field_ident),* } => {
                        let mut serde_state = atat::serde_at::serde::ser::Serializer::serialize_struct_variant(serializer, #ident_str, #val as u32, #variant_ident_str, 0)?;
                        #(
                            atat::serde_at::serde::ser::SerializeStructVariant::serialize_field(
                                &mut serde_state,
                                #field_str,
                                #field_ident,
                            )?;
                        )*
                        atat::serde_at::serde::ser::SerializeStructVariant::end(serde_state)
                    }
                });
            }
            Fields::Unnamed(f) => {
                let (anon_fields, field_ty): (Vec<_>, Vec<_>) = f
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, field)| (format_ident!("_field{}", i), field.ty.clone()))
                    .unzip();
                let variant_fields_len = anon_fields.len();

                info.deserialize_match_arms.push(quote! {
                    (#anon_enum::#anon_ident, __variant) => {
                        struct __Visitor #deserialize_impl_generics #deserialize_where_clause {
                            marker: atat::serde_at::serde::export::PhantomData<#ident #ty_generics>,
                            lifetime: atat::serde_at::serde::export::PhantomData<&'de ()>,
                        }
                        impl #deserialize_impl_generics atat::serde_at::serde::de::Visitor<'de> for __Visitor #deserialize_ty_generics #deserialize_where_clause {
                            type Value = #ident #ty_generics;
                            #[inline]
                            fn expecting(
                                &self,
                                formatter: &mut atat::serde_at::serde::export::Formatter,
                            ) -> atat::serde_at::serde::export::fmt::Result {
                                atat::serde_at::serde::export::Formatter::write_str(formatter, "tuple variant")
                            }

                            #[inline]
                            fn visit_seq<__A>(
                                self,
                                mut __seq: __A,
                            ) -> atat::serde_at::serde::export::Result<Self::Value, __A::Error>
                            where
                                __A: atat::serde_at::serde::de::SeqAccess<'de>,
                            {

                                #(
                                    let #anon_fields = match atat::serde_at::serde::de::SeqAccess::next_element::<#field_ty>(
                                        &mut __seq,
                                    )? {
                                        atat::serde_at::serde::export::Some(__value) => __value,
                                        atat::serde_at::serde::export::None => {
                                            return atat::serde_at::serde::export::Err(serde::de::Error::invalid_length(0usize, &"tuple variant tester::tupleTwo with 3 elements"));
                                        }
                                    };
                                )*
                                atat::serde_at::serde::export::Ok(#ident::#variant_ident(
                                    #(#anon_fields),*
                                ))
                            }
                        }


                        atat::serde_at::serde::de::VariantAccess::tuple_variant(__variant, #variant_fields_len, __Visitor {
                            marker: atat::serde_at::serde::export::PhantomData::<#ident #ty_generics>,
                            lifetime: atat::serde_at::serde::export::PhantomData,
                        })
                    }
                });

                info.serialize_match_arms.push(quote! {
                    #ident::#variant_ident ( #(ref #anon_fields),* ) => {
                        let mut serde_state = atat::serde_at::serde::ser::Serializer::serialize_tuple_variant(serializer, #ident_str, #val as u32, #variant_ident_str, 0)?;
                        #(
                            atat::serde_at::serde::ser::SerializeTupleVariant::serialize_field(
                                &mut serde_state,
                                #anon_fields,
                            )?;
                        )*
                        atat::serde_at::serde::ser::SerializeTupleVariant::end(serde_state)
                    }
                });
            }
            Fields::Unit => {
                info.deserialize_match_arms.push(quote! {
                    (#anon_enum::#anon_ident, __variant) => atat::serde_at::serde::export::Ok(#ident::#variant_ident)
                });

                info.serialize_match_arms.push(quote! {
                    #ident::#variant_ident => atat::serde_at::serde::Serialize::serialize(&(#val as #repr), serializer)
                });
            }
        }
        info.anonymous_enum.fields.push(anon_ident);
    }

    let enum_len = crate::len::enum_len(variants, &repr, &mut atat_len_generics);

    let Info {
        serialize_match_arms,
        anonymous_enum,
        identifier_match_arms,
        deserialize_match_arms,
    } = info;

    let AnonymousEnum {
        ident: anon_ident,
        fields: anon_fields,
    } = anonymous_enum;

    let (atat_len_impl_generics, atat_len_ty_generics, atat_len_where_clause) =
        atat_len_generics.split_for_impl();

    TokenStream::from(quote! {
        use atat::serde_at::serde;

        #[automatically_derived]
        impl #atat_len_impl_generics atat::AtatLen for #ident #atat_len_ty_generics #atat_len_where_clause {
            type Len = #enum_len;
        }

        #[automatically_derived]
        impl #serialize_impl_generics atat::serde_at::serde::Serialize for #ident #serialize_ty_generics #serialize_where_clause {
            #[inline]
            fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
            where
                S: atat::serde_at::serde::Serializer
            {
                match *self {
                    #(#serialize_match_arms),*
                }
            }
        }

        #[automatically_derived]
        impl #deserialize_impl_generics atat::serde_at::serde::Deserialize<'de> for #ident #ty_generics #deserialize_where_clause {
            fn deserialize<D>(deserializer: D) -> atat::serde_at::serde::export::Result<Self, D::Error>
            where
                D: atat::serde_at::serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum #anon_ident {
                    #(#anon_fields,)*
                }
                struct #field_visitor;
                impl<'de> atat::serde_at::serde::de::Visitor<'de> for #field_visitor {
                    type Value = #anon_ident;
                    #[inline]
                    fn expecting(
                        &self,
                        formatter: &mut atat::serde_at::serde::export::Formatter,
                    ) -> atat::serde_at::serde::export::fmt::Result {
                        atat::serde_at::serde::export::Formatter::write_str(formatter, "variant identifier")
                    }
                    #[inline]
                    fn visit_i64<E>(
                        self,
                        value: i64,
                    ) -> atat::serde_at::serde::export::Result<Self::Value, E>
                    where
                        E: atat::serde_at::serde::de::Error,
                    {
                        match value {
                            #(#identifier_match_arms,)*
                            _ => atat::serde_at::serde::export::Err(serde::de::Error::invalid_value(
                                atat::serde_at::serde::de::Unexpected::Signed(value),
                                &#invalid_val_err,
                            )),
                        }
                    }
                }


                impl<'de> atat::serde_at::serde::Deserialize<'de> for #anon_ident {
                    #[inline]
                    fn deserialize<D>(
                        deserializer: D,
                    ) -> atat::serde_at::serde::export::Result<Self, D::Error>
                    where
                        D: atat::serde_at::serde::Deserializer<'de>,
                    {
                        atat::serde_at::serde::Deserializer::deserialize_i64(deserializer, #field_visitor)
                    }
                }
                struct #visitor #deserialize_impl_generics #deserialize_where_clause {
                    marker: atat::serde_at::serde::export::PhantomData<#ident #ty_generics>,
                    lifetime: atat::serde_at::serde::export::PhantomData<&'de ()>,
                }
                impl #deserialize_impl_generics atat::serde_at::serde::de::Visitor<'de> for #visitor #deserialize_ty_generics #deserialize_where_clause {
                    type Value = #ident #ty_generics;
                    fn expecting(
                        &self,
                        formatter: &mut atat::serde_at::serde::export::Formatter,
                    ) -> atat::serde_at::serde::export::fmt::Result {
                        atat::serde_at::serde::export::Formatter::write_str(formatter, #enum_name)
                    }
                    #[inline]
                    fn visit_enum<A>(
                        self,
                        __data: A,
                    ) -> atat::serde_at::serde::export::Result<Self::Value, A::Error>
                    where
                        A: atat::serde_at::serde::de::EnumAccess<'de>,
                    {
                        match atat::serde_at::serde::de::EnumAccess::variant(__data)? {
                            #(#deserialize_match_arms,)*
                            _ => atat::serde_at::serde::export::Err(serde::de::Error::unknown_variant("__variant", VARIANTS)),
                        }
                    }
                }
                const VARIANTS: &'static [&'static str] = &[#(#variant_names_str),*];
                atat::serde_at::serde::Deserializer::deserialize_enum(
                    deserializer,
                    #ident_str,
                    VARIANTS,
                    #visitor {
                        marker: atat::serde_at::serde::export::PhantomData::<#ident #ty_generics>,
                        lifetime: atat::serde_at::serde::export::PhantomData,
                    },
                )
            }
        }
    })
}
