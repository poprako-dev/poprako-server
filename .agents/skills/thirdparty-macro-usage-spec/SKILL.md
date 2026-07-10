---
name: thirdparty-macro-usage-spec
description: Third-party macro import and call-site rules for active PopRaKo Rust code. Use when adding derives, attributes, tracing events, Diesel entities, or OpenAPI metadata.
---

# Third-Party Macro Usage

Import derive and attribute macros explicitly, then use their bare names:

```rust
use async_trait::async_trait;
use tracing::instrument;

#[async_trait]
trait Example {}

#[instrument]
async fn example() {}
```

Use fully qualified tracing event macros at the call site:

```rust
tracing::info!(resource_id = id, "resource updated");
```

Do not import `error`, `warn`, `info`, `debug`, or `trace` as bare macros.

## Active framework locations

- Serde derives and `#[serde(...)]` attributes belong in `data` and values.
- `#[cfg_attr(feature = "swagger-ui", ...)]` OpenAPI derives and `#[utoipa::path]` annotations belong in API DTOs and handlers.
- Diesel derives and `#[diesel(...)]` attributes belong in `part_impl/repo/rdb_impl/entity`. Refer to the generated table module through its local path; never introduce `use ...::schema` or `schema::`.
- `#[async_trait]` is used for async port and adapter implementations.

Follow nearby code for exact imports and macro options. Macro style does not override the layer rules in `tracing-usage-spec` or repository conventions.
