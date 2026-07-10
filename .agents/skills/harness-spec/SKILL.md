---
name: harness-spec
description: Application Harn composition conventions for active PopRaKo ports. Use when changing src/harn.rs, AppHarn, or test port wiring.
---

# Harn Composition

`Harn<C, D, R, P, A, I, V>` is the active composition root. It stores the transaction driver, repository, prom queue, authentication, image pool, and effect developer behind an `Arc`, with read-only accessors. There is no active `Harness`, `TestHarness`, or ForwardRef bridge layer.

- Add a port bound to `Harn` only when the production composition needs it.
- Keep `Harn::new` argument order and accessor names aligned: `drive`, `repo`, `prom`, `auth`, `image_pool`, `develop`.
- Repository bounds belong on `R`; matching transactional bounds belong on `R::Transactional`. Keep the `C` context anchor in trait bounds.
- `AppHarn` in `src/api/http/state.rs` is the concrete production alias; `main.rs` builds its RDB/R2/JWT/effect values.
- Use `part_impl::repo::mock_impl` and module-local tests for usecase tests. Do not add another test composition layer without a demonstrated repeated need.
- Do not instrument constructors or plain accessors.

## Review checklist

- [ ] New production ports are wired in `main.rs`, `Harn`, and `AppHarn`.
- [ ] Repository and transactional bounds remain paired.
- [ ] Tests follow the existing mock-adapter pattern.
