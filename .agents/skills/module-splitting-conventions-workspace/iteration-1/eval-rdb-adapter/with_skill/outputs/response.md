The measured unsplit size is 910 lines, so a line-limit split is warranted. I would not preserve `statement` and `impl` as module boundaries: those are technical buckets, and `statement` is explicitly forbidden as a child-module name.

Proposed split:

- Parent RDB adapter module — projected 180 lines. Keep the adapter type, repository trait implementations that define the layer entry point, shared connection/transaction plumbing, transaction begin/commit/rollback coordination, and dispatch into the owned capabilities.
- `account` — projected 470 lines. Own all account lifecycle and profile implementation: create/read/update/delete or status-transition SQL, profile queries and mutations, their row/input mappings, and the private execution functions that use those statements.
- `avatar` — projected 260 lines. Own the complete avatar-reservation implementation: reserve, inspect, consume/finalize, release/expire SQL, associated mappings, and its private execution functions.

The projections total the current 910 lines before minor import/module-declaration movement; every resulting Rust file remains strictly below 600 lines. This is the smallest stable split that gives each extracted child one complete business responsibility while leaving transaction coordination at the architectural entry point. It does not introduce new public exports or change behavior, errors, transaction boundaries, or tracing.

Please confirm this exact `parent + account + avatar` split before I move or edit anything. After confirmation, I would perform the extraction, run the module-split audit again, and then run the repository formatting, compile, test, and lint checks.
