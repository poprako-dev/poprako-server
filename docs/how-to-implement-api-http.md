# How to Implement API HTTP

This document defines the PopRaKo-R HTTP API implementation rules. Follow it
whenever adding or changing code under `src/api/http/`.

## Core Rules

1. Import `Accept` as an anonymous trait and use `.accept(...)` for successful
   responses.

   ```rust
   use axum::http::StatusCode;

   use crate::api::http::result::Accept as _;
   use crate::api::http::result::HttpResult;

   pub async fn get_info(...) -> HttpResult<UserInfoVal> {
       let user_info_val = usecase::user::get_info(...).await?;

       user_info_val.accept(StatusCode::OK)
   }
   ```

   Do not import the trait as `Accept`, and do not return successful responses
   with a manually assembled successful `HttpResult` in new handlers.

2. Propagate usecase errors directly with `?`.

   ```rust
   let team_info_val = usecase::team::get_info(...).await?;
   ```

   Do not `match` usecase errors in handlers unless the handler is deliberately
   translating a request-local condition that does not belong in the usecase.
   Application errors already convert into `HttpError`.

3. Use standard RESTful route structure.

   - Use plural resource nouns: `/users`, `/teams`, `/comics`, `/chapters`.
   - Use path params for resource identity: `/teams/{team_id}`.
   - Use query params for filtering, pagination, includes, and sort options.
   - Use HTTP methods by semantics:
     - `GET /resources` lists resources.
     - `POST /resources` creates a resource.
     - `GET /resources/{resource_id}` reads one resource.
     - `PUT /resources/{resource_id}` replaces or updates by PUT semantics.
     - `DELETE /resources/{resource_id}` removes one resource.

   Process-oriented routes are allowed only when the domain operation is
   genuinely not CRUD-shaped. Examples include upload reservation and upload
   confirmation, such as `/users/{user_id}/avatar/reserve`.

4. Every handler must have matching `utoipa` OpenAPI documentation.

   - Add `#[utoipa::path(...)]` on the handler.
   - Keep the documented method and path exactly aligned with the router.
   - Add request/response schemas to `src/api/http/openapi.rs` when needed.
   - OpenAPI derives and routes are gated by the `swagger-ui` Cargo feature.
   - Swagger and OpenAPI routes live outside versioned API routes:
     `/api/swagger-ui` and `/api/openapi.json`, not under `/api/v1`.

5. Do not copy the Go API implementation.

   The Go project is a reference for business behavior, not a source template.
   PopRaKo-R and PopRaKo-S differ substantially in naming, route shape, error
   handling, handler structure, and sometimes behavior. The Rust implementation
   only needs functional consistency where the Rust design intentionally
   requires it. List ordering, route names, value object names, and handler
   mechanics may differ.

## Handler Checklist

Before finishing an API handler:

- Import `Accept as _`.
- Return successful responses with `.accept(StatusCode::...)`.
- Call the usecase function and propagate its error with `?`.
- Keep local request validation limited to HTTP boundary concerns.
- Ensure all protected handlers rely on auth middleware extensions instead of
  reparsing auth credentials.
- Add or update `#[utoipa::path]`.
- Register the handler in `src/api/http/openapi.rs`.
- Register the route in `src/api/http/router.rs`.
- Verify the route uses plural resource nouns and resource path params.
- Run `cargo fmt --check` and `cargo check`.
