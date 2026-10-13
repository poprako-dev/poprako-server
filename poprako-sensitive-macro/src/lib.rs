//! Derive support for source-declared sensitive struct fields.

// Generate the consumer's Debug implementations.
mod debug;

#[cfg(test)]
mod tests;

use proc_macro::TokenStream;

/// Implements Debug for a struct, replacing `#[sensitive]` fields with
/// `[REDACTED]` when the consuming crate has debug assertions disabled.
///
/// Supports named, tuple, and unit structs, including generics. Do not also
/// derive Debug. Field types need Debug only in configurations that show them.
#[proc_macro_derive(SensitiveDebug, attributes(sensitive))]
pub fn sensitive_debug(input: TokenStream) -> TokenStream {
    //
    debug::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
