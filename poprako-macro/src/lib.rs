use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::ToTokens;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Error;
use syn::Field;
use syn::Fields;
use syn::FieldsNamed;
use syn::FnArg;
use syn::Ident;
use syn::ItemTrait;
use syn::Pat;
use syn::Path;
use syn::PathArguments;
use syn::Result;
use syn::Token;
use syn::TraitItem;
use syn::TraitItemFn;
use syn::Type;
use syn::parse::Parse;
use syn::parse_macro_input;
use syn::punctuated::Punctuated;

/// Generates a forwarding marker and blanket forwarding implementation for a trait.
#[proc_macro_attribute]
pub fn forward_ref(attr: TokenStream, item: TokenStream) -> TokenStream {
    parse_macro_input!(attr as EmptyArgs);
    let item_trait = parse_macro_input!(item as ItemTrait);

    expand_forward_impl(item_trait)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Generates a forwarding marker and blanket [`ForwardRef`] bridge impls for a
/// composite trait whose supertraits each have their own [`forward_ref`] markers.
///
/// # Example
///
/// ```ignore
/// #[forward_ref_super]
/// pub trait Query: UserQuery + TeamQuery {
/// }
/// ```
///
/// This generates:
/// - `pub struct QueryForward;` — the forwarding marker for [`Query`]
/// - Blanket `ForwardRef<UserQueryForward>` and `ForwardRef<TeamQueryForward>`
///   impls that delegate to `ForwardRef<QueryForward>`
///
/// Use `Query` as a single marker in `#[derive(ForwardRefs)]` instead of listing
/// each sub-trait individually.
#[proc_macro_attribute]
pub fn forward_ref_super(attr: TokenStream, item: TokenStream) -> TokenStream {
    parse_macro_input!(attr as EmptyArgs);
    let item_trait = parse_macro_input!(item as ItemTrait);

    expand_forward_sub(item_trait)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Generates [`crate::ForwardRef`] implementations from field markers.
#[proc_macro_derive(ForwardRefs, attributes(forward_ref))]
pub fn derive_forward_refs(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    expand_forward_refs(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

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

impl Parse for EmptyArgs {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        if !input.is_empty() {
            return Err(Error::new(
                input.span(),
                "forward_ref does not accept arguments; marker name is inferred",
            ));
        }

        Ok(Self)
    }
}

fn expand_forward_impl(item_trait: ItemTrait) -> Result<proc_macro2::TokenStream> {
    let trait_ident = &item_trait.ident;
    let vis = &item_trait.vis;

    if !item_trait.generics.params.is_empty() || item_trait.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &item_trait.generics,
            "forward_ref does not support trait generics",
        ));
    }

    let async_impl = trait_has_async_method(&item_trait);
    let async_trait_attr = find_async_trait_attr(&item_trait);
    let sync_bound = async_impl.then(|| quote!(+ Sync));
    let target_sync_bound = async_impl.then(|| quote!(+ Sync));
    let marker = Ident::new(&format!("{}Forward", trait_ident), trait_ident.span());
    let marker_doc = syn::LitStr::new(
        &format!("Forwarding marker for [`{}`].", trait_ident),
        Span::call_site(),
    );

    let methods = item_trait
        .items
        .iter()
        .map(|item| match item {
            TraitItem::Fn(method) => expand_forward_method(method),
            _ => Err(Error::new_spanned(
                item,
                "forward_ref only supports method items in traits",
            )),
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #[doc = #marker_doc]
        #vis struct #marker;

        #item_trait

        #async_trait_attr
        impl<T> #trait_ident for T
        where
            T: crate::ForwardRef<#marker> #sync_bound,
            T::Target: #trait_ident #target_sync_bound,
        {
            #(#methods)*
        }
    })
}

fn trait_has_async_method(item_trait: &ItemTrait) -> bool {
    item_trait
        .items
        .iter()
        .any(|item| matches!(item, TraitItem::Fn(method) if method.sig.asyncness.is_some()))
}

