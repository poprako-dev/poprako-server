use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;

use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::{Loader as _, static_loader};
use unic_langid::{LanguageIdentifier, langid};

static_loader! {
    static LOCALES = {
        locales: "locales",
        fallback_language: "zh-CN",
    };
}

static LANGUAGE: LazyLock<LanguageIdentifier> = LazyLock::new(|| {
    let language = std::env::var("LANGUAGE").unwrap_or_else(|_| "zh-CN".to_string());
    language.parse().unwrap_or_else(|_| langid!("zh-CN"))
});

pub fn trl(key: &str) -> String {
    LOCALES.lookup(&LANGUAGE, key)
}

/// Looks up a parameterized Fluent message for the current language.
///
/// `args` maps Fluent variable names (without the `$` prefix) to their
/// [`FluentValue`] replacements. Variables in the `.ftl` file must use the
/// `{$name}` syntax.
pub fn trl_kv(key: &str, args: &HashMap<Cow<'static, str>, FluentValue>) -> String {
    LOCALES.lookup_with_args(&LANGUAGE, key, args)
}
