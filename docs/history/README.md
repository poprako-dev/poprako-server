# Pre 1.0 implementation records

These dated records preserve completed implementation work and necessary
operational evidence across the 1.0.0 Git history replacement. Their old
branches, commit IDs, validation commands and production observations describe
the original work, not the current deployment contract or fresh authorization.

- [ObjDept initial plan](pre-1.0/obj-dept-initial-plan.md) and
  [initial review](pre-1.0/obj-dept-initial-review.md): superseded PageImage slice.
- [Typed identity cutover](pre-1.0/obj-dept-typed-identity-cutover.md): retained
  production repair outcome and backup/report locations; do not repeat it from
  this historical record.
- [Prom completion change](pre-1.0/prom-completion-delete.md): UUID fencing and
  deletion implementation evidence. Current upgrade support is defined in
  [the upgrade inventory](../database-upgrade-support.md).
- [Debug reduction](pre-1.0/debug-reduction.md): completed diagnostic-redaction work.
- [Workspace lint policy](pre-1.0/workspace-lint-policy.md): historical validation
  record. Current policy lives in Cargo.toml, AGENTS.md and checked-in CI scripts.

The incomplete KeyMap scratchpad and unreferenced old database bridge fixture
were removed after reference review. Neither was an applied migration source
or a supported upgrade path. Active migrations and their genuine rollback
definitions remain in the repository.
