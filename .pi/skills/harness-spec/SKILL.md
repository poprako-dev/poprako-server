---
name: harness-spec
description: |
  Conventions for the harness layer in poprako-r. The harness
  (`src/api/harness.rs`) is a bridge/facade that wires infrastructure
  implementations to the domain trait contracts. Use whenever modifying
  or adding `impl` blocks in the harness module.
---

# Harness Specification

The harness (`Harness` / `HarnessInner`) is a **bridge layer** that sits
between the API/HTTP layer and the infrastructure implementations. Its
sole purpose is to wire concrete infrastructure types (`Query`,
`JwtCodec`, `OssImagePool`) to the domain's trait contracts
(`Transactional`, `ImageGet`, `ImagePut`, `ImageDelete`, `TokenSign`,
`TokenParse`).

Every `impl` block in the harness is a **pure delegation** — it forwards
the call to the underlying implementation without adding any business
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

**Do NOT:**
```rust
use tracing::instrument;

#[async_trait]
impl ImageGet for HarnessInner {
    #[instrument(skip(self))]           // BAD — pure delegation
    async fn get_signed(&self, key: &str) -> DomainResult<url::Url> {
        self.image_pool.get_signed(key).await
    }
}
```

**Do:**
```rust
#[async_trait]
impl ImageGet for HarnessInner {
    async fn get_signed(&self, key: &str) -> DomainResult<url::Url> {
        self.image_pool.get_signed(key).await
    }
}
```

---

## 2. Harness structure

`HarnessInner` holds the concrete infrastructure dependencies:

```rust
pub struct HarnessInner {
    query: Query,
    jwt_codec: JwtCodec,
    image_pool: OssImagePool,
}
```

It implements:
- `Deref<Target = Query>` — so `*Query` traits are available via deref
- `Transactional` — delegates to `self.query.run_in_transaction`
- Domain external traits (`ImageGet`, `ImagePut`, `ImageDelete`, `TokenSign`, `TokenParse`) — each delegates to the corresponding inner field

`Harness` wraps `HarnessInner` in an `Arc` and adds an `EffectSink`:

```rust
pub struct Harness {
    inner: Arc<HarnessInner>,
    effect_sink: Arc<AsyncEffectSink>,
}
```

It implements `Deref<Target = HarnessInner>`, `DerefTo<Target = HarnessInner>`, and `EffectSink`.

---

## 3. Adding a new delegation

When adding a new infrastructure dependency to the harness:

1. Add the field to `HarnessInner`.
2. Add it to `HarnessInner`'s constructor call site in `Harness::new`.
3. Implement the domain trait for `HarnessInner` with a single-line delegation.
4. Do **not** add `#[instrument]` to the delegation method.

```rust
// 1. Field
pub struct HarnessInner {
    // ...
    new_service: NewService,
}

// 3. Delegation
#[async_trait]
impl NewServiceTrait for HarnessInner {
    async fn do_thing(&self, arg: &str) -> DomainResult<Thing> {
        self.new_service.do_thing(arg).await
    }
}
```
