use proc_macro2::Span;
use syn::parse::{Error, Parse, ParseStream, Parser, Result};
use syn::{
    parenthesized, Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields, Generics, Ident,
    Lit, LitByteStr, Path, Type,
};

#[derive(Clone)]
pub struct ParseInput {
    pub ident: Ident,
    pub generics: Generics,
    pub at_cmd: Option<CmdAttributes>,
    pub at_enum: Option<EnumAttributes>,
    pub variants: Vec<Variant>,
}

/// Parsed attributes of `#[at_cmd(..)]`
#[derive(Clone)]
pub struct CmdAttributes {
    pub cmd: String,
    pub resp: Path,
    pub timeout_ms: Option<u32>,
    pub abortable: Option<bool>,
    pub value_sep: bool,
    pub cmd_prefix: String,
    pub termination: String,
}
/// Parsed attributes of `#[at_arg(..)]`
#[derive(Clone)]
pub struct ArgAttributes {
    pub value: Option<i64>,
    pub position: Option<usize>,
    pub len: Option<usize>,
    pub default: bool,
}

/// Parsed attributes of `#[at_urc(..)]`
#[derive(Clone)]
pub struct UrcAttributes {
    pub code: LitByteStr,
}

/// Parsed attributes of `#[at_enum(..)]`
#[derive(Clone)]
pub struct EnumAttributes {
    pub repr: Ident,
}

/// Parsed field level attributes
#[derive(Clone)]
pub struct FieldAttributes {
    pub at_urc: Option<UrcAttributes>,
    pub at_arg: Option<ArgAttributes>,
}

#[derive(Clone)]
pub struct Variant {
    /// Ident will be set on named variants, and None on unnamed variants
    pub ident: Option<Ident>,
    /// Type of a struct variant
    pub ty: Option<Type>,
    /// Fields of an enum variant
    pub fields: Option<Fields>,
    /// Parsed contents on `#[at_arg(..)]` and `#[at_urc(..)]`
    pub attrs: FieldAttributes,
}

/// Parse valid field attributes
pub fn parse_field_attr(attributes: &[Attribute]) -> Result<FieldAttributes> {
    let mut attrs = FieldAttributes {
        at_urc: None,
        at_arg: None,
    };
    for attr in attributes {
        if attr.path.is_ident("at_arg") {
            fn at_arg(input: ParseStream) -> Result<ArgAttributes> {
                let content;
                parenthesized!(content in input);
                content.parse()
            }
            attrs.at_arg = Some(at_arg.parse2(attr.tokens.clone())?);
        }
        if attr.path.is_ident("at_urc") {
            fn at_urc(input: ParseStream) -> Result<UrcAttributes> {
                let content;
                parenthesized!(content in input);
                content.parse()
            }
            attrs.at_urc = Some(at_urc.parse2(attr.tokens.clone())?);
        }
    }
    Ok(attrs)
}

fn sorted_variants(data: Data) -> Result<Vec<Variant>> {
    let mut variants = match data {
        Data::Struct(DataStruct { fields, .. }) => {
            let unwrapped_fields = match fields {
                Fields::Named(fields) => fields.named.iter().cloned().collect(),
                Fields::Unnamed(fields) => fields.unnamed.iter().cloned().collect(),
                Fields::Unit => Vec::new(),
            };

            unwrapped_fields
                .into_iter()
                .enumerate()
                .map(|(i, f)| {
                    Ok((
                        i,
                        Variant {
                            ident: f.ident,
                            ty: Some(f.ty),
                            fields: None,
                            attrs: parse_field_attr(&f.attrs)?,
                        },
                    ))
                })
                .collect::<Result<Vec<(usize, Variant)>>>()?
        }
        Data::Enum(DataEnum { variants, .. }) => variants
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                Ok((
                    i,
                    Variant {
                        ident: Some(v.ident.clone()),
                        ty: None,
                        fields: Some(v.fields.clone()),
                        attrs: parse_field_attr(&v.attrs)?,
                    },
                ))
            })
            .collect::<Result<Vec<(usize, Variant)>>>()?,
        Data::Union(_) => {
            return Err(Error::new(Span::call_site(), "union types are unsupported"));
        }
    };

    variants.sort_by(|(ai, a), (bi, b)| {
        let ap = if let Some(ArgAttributes {
            position: Some(p), ..
        }) = a.attrs.at_arg
        {
            p
        } else {
            *ai
        };

        let bp = if let Some(ArgAttributes {
            position: Some(p), ..
        }) = b.attrs.at_arg
        {
            p
        } else {
            *bi
        };

        ap.cmp(&bp)
    });

    Ok(variants.into_iter().map(|t| t.1).collect())
}

impl Parse for ArgAttributes {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs = Self {
            value: None,
            position: None,
            len: None,
            default: false,
        };