fn find_async_trait_attr(item_trait: &ItemTrait) -> proc_macro2::TokenStream {
    item_trait
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("async_trait"))
        .map(ToTokens::to_token_stream)
        .unwrap_or_default()
}

fn expand_forward_method(method: &TraitItemFn) -> Result<proc_macro2::TokenStream> {
    if method.default.is_some() {
        return Err(Error::new_spanned(
            method,
            "forward_ref does not support trait methods with default bodies",
        ));
    }

    if !method.sig.generics.params.is_empty() || method.sig.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &method.sig.generics,
            "forward_ref does not support generic trait methods",
        ));
    }

    let receiver =
        method.sig.inputs.first().ok_or_else(|| {
            Error::new_spanned(&method.sig, "forward_ref methods must take &self")
        })?;

    match receiver {
        FnArg::Receiver(receiver)
            if receiver.reference.is_some() && receiver.mutability.is_none() => {}
        _ => {
            return Err(Error::new_spanned(
                receiver,
                "forward_ref only supports methods whose receiver is &self",
            ));
        }
    }

    let sig = &method.sig;
    let method_ident = &method.sig.ident;
    let args = method
        .sig
        .inputs
        .iter()
        .skip(1)
        .map(method_call_arg)
        .collect::<Result<Vec<_>>>()?;

    let await_token = method.sig.asyncness.map(|_| quote!(.await));

    Ok(quote! {
        #sig {
            self.forward_ref().#method_ident(#(#args),*) #await_token
        }
    })
}

