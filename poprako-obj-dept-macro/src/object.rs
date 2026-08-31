// Generates typed Diesel entries for each object module.
mod rdb_entry;

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Path, Result, Token, braced};

// Stores one parsed object manifest declaration.
struct ObjInput {
    //
    // Identifies the object marker type.
    marker: Ident,

    // Identifies the typed Diesel table module.
    table: Path,

    // Stores the object task topic.
    topic: LitStr,
}

impl Parse for ObjInput {
    // Parses one object manifest declaration.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        //
        let marker = input.parse()?;

        let content;

        braced!(content in input);

        parse_field(&content, "table")?;

        let table = content.parse()?;

        content.parse::<Token![,]>()?;

        parse_field(&content, "topic")?;

        let topic = content.parse()?;

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }

        if !content.is_empty() {
            return Err(input.error("unexpected object declaration tokens"));
        }

        Ok(Self {
            marker,
            table,
            topic,
        })
    }
}

// Wraps the object declarations accepted by the macro.
struct ObjsInput(Vec<ObjInput>);

impl Parse for ObjsInput {
    // Parses all object manifest declarations.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        //
        let mut objs = Vec::new();

        while !input.is_empty() {
            //
            objs.push(input.parse()?);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        if objs.is_empty() {
            return Err(input.error("expected at least one object"));
        }

        Ok(Self(objs))
    }
}

/// Expands the complete object manifest and typed Diesel operations.
pub fn expand(input: TokenStream) -> Result<TokenStream> {
    //
    let ObjsInput(objs) = syn::parse2(input)?;

    validate_unique(&objs)?;

    let modules = objs.iter().map(expand_obj);

    let unique_markers = objs.iter().map(|obj| &obj.marker);

    let unique_tables = objs.iter().map(|obj| &obj.table);

    let manifest = objs.iter().map(|obj| {
        //
        let marker = &obj.marker;

        let module = marker_module(marker);

        let topic = &obj.topic;

        quote!((#marker, #module, #topic),)
    });

    Ok(quote! {
        mod obj_manifest_uniqueness {
            trait Marker {}

            #(impl Marker for super::#unique_markers {})*

            trait Table {}

            #(impl Table for super::#unique_tables::table {})*
        }

        #(#modules)*

        macro_rules! for_each_obj {
            ($callback:ident) => {
                $callback! {
                    #(#manifest)*
                }
            };
        }
    })
}

// Validates object-manifest identifiers for uniqueness.
fn validate_unique(objs: &[ObjInput]) -> Result<()> {
    //
    let mut markers = HashSet::new();

    let mut tables = HashSet::new();

    let mut topics = HashSet::new();

    for obj in objs {
        //
        validate_value(
            &mut markers,
            obj.marker.to_string(),
            &obj.marker,
            "marker",
        )?;

        let table = &obj.table;

        validate_value(
            &mut tables,
            quote!(#table).to_string(),
            &obj.marker,
            "table",
        )?;

        validate_value(&mut topics, obj.topic.value(), &obj.marker, "topic")?;
    }

    Ok(())
}

// Builds the generated RDB module identifier for an object marker.
fn marker_module(marker: &Ident) -> Ident {
    format_ident!("{}_rdb_impl", to_snake_case(&marker.to_string()))
}

// Validates one distinct object-manifest value.
fn validate_value(
    values: &mut HashSet<String>,
    value: String,
    marker: &Ident,
    kind: &str,
) -> Result<()> {
    //
    if values.insert(value) {
        return Ok(());
    }

    Err(syn::Error::new(
        marker.span(),
        format!("duplicate object {}", kind),
    ))
}

// Converts an object marker into a generated module name.
fn to_snake_case(name: &str) -> String {
    //
    let mut snake_case = String::with_capacity(name.len());

    let mut chars = name.chars().peekable();

    let mut prev = None;

    while let Some(current) = chars.next() {
        //
        let next = chars.peek().copied();

        let follows_lowercase = prev.is_some_and(|character: char| {
            character.is_lowercase() || character.is_ascii_digit()
        });

        let is_word_boundary = current.is_uppercase()
            && !snake_case.is_empty()
            && (follows_lowercase || next.is_some_and(char::is_lowercase));

        if is_word_boundary {
            snake_case.push('_');
        }

        snake_case.extend(current.to_lowercase());

        prev = Some(current);
    }

    snake_case
}

// Expands one object module and its typed Diesel operations.
fn expand_obj(obj: &ObjInput) -> TokenStream {
    //
    let table = &obj.table;

    let topic = &obj.topic;

    let module = marker_module(&obj.marker);

    let rdb_entry = rdb_entry::expand(table);

    quote! {
        mod #module {
            use super::#table;

            pub const TOPIC: &str = #topic;

            #rdb_entry
        }
    }
}

// Parses a named field in an object manifest declaration.
fn parse_field(input: ParseStream<'_>, expected: &str) -> Result<()> {
    //
    let field = input.parse::<Ident>()?;

    if field != expected {
        //
        return Err(syn::Error::new(
            field.span(),
            format!("expected `{}`", expected),
        ));
    }

    input.parse::<Token![:]>()?;

    Ok(())
}
