//! Runs generated code and the actual wrapper source in both consumer modes.

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use quote::quote;
use syn::{Data, DeriveInput, parse2};

use crate::debug::expand;

// Removes temporary compiler fixtures even when an assertion fails.
struct FixtureDirectory(PathBuf);

impl Drop for FixtureDirectory {
    // Cleanup failure does not obscure the original compiler/test failure.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// Expand real declarations, removing only derive helper markers afterwards.
fn declarations() -> syn::Result<String> {
    //
    let mut source = String::new();

    for input in [
        quote! {
            pub struct Credentials<'a, T: ?Sized, const N: usize>
            where T: 'a {
                pub id: &'a str,

                #[sensitive]
                pub secret: &'a T,

                pub sequence: [u8; N],
            }
        },
        quote!(
            pub struct Pair<T>(pub u32, #[sensitive] pub T);
        ),
        quote!(
            pub struct Unit;
        ),
        quote!(
            pub struct Empty {}
        ),
        quote!(
            pub struct EmptyTuple();
        ),
        quote!(
            pub struct Raw {
                pub r#type: u32,
            }
        ),
        quote! {
            pub struct Envelope<'a> {
                pub credentials: Option<Credentials<'a, str, 2>>,
            }
        },
    ] {
        //
        let mut declaration: DeriveInput = parse2(input.clone())?;

        let implementation = expand(input)?;

        if let Data::Struct(data) = &mut declaration.data {
            //
            for field in &mut data.fields {
                field
                    .attrs
                    .retain(|attr| !attr.path().is_ident("sensitive"));
            }
        }

        source.push_str(&quote!(#declaration #implementation).to_string());
    }

    Ok(source)
}

#[test]
fn consumer_modes_redact_without_invoking_sensitive_formatters()
-> Result<(), Box<dyn Error>> {
    //
    let directory = FixtureDirectory(
        std::env::temp_dir()
            .join(format!("poprako-sensitive-compile-{}", std::process::id())),
    );

    fs::create_dir_all(&directory.0)?;

    let wrapper = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../poprako-sensitive/src/sensitive.rs");

    let wrapper = wrapper.canonicalize()?;

    let source = format!(
        "#[path = {wrapper:?}] mod sensitive;\n{}\n{}",
        declarations()?,
        include_str!("fixture.txt"),
    );

    let source_path = directory.0.join("fixture.rs");

    fs::write(&source_path, source)?;

    for assertions in ["yes", "no"] {
        //
        let executable = directory.0.join(format!("fixture-{assertions}"));

        let compiled = Command::new("rustc")
            .arg("--edition=2024")
            .arg("--test")
            .arg("-Dwarnings")
            .arg("-C")
            .arg(format!("debug-assertions={assertions}"))
            .arg(&source_path)
            .arg("-o")
            .arg(&executable)
            .output()?;

        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );

        let tested = Command::new(executable).output()?;

        assert!(
            tested.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&tested.stdout),
            String::from_utf8_lossy(&tested.stderr)
        );
    }

    Ok(())
}
