use proc_macro::TokenStream;

use syn::{
    Data, DeriveInput, Error, Fields, FieldsNamed, Result, parse_macro_input,
};

/// Appends `pub offset: u64` and `pub limit: u64` fields to a struct.
///
/// # Example
///
/// ```ignore
/// #[Paginate]
/// pub struct ListTeams {
///     pub name: String,
/// }
/// ```
///
/// Expands to:
///
/// ```ignore
/// pub struct ListTeams {
///     pub name: String,
///     pub offset: u64,
///     pub limit: u64,
/// }
/// ```
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn Paginate(attr: TokenStream, item: TokenStream) -> TokenStream {
    parse_macro_input!(attr as EmptyArgs);
    let input = parse_macro_input!(item as DeriveInput);

    expand_page(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

struct EmptyArgs;

impl syn::parse::Parse for EmptyArgs {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        if !input.is_empty() {
            return Err(Error::new(
                input.span(),
                "Paginate does not accept arguments",
            ));
        }

        Ok(Self)
    }
}

fn expand_page(input: DeriveInput) -> Result<proc_macro2::TokenStream> {
    let named: &FieldsNamed = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields,
            _ => {
                return Err(Error::new_spanned(
                    &input.ident,
                    "Page only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(Error::new_spanned(
                &input.ident,
                "Page only supports structs",
            ));
        }
    };

    for field in &named.named {
        let ident = field.ident.as_ref().expect("named field ident");
        if ident == "offset" || ident == "limit" {
            return Err(Error::new_spanned(
                ident,
                format!(
                    "Page cannot add field `{}` because it already exists",
                    ident
                ),
            ));
        }
    }

    let vis = &input.vis;
    let attrs = &input.attrs;
    let struct_ident = &input.ident;
    let generics = &input.generics;
    let where_clause = &generics.where_clause;
    let fields = named.named.iter();

    Ok(quote::quote! {
        #(#attrs)*
        #vis struct #struct_ident #generics
        #where_clause
        {
            #(#fields,)*
            pub offset: u64,
            pub limit: u64,
        }
    })
}