fn method_call_arg(input: &FnArg) -> Result<proc_macro2::TokenStream> {
    match input {
        FnArg::Typed(typed) => match typed.pat.as_ref() {
            Pat::Ident(ident) => {
                let arg_ident = &ident.ident;
                Ok(quote!(#arg_ident))
            }
            Pat::Reference(reference) => match reference.pat.as_ref() {
                Pat::Ident(ident) => {
                    let arg_ident = &ident.ident;
                    Ok(quote!(#arg_ident))
                }
                _ => Err(Error::new_spanned(
                    &typed.pat,
                    "forward_ref only supports identifier argument patterns",
                )),
            },
            _ => Err(Error::new_spanned(
                &typed.pat,
                "forward_ref only supports identifier argument patterns",
            )),
        },
        FnArg::Receiver(_) => Err(Error::new_spanned(
            input,
            "unexpected receiver in forwarded argument list",
        )),
    }
}

fn expand_forward_sub(item_trait: ItemTrait) -> Result<proc_macro2::TokenStream> {
    let trait_ident = &item_trait.ident;
    let vis = &item_trait.vis;

    if !item_trait.generics.params.is_empty() || item_trait.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &item_trait.generics,
            "forward_ref_super does not support trait generics",
        ));
    }

    let marker = Ident::new(&format!("{}Forward", trait_ident), trait_ident.span());
    let marker_doc = syn::LitStr::new(
        &format!("Forwarding marker for [`{}`].", trait_ident),
        Span::call_site(),
    );

    // Extract supertrait paths from `trait Foo: A + B + C`.
    let super_paths: Vec<Path> = item_trait
        .supertraits
        .iter()
        .filter_map(|bound| match bound {
            syn::TypeParamBound::Trait(trait_bound) => Some(trait_bound.path.clone()),
            _ => None,
        })
        .collect();

    if super_paths.is_empty() {
        return Err(Error::new_spanned(
            &item_trait,
            "forward_ref_super requires at least one supertrait bound",
        ));
    }

    // For each supertrait, build the forward marker path by appending "Forward"
    // to the last segment.  Then emit a blanket ForwardRef<ChildForward> impl
    // that delegates to ForwardRef<ParentForward>.
    let bridge_impls = super_paths
        .iter()
        .map(|super_path| {
            let child_marker = marker_forward_path(super_path.clone())?;
            Ok(quote! {
                impl<T> crate::ForwardRef<#child_marker> for T
                where
                    T: crate::ForwardRef<#marker>,
                {
                    type Target = <T as crate::ForwardRef<#marker>>::Target;

                    fn forward_ref(&self) -> &Self::Target {
                        <T as crate::ForwardRef<#marker>>::forward_ref(self)
                    }
                }
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #[doc = #marker_doc]
        #vis struct #marker;

        #item_trait

        #(#bridge_impls)*
    })
}

fn expand_forward_refs(input: DeriveInput) -> Result<proc_macro2::TokenStream> {
    let struct_ident = &input.ident;
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(Error::new_spanned(
                    &input.ident,
                    "ForwardRefs only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(Error::new_spanned(
                &input.ident,
                "ForwardRefs only supports structs",
            ));
        }
    };

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let impls = fields
        .iter()
        .map(|field| {
            expand_field_forward_refs(
                field,
                struct_ident,
                &impl_generics,
                &ty_generics,
                &where_clause,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #(#impls)*
    })
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
    let struct_ident = &input.ident;
    let generics = &input.generics;
    let where_clause = &generics.where_clause;
    let fields = named.named.iter();

    Ok(quote! {
        #vis struct #struct_ident #generics
        #where_clause
        {
            #(#fields,)*
            pub offset: u64,
            pub limit: u64,
        }
    })
}

fn expand_field_forward_refs(
    field: &Field,
    struct_ident: &Ident,
    impl_generics: &syn::ImplGenerics<'_>,
    ty_generics: &syn::TypeGenerics<'_>,
    where_clause: &Option<&syn::WhereClause>,
) -> Result<proc_macro2::TokenStream> {
    let field_ident = field.ident.as_ref().ok_or_else(|| {
        Error::new_spanned(field, "ForwardRefs only supports structs with named fields")
    })?;

    let specs = field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("forward_ref"))
        .map(|attr| attr.parse_args_with(Punctuated::<ForwardRefArg, Token![,]>::parse_terminated))
        .collect::<Result<Vec<_>>>()?;

    let mut impls = Vec::new();

    for spec in specs {
        let mut target = None;
        let mut markers = Vec::new();

        for arg in spec {
            match arg {
                ForwardRefArg::Target(target_type) => target = Some(target_type),
                ForwardRefArg::Marker(marker) => markers.push(marker_forward_path(marker)?),
            }
        }

        if markers.is_empty() {
            return Err(Error::new_spanned(
                field,
                "forward_ref requires at least one marker",
            ));
        }

        let target = target.unwrap_or_else(|| field.ty.clone());
        impls.extend(markers.into_iter().map(|marker| {
            quote! {
                impl #impl_generics crate::ForwardRef<#marker>
                    for #struct_ident #ty_generics #where_clause
                {
                    type Target = #target;

                    fn forward_ref(&self) -> &#target {
                        &self.#field_ident
                    }
                }
            }
        }));
    }

    Ok(quote! {
        #(#impls)*
    })
}

enum ForwardRefArg {
    Target(Type),
    Marker(Path),
}

impl Parse for ForwardRefArg {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        if input.peek(Ident) && input.peek2(Token![=]) {
            let ident: Ident = input.parse()?;
            if ident != "target" {
                return Err(Error::new_spanned(
                    ident,
                    "forward_ref only supports `target = Type` named argument",
                ));
            }

            input.parse::<Token![=]>()?;
            return Ok(Self::Target(input.parse()?));
        }

        Ok(Self::Marker(input.parse()?))
    }
}

fn marker_forward_path(mut path: Path) -> Result<Path> {
    if path.segments.is_empty() {
        return Err(Error::new_spanned(
            path,
            "forward_ref marker path cannot be empty",
        ));
    }

    let segment = path.segments.last_mut().expect("path segment checked");

    if !matches!(segment.arguments, PathArguments::None) {
        return Err(Error::new_spanned(
            &segment.arguments,
            "forward_ref marker path must not include generic arguments",
        ));
    }

    let ident = segment.ident.to_string();
    if !ident.ends_with("Forward") {
        segment.ident = Ident::new(&format!("{}Forward", ident), segment.ident.span());
    }

    Ok(path)
}
