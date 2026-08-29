#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]
#![deny(clippy::exit)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::mod_module_files)]
#![warn(clippy::style)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::future_not_send)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::uninlined_format_args)]

//! Standalone binary that prints the generated `OpenAPI` specification to stdout.
//!
//! Run with `cargo run -p poprako-swagger` — the `swagger` feature is always
//! enabled by this crate's dependency on `poprako-server`, so no `--features`
//! flag is needed.

use std::io::Write as _;

use utoipa::OpenApi as _;

use poprako_server::ApiDoc;

fn main() -> anyhow::Result<()> {
    //
    let doc = ApiDoc::openapi();

    let swagger_json = serde_json::to_string_pretty(&doc)?;

    #[allow(clippy::print_stdout)]
    {
        std::io::stdout().write_all(swagger_json.as_bytes())?;
    }

    Ok(())
}
