#![feature(proc_macro_diagnostic)]

extern crate proc_macro;
extern crate proc_macro2;

use crate::proc_macro::TokenStream;
use crate::proc_macro2::{Literal, TokenTree};

use quote::{format_ident, quote};
use syn::{
    parse, spanned::Spanned, Attribute, Data, DataStruct, DeriveInput, Error, Fields, FieldsNamed,
    Ident, Result, Type,
};

#[proc_macro_derive(ATATResp, attributes(at_arg))]
pub fn derive_atat_resp(input: TokenStream) -> TokenStream {
    let item: DeriveInput = syn::parse(input).unwrap();

    match item.data {
        Data::Struct(struct_) => match struct_ {
            DataStruct {
                fields: Fields::Named(fields),
                ..
            } => generate_resp_output(&item.ident, Some(&fields)),
            DataStruct {
                fields: Fields::Unit,
                ..
            } => generate_resp_output(&item.ident, None),

            _ => panic!("Cannot handle unnamed struct fields"),
        },
        _ => {
            item.span()
                .unstable()
                .error("ATATResp can only be applied to structs!")
                .emit();
            TokenStream::new()
        }
    }
}

#[proc_macro_derive(ATATCmd, attributes(at_cmd, at_arg))]
pub fn derive_atat_cmd(input: TokenStream) -> TokenStream {
    let item: DeriveInput = parse(input).unwrap();

    match item.data {
        Data::Struct(struct_) => {
            let at_cmd_attr = get_cmd_response(&item.attrs).unwrap();
            match struct_ {
                DataStruct {
                    fields: Fields::Named(fields),
                    ..
                } => generate_cmd_output(&item.ident, &at_cmd_attr, Some(&fields)),
                DataStruct {
                    fields: Fields::Unit,
                    ..
                } => {
                    let at_cmd_attr = get_cmd_response(&item.attrs).unwrap();
                    generate_cmd_output(&item.ident, &at_cmd_attr, None)
                }
                _ => panic!("Cannot handle unnamed struct fields"),
            }
        }
        _ => {
            item.span()
                .unstable()
                .error("ATATCmd can only be applied to structs!")
                .emit();
            TokenStream::new()
        }
    }
}

