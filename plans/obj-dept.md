# ObjDept Implementation Plan

## Current stage

Implement and verify the smallest complete infrastructure slice without
cutting existing Page traffic over.

### 1. Preserve the lower RDB crate

- keep the name `poprako-rdb-core`;
- keep connection, transaction, level, and error mechanics there;
- ensure it has no ObjDept dependency or object knowledge.

### 2. Define ObjDept contracts

- keep `ObjKey`, `ObjSpec`, `ObjMeta`, and `ObjSlot` in
  `poprako-obj-dept`;
- define `GetObjMeta`, `GenObjUrl`, `GenObjSlot`, and `DelObjs`;
- compose them as `ObjDept<B, C>`;
- expose the optional typed RDB seam under `rdb_impl`.

### 3. Generate typed static access

- provide `#[obj_dept(Image, t_page_image)]`;
- emit one static bind type and typed Diesel operations;
- reject malformed declarations at compile time;
- add macro expansion tests;
- forbid raw SQL and runtime table selection.

### 4. Implement the server total object

- declare `PageImage` in `poprako-server`;
- implement its remote adapter locally;
- create one non-generic `RdbObjDept`;
- implement all four operations through static generic implementations;
- wire one total object through `Harn` and `main`;
- close it during graceful shutdown.

### 5. Add isolated persistence

- create `t_page_image` with only mechanical constraints;
- create `t_obj_prom_task` separately from `t_local_message`;
- generate Diesel schema from migrations;
- retain old `t_page` image columns because Page is not cut over yet.

### 6. Complete Check/Delete actor

- create tasks in the same transaction as latest-state changes;
- claim by exact static bind;
- use leases for claims and settlement;
- reclaim expired processing work;
- perform remote HEAD/delete outside transactions;
- use typed compare-and-set transitions;
- retry unresolved remote failures without an attempt limit;
- retain invalid state for operator repair.

### 7. Verify the slice

- test nullable-state classification and stale/current/future versions;
- test macro-generated typed access;
- test total capability composition;
- run `cargo fmt --all --check`;
- run `cargo check --all-features`;
- run focused crate and server tests;
- run the complete custom linter suite;
- scan the ObjDept paths for forbidden untyped Diesel APIs;
- run the checked-in CI entry point.

### 8. Read-only review

Ask subagents to review:

- dependency direction and aggregate ownership;
- typed Diesel boundaries;
- Check/Delete state closure and races;
- migration and task-table isolation;
- naming and file-layout compliance.

Resolve blockers, rerun validation, then record the final status.

## Deferred Page cutover

Do not switch Page producers, reads, publish, or deletion in this stage. The
legacy physical key is not derivable from the new key grammar. A later plan
must first choose and prove a safe movement strategy for existing verified
objects.

The deferred stage will eventually:

- move existing verified Page objects and metadata;
- call `GenObjSlot<PageImage>` from both Page reservation paths;
- call `GetObjMeta<PageImage>` and `GenObjUrl<PageImage>` from Page reads;
- call `DelObjs<PageImage>` from publish and owner deletion transactions;
- drain and remove only legacy Page image messages;
- remove old `t_page` image columns after its rollback window.

Inventory-based late-object repair remains outside this plan.

## Stop conditions

Stop instead of improvising if the implementation requires:

- a dependency from `poprako-rdb-core` to ObjDept;
- a generic `RdbObjDept` self type;
- more than one total object in `main` or `Harn`;
- a runtime object-kind registry;
- raw SQL or runtime table selection;
- ObjDept tasks in `t_local_message`;
- physical deletion inside `DelObjs`;
- remote I/O held inside a task-state transaction;
- deletion of a current version;
- retry limits that erase unresolved deletion debt;
- an unapproved Page cutover or data-movement mechanism;
- mutable business policy encoded as database constraints.
