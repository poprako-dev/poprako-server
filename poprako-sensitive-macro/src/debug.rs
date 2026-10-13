//! Generates independent Debug implementations for the consumer's two modes.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Field, Fields, Generics, Index, Meta,
    parse_quote, parse2,
};

/// Expands a struct declaration into Debug implementations without rewriting it.
pub fn expand(tokens: TokenStream) -> syn::Result<TokenStream> {
    //
    let input = parse2::<DeriveInput>(tokens)?;

    if let Some(attribute) = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("sensitive"))
    {
        //
        return Err(Error::new_spanned(
            attribute,
            "#[sensitive] belongs on a struct field",
        ));
    }

    let Data::Struct(data) = &input.data else {
        //
        return Err(Error::new_spanned(
            &input.ident,
            "SensitiveDebug supports structs only",
        ));
    };

    let sensitive = data
        .fields
        .iter()
        .map(is_sensitive)
        .collect::<syn::Result<Vec<_>>>()?;

    let visible = implementation(&input, &data.fields, &sensitive, true);

    let redacted = implementation(&input, &data.fields, &sensitive, false);

    Ok(quote!(#visible #redacted))
}

// Generate one mode with bounds only on fields that will actually be formatted.
fn implementation(
    input: &DeriveInput,
    fields: &Fields,
    sensitive: &[bool],
    reveal: bool,
) -> TokenStream {
    //
    let name = &input.ident;

    let mut generics = input.generics.clone();

    let mut statements = Vec::new();

    for (index, (field, sensitive)) in fields.iter().zip(sensitive).enumerate()
    {
        //
        let value =
            field_value(field, index, &mut generics, !reveal && *sensitive);

        let statement = field.ident.as_ref().map_or_else(
            || quote!(builder.field(#value);),
            |name| {
                //
                let label =
                    name.to_string().trim_start_matches("r#").to_owned();

                quote!(builder.field(#label, #value);)
            },
        );

        statements.push(statement);
    }

    let body = match fields {
        //
        Fields::Named(_) => quote! {
            //
            let mut builder = formatter.debug_struct(stringify!(#name));

            #(#statements)*

            builder.finish()
        },

        Fields::Unnamed(_) => quote! {
            //
            let mut builder = formatter.debug_tuple(stringify!(#name));

            #(#statements)*

            builder.finish()
        },

        Fields::Unit => quote!(formatter.write_str(stringify!(#name))),
    };

    let condition = assertion_cfg(reveal);

    let (impl_generics, type_generics, where_clause) =
        generics.split_for_impl();

    quote! {
        #[cfg(#condition)]
        impl #impl_generics ::core::fmt::Debug for #name #type_generics #where_clause {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #body
            }
        }
    }
}

// Select a field expression without touching the value or requiring hidden bounds.
fn field_value(
    field: &Field,
    index: usize,
    generics: &mut Generics,
    hidden: bool,
) -> TokenStream {
    //
    if hidden {
        return quote!(&"[REDACTED]");
    }

    let field_type = &field.ty;

    generics
        .make_where_clause()
        .predicates
        .push(parse_quote!(#field_type: ::core::fmt::Debug));

    field.ident.as_ref().map_or_else(
        || {
            //
            let index = Index::from(index);

            quote!(&self.#index)
        },
        |name| quote!(&self.#name),
    )
}

// Keep configuration selection in the generated consumer code.
fn assertion_cfg(reveal: bool) -> TokenStream {
    //
    if reveal {
        return quote!(debug_assertions);
    }

    quote!(not(debug_assertions))
}

// Validate the single field marker rather than silently ignoring misspellings.
fn is_sensitive(field: &Field) -> syn::Result<bool> {
    //
    let mut sensitive = false;

    for attribute in &field.attrs {
        //
        if !attribute.path().is_ident("sensitive") {
            continue;
        }

        if sensitive || !matches!(attribute.meta, Meta::Path(_)) {
            //
            return Err(Error::new_spanned(
                attribute,
                "use exactly one #[sensitive] marker without arguments",
            ));
        }

        sensitive = true;
    }

    Ok(sensitive)
}