fn generate_resp_output(name: &Ident, fields: Option<&FieldsNamed>) -> TokenStream {
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

    let output = quote! {
        impl ATATResp for #name {}

        impl<'de> ::serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> ::serde::export::Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum #enum_field {
                    #(#anon_field,)*
                    ignore,
                }
                struct #field_visitor;
                impl<'de> ::serde::de::Visitor<'de> for #field_visitor {
                    type Value = #enum_field;
                    fn expecting(
                        &self,
                        formatter: &mut ::serde::export::Formatter,
                    ) -> ::serde::export::fmt::Result {
                        ::serde::export::Formatter::write_str(formatter, "field identifier")
                    }
                    fn visit_u64<E>(
                        self,
                        value: u64,
                    ) -> ::serde::export::Result<Self::Value, E>
                    where
                        E: ::serde::de::Error,
                    {
                        match value {
                            #(#anon_field_ind64 => ::serde::export::Ok(#enum_field::#anon_field),)*
                            _ => ::serde::export::Err(::serde::de::Error::invalid_value(
                                ::serde::de::Unexpected::Unsigned(value),
                                &#invalid_val_err,
                            )),
                        }
                    }
                    fn visit_str<E>(
                        self,
                        value: &str,
                    ) -> ::serde::export::Result<Self::Value, E>
                    where
                        E: ::serde::de::Error,
                    {
                        match value {
                            #(
                                #field_names_str => ::serde::export::Ok(#enum_field::#anon_field),
                            )*
                            _ => ::serde::export::Ok(#enum_field::ignore),
                        }
                    }
                    fn visit_bytes<E>(
                        self,
                        value: &[u8],
                    ) -> ::serde::export::Result<Self::Value, E>
                    where
                        E: ::serde::de::Error,
                    {
                        match value {
                            #(
                                #field_names_bytestr => ::serde::export::Ok(#enum_field::#anon_field),
                            )*
                            _ => ::serde::export::Ok(#enum_field::ignore),
                        }
                    }
                }
                impl<'de> ::serde::Deserialize<'de> for #enum_field {
                    #[inline]
                    fn deserialize<D>(
                        deserializer: D,
                    ) -> ::serde::export::Result<Self, D::Error>
                    where
                        D: ::serde::Deserializer<'de>,
                    {
                        ::serde::Deserializer::deserialize_identifier(deserializer, #field_visitor)
                    }
                }
                struct #visitor<'de> {
                    marker: ::serde::export::PhantomData<#name>,
                    lifetime: ::serde::export::PhantomData<&'de ()>,
                }
                impl<'de> ::serde::de::Visitor<'de> for #visitor<'de> {
                    type Value = #name;
                    fn expecting(
                        &self,
                        formatter: &mut ::serde::export::Formatter,
                    ) -> ::serde::export::fmt::Result {
                        ::serde::export::Formatter::write_str(formatter, #struct_name)
                    }
                    #[inline]
                    fn visit_seq<A>(
                        self,
                        mut seq: A,
                    ) -> ::serde::export::Result<Self::Value, A::Error>
                    where
                        A: ::serde::de::SeqAccess<'de>,
                    {
                        #(
                            let #anon_field =
                                match match ::serde::de::SeqAccess::next_element::<#field_types>(&mut seq) {
                                    ::serde::export::Ok(val) => val,
                                    ::serde::export::Err(err) => {
                                        return ::serde::export::Err(err);
                                    }
                                } {
                                    ::serde::export::Some(value) => value,
                                    ::serde::export::None => {
                                        return ::serde::export::Err(::serde::de::Error::invalid_length(
                                            #anon_field_ind,
                                            &#invalid_len_err,
                                        ));
                                    }
                                };
                        )*
                        ::serde::export::Ok(#name {
                            #(
                                #field_names: #anon_field
                            ),*
                        })
                    }
                    #[inline]
                    fn visit_map<A>(
                        self,
                        mut map: A,
                    ) -> ::serde::export::Result<Self::Value, A::Error>
                    where
                        A: ::serde::de::MapAccess<'de>,
                    {
                        #(
                            let mut #anon_field: ::serde::export::Option<#field_types> = ::serde::export::None;
                        )*
                        while let ::serde::export::Some(key) =
                            match ::serde::de::MapAccess::next_key::<#enum_field>(&mut map) {
                                ::serde::export::Ok(val) => val,
                                ::serde::export::Err(err) => {
                                    return ::serde::export::Err(err);
                                }
                            }
                        {
                            match key {
                                #(
                                    #enum_field::#anon_field => {
                                        if ::serde::export::Option::is_some(&#anon_field) {
                                            return ::serde::export::Err(
                                                <A::Error as ::serde::de::Error>::duplicate_field(
                                                    #field_names_str,
                                                ),
                                            );
                                        }
                                        #anon_field = ::serde::export::Some(
                                            match ::serde::de::MapAccess::next_value::<#field_types>(&mut map) {
                                                ::serde::export::Ok(val) => val,
                                                ::serde::export::Err(err) => {
                                                    return ::serde::export::Err(err);
                                                }
                                            },
                                        );
                                    }
                                )*
                                _ => {
                                    let _ = match ::serde::de::MapAccess::next_value::<
                                        ::serde::de::IgnoredAny,
                                    >(&mut map)
                                    {
                                        ::serde::export::Ok(val) => val,
                                        ::serde::export::Err(err) => {
                                            return ::serde::export::Err(err);
                                        }
                                    };
                                }
                            }
                        }
                        #(
                            let #anon_field = match #anon_field {
                                ::serde::export::Some(#anon_field) => #anon_field,
                                ::serde::export::None => {
                                    match ::serde::private::de::missing_field(#field_names_str) {
                                        ::serde::export::Ok(val) => val,
                                        ::serde::export::Err(err) => {
                                            return ::serde::export::Err(err);
                                        }
                                    }
                                }
                            };
                        )*
                        ::serde::export::Ok(#name {
                            #(
                                #field_names: #anon_field
                            ),*
                        })
                    }
                }
                const FIELDS: &'static [&'static str] = &[#(#field_names_str),*];
                ::serde::Deserializer::deserialize_struct(
                    deserializer,
                    #name_str,
                    FIELDS,
                    #visitor {
                        marker: ::serde::export::PhantomData::<#name>,
                        lifetime: ::serde::export::PhantomData,
                    },
                )
            }
        }
    };
    output.into()
}

#[derive(Debug)]
struct AtCmdAttr {
    cmd: Literal,
    resp: Ident,
    timeout_ms: Option<u32>,
    abortable: Option<bool>,
}

fn get_lit(tokens: proc_macro2::TokenStream) -> Result<Literal> {
    for f in tokens.clone() {
        if let TokenTree::Group(g) = f {
            for l in g.stream() {
                match l {
                    TokenTree::Literal(lit) => return Ok(lit),
                    _ => (),
                }
            }
        }
    }
    Err(Error::new(tokens.span(), "Cannot find AT Command!"))
}

fn get_cmd_response(attrs: &Vec<Attribute>) -> Result<AtCmdAttr> {
    fn get_ident(tokens: proc_macro2::TokenStream) -> Result<Ident> {
        for f in tokens.clone() {
            if let TokenTree::Group(g) = f {
                for l in g.stream() {
                    match l {
                        TokenTree::Ident(ident) => return Ok(ident),
                        _ => (),
                    }
                }
            }
        }
        Err(Error::new(tokens.span(), "Cannot find response type!"))
    }

    if let Some(attr) = attrs.iter().find(|attr| attr.path.is_ident("at_cmd")) {
        Ok(AtCmdAttr {
            cmd: get_lit(attr.tokens.clone())?,
            resp: get_ident(attr.tokens.clone())?,
            timeout_ms: None,
            abortable: None,
        })
    } else {
        panic!("Failed to find non-optional at_cmd attribute!",)
    }
}

