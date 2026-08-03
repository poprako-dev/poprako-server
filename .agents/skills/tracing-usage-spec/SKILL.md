---
name: tracing-usage-spec
description: Current tracing span, event, field, redaction, and error-propagation rules for PopRaKo use cases, HTTP boundaries, schedulers, and adapters. Use whenever adding or reviewing instrumentation or logs.
---

# Tracing usage

Draw spans around observable operations, not pure data transformation.

| Location | Convention |
| --- | --- |
| Public fallible use case | `#[instrument(level = "info", skip(...))]` |
| HTTP handler | `#[instrument(level = "info", skip_all)]` |
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
- Returning an error through `?` is propagation, not error production. Do not
  configure `#[instrument]` to emit an error event merely because an
  instrumented function returns `Err`.
- Emit a direct event for lifecycle state, retries, intentional error
  consumption, application error construction, or external error conversion.
- Before an adapter converts an external SDK, driver, or client error into an
  application error, emit exactly one event with the original error's `Debug`
  representation. This conversion site is the error-production leaf; callers
  using `?` only propagate and must not log the same failure again.
- Never emit credentials, JWTs, plaintext passwords, or presigned URLs. Keep
  business instruction DTOs in use-case spans when they are required to
  reconstruct the failed operation; skip authentication tokens and redact any
  secret-bearing instruction fields explicitly.

Import `tracing::instrument` explicitly and apply it by its bare attribute
name.
