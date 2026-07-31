---
name: error-handling-spec
description: Error construction and propagation rules for active PopRaKo Rust layers. Use when returning, mapping, or reviewing RegularError and HTTP errors.
---

# Error Handling

The active error surface is `crate::result::{Error, ExpectedVariant, RegularError, RegularResult}`. Do not introduce legacy `DomainError`, `UseCaseError`, `RootError`, or result aliases from removed architectures.

## Classify errors at the boundary that understands them

- Use `Error::Expected` for a client-visible argument, authentication, or perm condition. Select `ExpectedVariant::{Args, Auth, Perm}` and use the established `poprako_util::i18n::trl` key when a user-facing message is needed.
- Use `Error::Unrecoverable` for failed infrastructure, invalid internal state, or any condition that must not be presented as a client mistake.
- Preserve an existing error with `?`. Do not wrap it merely to add context if the caller already records the operation boundary.

```rust
return Err(RegularError::Expected {
    variant: ExpectedVariant::Perm,
    message: trl("error-forbidden"),
});
```

## Transaction and adapter boundaries

- Let `Drive::with_context` errors convert through the existing `From` impl in `result.rs`; do not invent a parallel transaction-error mapper.
- RDB adapters convert Diesel errors through `part_impl::shared::result` helpers. Keep database error classification in that adapter boundary.
- A missing row is expected only where the local adapter maps it to a specific translated key. Follow the neighboring operation.

## HTTP and logging boundaries

Handlers return `HttpResult<T>`, call the use case with `?`, and return successful values through `Accept as _`. Do not match application errors in a handler unless enforcing an HTTP-only condition, such as a path/body identifier mismatch.

Use `#[instrument(err, ...)]` on observable fallible operations according to `tracing-usage-spec`. Do not emit duplicate `tracing::error!` events while propagating with `?`; emit a direct event only when an error is intentionally consumed or retried.

## Review checklist

- [ ] The code uses the active `RegularError` / `RegularResult` surface.
- [ ] Expected errors have the right variant and established i18n key.
- [ ] Adapter and transaction errors use existing conversions.
- [ ] The HTTP handler propagates usecase errors unchanged.
