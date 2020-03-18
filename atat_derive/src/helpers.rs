use crate::proc_macro2::{Literal, TokenStream, TokenTree};
use quote::quote;

use syn::{spanned::Spanned, Error, FieldsNamed, Ident, Result, Type};

pub fn stream_from_tokens(tokens: &proc_macro2::TokenStream) -> TokenStream {
    for f in tokens.clone() {
        if let TokenTree::Group(g) = f {
            return g.stream();
        }
    }
    panic!("Cannot find stream from tokens!");
}

pub fn get_lit(tokens: &proc_macro2::TokenStream) -> Result<Literal> {
    for l in stream_from_tokens(&tokens) {
        if let TokenTree::Literal(lit) = l {
            return Ok(lit);
        }
    }
    Err(Error::new(tokens.span(), "Cannot find AT Command!"))
}

pub fn get_ident(tokens: &proc_macro2::TokenStream) -> Result<Ident> {
    for l in stream_from_tokens(tokens) {
        if let TokenTree::Ident(ident) = l {
            return Ok(ident);
        }
    }
    Err(Error::new(tokens.span(), "Cannot find ident type!"))
}

pub fn get_name_ident_lit(tokens: &proc_macro2::TokenStream, needle: &str) -> Result<String> {
    let mut found = false;
    for l in stream_from_tokens(tokens) {
        match l {
            TokenTree::Ident(i) => {
                if i.to_string() == needle {
                    found = true;
                } else if found {
                    return Ok(i.to_string());
                }
            }
            TokenTree::Literal(lit) => {
                if found {
                    return Ok(lit.to_string());
                } else {
                    found = false;
                }
            }
            TokenTree::Punct(p) => {
                if p.to_string() == "=" && found {
                    found = true;
                } else {
                    found = false;
                }
            }
            _ => {
                found = false;
            }
        }
    }
    Err(Error::new(tokens.span(), "Cannot find literal type!"))
}

pub fn get_field_names(fields: Option<&FieldsNamed>) -> (Vec<Ident>, Vec<Type>, Vec<String>) {
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
                                match syn::parse_str::<syn::Lit>(
                                    &get_name_ident_lit(&attr.tokens, "position").unwrap(),
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
                                match syn::parse_str::<syn::Lit>(
                                    &get_name_ident_lit(&attr.tokens, "position").unwrap(),
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

fn get_string_length(ty: &Type) -> usize {
    let type_string = quote! { #ty }.to_string();
    let len = match type_string.replace(" ", "").as_str() {
        "str" => panic!("String slices must be annotated with a length using #[at_arg()]"),
        "tuple" => panic!("Tuples are not supported!"),
        "char" => "a".len(),
        "bool" => "false".len(),
        "isize" => format!("{:?}", std::isize::MAX).len(),
        "usize" => format!("{:?}", std::usize::MAX).len(),
        "u8" => format!("{:?}", std::u8::MAX).len(),
        "u16" => format!("{:?}", std::u16::MAX).len(),
        "u32" => format!("{:?}", std::u32::MAX).len(),
        "u64" => format!("{:?}", std::u64::MAX).len(),
        "u128" => format!("{:?}", std::u128::MAX).len(),
        "i8" => format!("{:?}", std::i8::MIN).len(),
        "i16" => format!("{:?}", std::i16::MIN).len(),
        "i32" => format!("{:?}", std::i32::MIN).len(),
        "i64" => format!("{:?}", std::i64::MIN).len(),
        "i128" => format!("{:?}", std::i128::MIN).len(),
        "f32" => format!("{:?}", std::f32::MIN).len(),
        "f64" => format!("{:?}", std::f64::MIN).len(),
        _ => {
            // println!("Unexpected type: {:?}", type_string);
            0
        }
    };

    // println!("Got len! {:?}: {:?}", type_string, len);
    len
}

fn next_power_of_2(mut n: usize) -> usize {
    n = n - 1;
    while (n & (n - 1)) != 0 {
        n = n & (n - 1);
    }
    return n << 1;
}

pub fn calculate_cmd_len(
    subcmd_len: usize,
    fields: Option<&FieldsNamed>,
    termination_len: usize,
) -> usize {
    let fields_len = if let Some(fields) = fields {
        fields
            .named
            .iter()
            .map(|field| get_string_length(&field.ty))
            .sum()
    } else {
        0
    };

    let total_len = fields_len + subcmd_len + termination_len;
    if total_len <= 1024 {
        total_len
    } else {
        next_power_of_2(total_len)
    }
}
