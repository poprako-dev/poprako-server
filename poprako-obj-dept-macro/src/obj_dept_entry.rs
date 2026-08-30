use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Result, Token, parenthesized};

/// Describes one object marker in a total department manifest.
pub struct ObjEntry {
    //
    /// Identifies the object marker type.
    marker: Ident,

    /// Identifies the generated object module.
    module: Ident,

    /// Stores the object task topic.
    topic: LitStr,

    /// Retains the manifest namespace for future macro expansion.
    _namespace: LitStr,

    /// Retains the manifest URL profile for future macro expansion.
    _url_profile: Ident,
}

impl ObjEntry {
    /// Returns the object marker type.
    pub const fn marker(&self) -> &Ident {
        &self.marker
    }

    /// Returns the generated object module.
    pub const fn module(&self) -> &Ident {
        &self.module
    }

    /// Returns the object task topic.
    pub const fn topic(&self) -> &LitStr {
        &self.topic
    }
}

impl Parse for ObjEntry {
    // Parses one object manifest entry.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        //
        let content;

        parenthesized!(content in input);

        let marker = content.parse()?;

        content.parse::<Token![,]>()?;

        let module = content.parse()?;

        content.parse::<Token![,]>()?;

        let topic = content.parse()?;

        content.parse::<Token![,]>()?;

        let namespace = content.parse()?;

        content.parse::<Token![,]>()?;

        let url_profile = content.parse()?;

        Ok(Self {
            marker,
            module,
            topic,
            _namespace: namespace,
            _url_profile: url_profile,
        })
    }
}