fn get_field_names(fields: Option<&FieldsNamed>) -> (Vec<Ident>, Vec<Type>, Vec<String>) {
    if let Some(fields) = fields {
        let (mut field_name_pos, mut field_type_pos): (Vec<(Ident, usize)>, Vec<(Type, usize)>) = {
            (
                fields
                    .named
                    .iter()
                    .map(|field| {
                        (
                            field.ident.clone().unwrap(),
                            if let Some(attr) =
                                field.attrs.iter().find(|attr| attr.path.is_ident("at_arg"))
                            {
                                // TODO: Only find position attribute!
                                match syn::parse_str::<syn::Lit>(
                                    &get_lit(attr.tokens.clone()).unwrap().to_string(),
                                )
                                .unwrap()
                                {
                                    syn::Lit::Int(l) => l.base10_parse::<usize>().unwrap(),
                                    _ => panic!("Position argument must be an integer!"),
                                }
                            } else {
                                0
                            },
                        )
                    })
                    .collect(),
                fields
                    .named
                    .iter()
                    .map(|field| {
                        (
                            field.ty.clone(),
                            if let Some(attr) =
                                field.attrs.iter().find(|attr| attr.path.is_ident("at_arg"))
                            {
                                // TODO: Only find position attribute!
                                match syn::parse_str::<syn::Lit>(
                                    &get_lit(attr.tokens.clone()).unwrap().to_string(),
                                )
                                .unwrap()
                                {
                                    syn::Lit::Int(l) => l.base10_parse::<usize>().unwrap(),
                                    _ => panic!("Position argument must be an integer!"),
                                }
                            } else {
                                0
                            },
                        )
                    })
                    .collect(),
            )
        };

        field_name_pos.sort_by(|(_, a), (_, b)| a.cmp(b));
        field_type_pos.sort_by(|(_, a), (_, b)| a.cmp(b));
        let (field_name, _): (Vec<Ident>, Vec<usize>) = field_name_pos.iter().cloned().unzip();
        let (field_type, _): (Vec<Type>, Vec<usize>) = field_type_pos.iter().cloned().unzip();

        let field_name_str: Vec<String> = field_name.iter().map(|n| n.to_string()).collect();

        (field_name, field_type, field_name_str)
    } else {
        (vec![], vec![], vec![])
    }
}

fn generate_cmd_output(
    name: &Ident,
    attr: &AtCmdAttr,
    fields: Option<&FieldsNamed>,
) -> TokenStream {
    let name_str = &name.to_string();
    let cmd = &attr.cmd;
    let response = &attr.resp;

    let (field_names, _, field_names_str) = get_field_names(fields);
    let len = field_names.len();

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

    let output = quote! {
        impl ::atat::ATATCmd for #name {
            type Response = #response;
            type CommandLen = ::heapless::consts::U64;

            fn as_str(&self) -> ::heapless::String<Self::CommandLen> {
                let s: ::heapless::String<Self::CommandLen> = ::heapless::String::from(#cmd);
                ::serde_at::to_string(self, s).unwrap()
            }

            fn parse(&self, resp: &str) -> core::result::Result<#response, ::atat::Error> {
                ::serde_at::from_str::<#response>(resp).map_err(|e| ::atat::Error::InvalidResponse)
            }

            #timeout

            #abortable
        }

        impl ::serde::Serialize for #name {
            fn serialize<S>(
                &self,
                serializer: S,
            ) -> ::serde::export::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                let mut serde_state = match ::serde::Serializer::serialize_struct(
                    serializer,
                    #name_str,
                    #len,
                ) {
                    ::serde::export::Ok(val) => val,
                    ::serde::export::Err(err) => {
                        return ::serde::export::Err(err);
                    }
                };

                #(
                    match ::serde::ser::SerializeStruct::serialize_field(
                        &mut serde_state,
                        #field_names_str,
                        &self.#field_names,
                    ) {
                        ::serde::export::Ok(val) => val,
                        ::serde::export::Err(err) => {
                            return ::serde::export::Err(err);
                        }
                    };
                )*

                ::serde::ser::SerializeStruct::end(serde_state)
            }
        }

        #[cfg(feature = "use_ufmt")]
        impl ::ufmt::uDebug for #name {
            fn fmt<W>(&self, f: &mut ::ufmt::Formatter<'_, W>) -> core::result::Result<(), W::Error>
            where
                W: ::ufmt::uWrite + ?Sized,
            {
                f.write_str(&self.as_str())
            }
        }

        #[cfg(feature = "use_ufmt")]
        impl ::ufmt::uDisplay for #name {
            fn fmt<W>(&self, f: &mut ::ufmt::Formatter<'_, W>) -> core::result::Result<(), W::Error>
            where
                W: ::ufmt::uWrite + ?Sized,
            {
                let c = self.as_str();
                f.write_str(&c[0..c.len() - 2])
            }
        }
    };
    output.into()
}
