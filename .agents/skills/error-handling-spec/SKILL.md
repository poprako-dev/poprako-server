---
name: error-handling-spec
description: Active PopRaKo application, adapter, transaction, and HTTP error rules. Use whenever constructing, converting, propagating, logging, or reviewing errors in Rust code.
---

# Error handling

The application error surface is
`crate::result::{BaseError, BaseRest, ExpectedVariant, accept}`.

## Classify errors where their meaning is known

- Use `BaseError::Expected` for client-correctable argument,
  authentication, and permission conditions. Select
  `ExpectedVariant::{Args, Auth, Perm}` and use an established
  `poprako_util::i18n::trl` key for a client-visible message.
- Use `BaseError::Unrecoverable` for infrastructure failures, corrupt
  persisted state, and violated internal invariants.
- Preserve a `BaseError` with `?` when the caller adds no new classification.
  Do not wrap an error only to restate the function name.
- Use `accept(value)` for a simple successful `BaseRest<T>` return when that is
  the nearby convention.

## Boundaries

- `Nucl::coord` converts backend and step errors through the existing
  `From<NuclError<...>> for BaseError` implementation in `src/result.rs`.
- Diesel and pool failures use `crate::shared::result` helpers. Query code
  should use `.optional()` when absence has local business meaning, then map
  `None` to the appropriate translated expected error.
- HTTP handlers return `HttpResult<T>`, propagate application errors with `?`,
  and use `Accept as _` or `no_content` for success. Only HTTP-specific facts,
  such as path/body identifier mismatches, are classified in handlers.
- `From<BaseError> for HttpError` owns the application-to-HTTP mapping and must
  not expose unrecoverable details to clients.

## Observability

Let an instrumented operation boundary record propagated failures. Add a
direct tracing event only when an error is constructed, consumed, retried, or
converted and the event adds structured diagnostic fields. Never record
passwords, tokens, credentials, or private payloads.

## Review

- [ ] No retired error aliases or parallel transaction mappers were added.
- [ ] Expected and unrecoverable conditions are classified at the narrowest
  boundary that understands them.
- [ ] Handlers propagate use-case errors without duplicating business mapping.
- [ ] Logs contain structured context but no secret or duplicate error event.
