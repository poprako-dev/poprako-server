//! HTTP API surface for the `PopRaKo` application.
//!
//! Business routes live under `/api/v1`; the health endpoint lives at
//! `/api/health`. Swagger UI and `OpenAPI` JSON are exposed in debug builds only
//! at `/api/swagger-ui` and `/api/openapi.json`.

/// HTTP server, handlers, middleware, router, and OpenAPI documentation.
pub mod http;
