---
name: tracing-usage-spec
description: Current tracing span, event, field, redaction, and error-propagation rules for PopRaKo use cases, HTTP boundaries, schedulers, and adapters. Use whenever adding or reviewing instrumentation or logs.
---

# Tracing usage

Draw spans around observable operations, not pure data transformation.

| Location | Convention |
| --- | --- |
| Public fallible use case | `#[instrument(level = "info", err(Debug), skip(...))]` |
| HTTP handler | `#[instrument(level = "info", err(Debug), skip_all)]` |
| RDB, R2, JWT, Prom, effect, scheduler I/O | Instrument useful operation boundaries, normally with `skip_all` |
| `complex`, `model`, `data`, `value`, constructors, conversions | No span unless they perform observable I/O |
| `Harn` construction, clone, accessors | No span |

- Skip ports, contexts, connection handles, large DTOs, and sensitive values.
- Redact passwords and tokens explicitly when a useful non-secret field must
  still be recorded.
- Use stable structured fields such as `resource_id`, `operation`,
  `err_variant`, and `err_message`; keep the event message static.
- Call event macros as `tracing::info!`, `tracing::warn!`, and so on. Do not
  import event macro names into scope.
- Let an instrumented return record a propagated error. Emit a direct event for
  lifecycle state, retries, intentional error consumption, or error
  construction when structured context adds diagnostic value.
- Never emit credentials, JWTs, plaintext passwords, presigned URLs, or full
  private request bodies.

Import `tracing::instrument` explicitly and apply it by its bare attribute
name.
