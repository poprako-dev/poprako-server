---
name: api-http-spec
description: Axum HTTP handler, router, utoipa OpenAPI, and Swagger conventions for src/api/http/.
---

# API HTTP Specification

Follow `docs/how-to-implement-api-http.md` for the full rules. The summary
below captures the mandatory checks that most often fail in implementation.

## Success responses

Handlers must import `Accept` anonymously and return successful responses via
`.accept(...)`.

```rust
use axum::http::StatusCode;

use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpResult;

pub async fn get_info(...) -> HttpResult<UserBase> {
    let data = usecase::user::get_info(&harn, &user_id).await?;

    data.accept(StatusCode::OK)
}
```

Do not import `Accept` directly. Do not use `Ok(HttpResponse::from(data))` for
new successful handler returns.

## Error propagation

Usecase errors must be propagated directly with `?`.

```rust
let data = usecase::team::get_info(&harn, &team_id).await?;
```

Do not manually map ordinary usecase errors in HTTP handlers.

## RESTful routes

Routes must use standard RESTful structure:

- plural resource nouns (`/users`, `/teams`, `/chapters`);
- path params for resource IDs (`/teams/{team_id}`);
- query params for pagination, filters, includes, and sorting;
- method semantics (`GET`, `POST`, `PUT`, `DELETE`) instead of verbs in paths.

Process-style routes are allowed only when the operation is truly procedural,
such as upload reservation or upload confirmation.

## OpenAPI

Every handler must have matching `#[utoipa::path(...)]` documentation and must
be registered in `src/api/http/openapi.rs` when it should appear in the spec.
Debug Swagger routes must stay outside `/api/v1`, currently under
`/api/swagger-ui` with the JSON spec at `/api/openapi.json`.

## Go reference boundary

Read the Go code for business intent only. Do not copy Go route names,
handler names, value-object names, response mechanics, error handling, or list
ordering when they conflict with PopRaKo-R's Rust design.
