//! Standalone binary that prints the generated OpenAPI specification to stdout.
//!
//! Run with `cargo run -p poprako-swagger` — the `swagger` feature is always
//! enabled by this crate's dependency on `poprako-server`, so no `--features`
//! flag is needed.

use std::io::Write as _;

use utoipa::OpenApi as _;

fn main() -> anyhow::Result<()> {
    let doc = poprako_server::ApiDoc::openapi();

    let swagger_json = serde_json::to_string_pretty(&doc)?;

    #[allow(clippy::print_stdout)]
    {
        std::io::stdout().write_all(swagger_json.as_bytes())?;
    }

    Ok(())
}
