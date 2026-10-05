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

    #[allow(
        clippy::print_stdout,
        reason = "The Swagger executable writes its generated JSON document to stdout"
    )]
    {
        std::io::stdout().write_all(swagger_json.as_bytes())?;
    }

    Ok(())
}
