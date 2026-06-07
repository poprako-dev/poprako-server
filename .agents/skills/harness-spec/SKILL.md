---
name: harness-spec
description: Harness bridge layer conventions. No instrument on delegation; ForwardRef wiring; TestHarness must forward all usecase trait bounds.
---

# Harness Specification

The harness (`Harness` / `HarnessBase`) is a **bridge layer** that sits
between the API/HTTP layer and the infrastructure implementations. Its
sole purpose is to wire concrete infrastructure types (`Query`,
`JwtCodec`, `OssImagePool`) to the domain's trait contracts
(`Transactional`, `ImageGet`, `ImagePut`, `ImageDelete`, `TokenSign`,
`TokenParse`).

Every `ForwardRef` implementation in the harness is a **pure delegation**
that selects the underlying implementation without adding any business
logic, error handling, or I/O.

---

## 1. No `#[tracing::instrument]` on harness impl functions

Do **NOT** place `#[tracing::instrument]` on any `impl` method in the
harness module.

**Why**: Harness methods are pure delegation. The underlying
implementation (e.g., `Query`, `JwtCodec`, `OssImagePool`) already carries
its own `#[instrument]` or tracing at the call site. Adding another span
in the harness would produce a redundant wrapper span with no additional
diagnostic value — every harness call would generate two nested spans
pointing to the same operation.

**Do NOT add handwritten delegation when a forwarding marker exists:**
```rust
impl ImageGet for HarnessBase { /* BAD */ }
```

**Do:**
```rust
use crate::impl_forward_ref;

impl_forward_ref!(HarnessBase => OssImagePool, oss_pool, ImageGetForward);
```

---

## 2. Harness structure

`HarnessBase` holds the concrete infrastructure dependencies:

```rust
pub struct HarnessBase {
    rdb_query: RdbQuery,
    jwt_codec: JwtCodec,
    oss_pool: OssImagePool,
}
```

It implements:
- `ForwardRef<TransactionalForward>` and query forward markers to `RdbQuery`
- `ForwardRef<TokenSignForward>` and `ForwardRef<TokenParseForward>` to `JwtCodec`
- `ForwardRef<ImageGetForward>`, `ForwardRef<ImagePutForward>`, and `ForwardRef<ImageDeleteForward>` to `OssImagePool`

`Harness` wraps `HarnessBase` in an `Arc` and adds an `EffectSink`:

```rust
pub struct Harness {
    base: Arc<HarnessBase>,
    effect_sink: SharedEffectSink,
}
```

It implements the same forwarding markers to `HarnessBase` and implements
`EffectSink`.

---

## 3. Adding a new delegation

When adding a new infrastructure dependency to the harness:

1. Add the field to `HarnessBase`.
2. Add it to `HarnessBase`'s constructor call site in `Harness::new`.
3. Define a forwarding marker in the trait's own module.
4. Add a blanket trait implementation for `T: ForwardRef<ThatMarker>`.
5. Add an `impl_forward_ref!` entry in `src/harness.rs`.
6. Do **not** add `#[instrument]` to the delegation method.

```rust
// 1. Field
pub struct HarnessBase {
    // ...
    new_service: NewService,
}

// 5. Forwarding
impl_forward_ref!(HarnessBase => NewService, new_service, NewServiceForward);
```
