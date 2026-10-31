# Maintenance backlog

These open audits preserve the original two TODO requests. They do not establish
a current defect or authorize changing business contracts without an affected
design review. ObjDept-specific feedback is tracked in [to-fix](to-fix.md).

- [ ] **MAINT-01 — Audit resource safety limits.** Inventory request bodies,
  uploads, batch/list/include sizes, imports, text and archive expansion, cache
  memory, database/remote concurrency, actor work and retry retention. Existing
  controls include `src/config/{http,image,artwork,cache,repo,sched}.rs` and
  bounded URL concurrency in `poprako-obj-dept/src/pool.rs`. Record each resource's
  owner, unit, default, maximum, enforcement point, and boundary regression.
  Acceptance: each resource has an enforced bound or a reviewed reason for
  remaining unbounded; config defaults and rejection follow AGENTS.md; static
  analysis and small bounded probes cover gaps without destructive workloads.
- [ ] **MAINT-02 — Audit value and model ownership.** Review `src/value`,
  `src/model`, `src/data`, and repository/use-case signatures against
  [model and Shape boundaries](../.agents/skills/model-shape-boundaries/SKILL.md)
  and [DTO boundaries](../.agents/skills/data-dto-boundaries/SKILL.md). Start with
  business IDs, image/object identities, normalized writes and persisted read
  projections. Acceptance: record each mismatch with source paths and correct
  owner; validated values remain independent of persisted models, conversions
  retain validation, and HTTP DTOs stay outside repository contracts. Resolve
  concrete findings with affected tests and canonical checks, or retain each
  as a separately scoped task.

- [ ] **MAINT-04 — Restrict the public HTTP module surface.** Inventory the
  server binary and OpenAPI exporter's public entry points before reducing
  HTTP delivery modules to private implementation modules with explicit public
  entry points. Acceptance: both binaries and generated OpenAPI compile through
  the documented public surface, with handler/router internals exposed only
  where a real consumer needs them. Follow the repository's plain `pub` or
  private visibility rule; do not introduce restricted visibility.

The source TODO inventory now retains the visibility audit and versioned
cursor design. Diesel nullable changesets are stable three-state update design
notes; password verification intentionally returns a boolean verdict; Team
creation retains its existing hydrated response; translation export already
uses dedicated document models. Repository orchestration no longer appears in
`complex`, and connection acquisition remains private to the RDB adapter.

## Records retained across history replacement

The planned single-root Git history cannot preserve earlier implementation
evidence. Keep these records reachable until a replacement contains their
necessary operational facts; historical passing checks are not current gates.

| Record | Disposition |
| --- | --- |
| [Typed identity repair](history/pre-1.0/obj-dept-typed-identity-cutover.md) | Retain the dated production result, stopped API state and private server-local backup/report location independently of old commit IDs. This record does not authorize another repair. |
| [Prom completion change](history/pre-1.0/prom-completion-delete.md) | Retain UUID fencing/deletion regression evidence; the old [manual upgrade](prom-manual-upgrade.md) is historical; current support is defined in [the upgrade inventory](database-upgrade-support.md). |
| [Debug reduction](history/pre-1.0/debug-reduction.md) | Retain logging-redaction rationale and completed snapshot; active instrumentation rules live in `.agents/skills/tracing-usage-spec/SKILL.md`. |
| [Lint policy record](history/pre-1.0/workspace-lint-policy.md) | Retain dated validation evidence until relocated; current authority is `Cargo.toml`, `AGENTS.md` and `scripts/ci-lint-policy.sh`. |
| [Old ObjDept slice plan](history/pre-1.0/obj-dept-initial-plan.md) and [review](history/pre-1.0/obj-dept-initial-review.md) | Superseded PageImage-only context; migrate references in `plans/obj-dept-review-issues.md` before any removal. |
| Removed KeyMap scratchpad | Incomplete sketch removed after reference review; current typed forward mapping and validated restoration live in `poprako-obj-dept/src/key.rs`. |

- [x] **MAINT-03 — Retire obsolete documentation and fixture.** Six completed or
  superseded records now live under [the history index](history/README.md), with
  relative links updated. Typed-identity production repair and Prom implementation
  evidence remain in the snapshot independently of old SHAs. The incomplete
  KeyMap sketch and `tests/fixtures/db_bridge_v1_old.sql` were removed: the latter
  referenced retired avatar columns/old IDs and had no invoking test, script or
  supported rehearsal. No applied migration or rollback source was removed.
