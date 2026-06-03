use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use quote::quote;
use syn::Error;
use syn::FnArg;
use syn::Ident;
use syn::ItemTrait;
use syn::Pat;
use syn::Result;
use syn::TraitItem;
use syn::TraitItemFn;
use syn::parse::Parse;
use syn::parse_macro_input;

/// Generates a forwarding marker and blanket forwarding implementation for a trait.
#[proc_macro_attribute]
pub fn forward_ref(attr: TokenStream, item: TokenStream) -> TokenStream {
    parse_macro_input!(attr as EmptyArgs);
    let item_trait = parse_macro_input!(item as ItemTrait);

    expand_forward_impl(item_trait)
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
            T: crate::util::ForwardRef<#marker> #sync_bound,
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