        while {
            match input.parse::<syn::Meta>()? {
                syn::Meta::NameValue(name_value) if name_value.path.is_ident("value") => {
                    match name_value.lit.clone() {
                        Lit::Int(v) => attrs.value = Some(v.base10_parse().unwrap()),
                        _ => {
                            return Err(Error::new(
                                Span::call_site(),
                                "value argument must be an integer",
                            ))
                        }
                    }
                }
                syn::Meta::NameValue(name_value) if name_value.path.is_ident("position") => {
                    match name_value.lit.clone() {
                        Lit::Int(v) => attrs.position = Some(v.base10_parse().unwrap()),
                        _ => {
                            return Err(Error::new(
                                Span::call_site(),
                                "position argument must be a positive integer",
                            ))
                        }
                    }
                }
                syn::Meta::NameValue(name_value) if name_value.path.is_ident("len") => {
                    match name_value.lit.clone() {
                        Lit::Int(v) => attrs.len = Some(v.base10_parse().unwrap()),
                        _ => {
                            return Err(Error::new(
                                Span::call_site(),
                                "len argument must be a positive integer",
                            ))
                        }
                    }
                }
                syn::Meta::NameValue(name_value) if name_value.path.is_ident("default") => {
                    return Err(Error::new(
                        Span::call_site(),
                        "default does not have a value. Eg #[at_arg(default)]",
                    ))
                }
                syn::Meta::Path(path) if path.is_ident("default") => {
                    attrs.default = true;
                }
                _ => return Err(Error::new(Span::call_site(), "unknown argument!")),
            }

            input.parse::<syn::token::Comma>().is_ok()
        } {}

        Ok(attrs)
    }
}

impl Parse for UrcAttributes {
    fn parse(input: ParseStream) -> Result<Self> {
        let code = match input.parse::<syn::Lit>()? {
            Lit::ByteStr(b) => b,
            Lit::Str(s) => LitByteStr::new(s.value().as_bytes(), input.span()),
            _ => {
                return Err(Error::new(
                    input.span(),
                    "expected string value for `at_urc`",
                ))
            }
        };

        Ok(Self { code })
    }
}

impl Parse for CmdAttributes {
    fn parse(input: ParseStream) -> Result<Self> {
        let call_site = Span::call_site();
        let cmd = input.parse::<syn::LitStr>()?;
        let _comma = input.parse::<syn::token::Comma>()?;
        let response_ident = input.parse::<Path>()?;

        let mut at_cmd = Self {
            cmd: cmd.value(),
            resp: response_ident,
            timeout_ms: None,
            abortable: None,
            value_sep: true,
            cmd_prefix: String::from("AT"),
            termination: String::from("\r\n"),
        };

        while input.parse::<syn::token::Comma>().is_ok() {
            let optional = input.parse::<syn::MetaNameValue>()?;
            if optional.path.is_ident("timeout_ms") {
                match optional.lit {
                    Lit::Int(v) => {
                        at_cmd.timeout_ms = Some(v.base10_parse().unwrap());
                    }
                    _ => {
                        return Err(Error::new(
                            call_site,
                            "expected integer value for 'timeout_ms'",
                        ))
                    }
                }
            } else if optional.path.is_ident("abortable") {
                match optional.lit {
                    Lit::Bool(v) => {
                        at_cmd.abortable = Some(v.value);
                    }
                    _ => return Err(Error::new(call_site, "expected bool value for 'abortable'")),
                }
            } else if optional.path.is_ident("value_sep") {
                match optional.lit {
                    Lit::Bool(v) => {
                        at_cmd.value_sep = v.value;
                    }
                    _ => return Err(Error::new(call_site, "expected bool value for 'value_sep'")),
                }
            } else if optional.path.is_ident("cmd_prefix") {
                match optional.lit {
                    Lit::Str(v) => {
                        at_cmd.cmd_prefix = v.value();
                    }
                    _ => {
                        return Err(Error::new(
                            call_site,
                            "expected string value for 'cmd_prefix'",
                        ))
                    }
                }
            } else if optional.path.is_ident("termination") {
                match optional.lit {
                    Lit::Str(v) => {
                        at_cmd.termination = v.value();
                    }
                    _ => {
                        return Err(Error::new(
                            call_site,
                            "expected string value for 'termination'",
                        ))
                    }
                }
            }
        }

        Ok(at_cmd)
    }
}

impl Parse for ParseInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let derive_input = DeriveInput::parse(input)?;

        let mut at_cmd = None;
        let mut at_enum = None;

        // Parse valid container attributes
        for attr in derive_input.attrs {
            if attr.path.is_ident("at_cmd") {
                fn at_cmd_arg(input: ParseStream) -> Result<CmdAttributes> {
                    let content;
                    parenthesized!(content in input);
                    content.parse()
                }
                at_cmd = Some(at_cmd_arg.parse2(attr.tokens)?);
            } else if attr.path.is_ident("at_enum") {
                fn at_enum_arg(input: ParseStream) -> Result<Ident> {
                    let content;
                    parenthesized!(content in input);
                    content.parse()
                }
                at_enum = Some(EnumAttributes {
                    repr: at_enum_arg.parse2(attr.tokens)?,
                });
            }
        }

        Ok(Self {
            ident: derive_input.ident,
            generics: derive_input.generics,
            at_cmd,
            at_enum,
            variants: sorted_variants(derive_input.data)?,
        })
    }
}
