---
name: tracing-usage-spec
description: Tracing placement for active PopRaKo code. Use when adding or reviewing #[instrument] and tracing events in use cases, HTTP handlers, or adapters.
---

# Tracing Placement

Draw spans at observable operation boundaries:

| Location | `#[instrument]` |
| --- | --- |
| Public fallible usecase orchestration | Usually yes |
| HTTP handlers | Yes |
| RDB, R2, JWT, prom, and effect I/O adapters | When useful to observe |
| `complex`, `model`, `value`, constructors, and conversions | No |
| `Harn` construction and accessors | No |

Use `#[instrument(err, skip(...))]` for handlers and usecases. Skip harnesses, large DTOs, secrets, connection handles, and values that should not be recorded. Prefer structured event fields such as `resource_id = id`.

Return errors through the instrumented boundary with `?`; do not emit a second error event while simply propagating. Emit direct tracing events for lifecycle state, retry decisions, or intentionally consumed errors.

Import `instrument` and use it bare. Invoke event macros as `tracing::info!`, `tracing::warn!`, and so on.
