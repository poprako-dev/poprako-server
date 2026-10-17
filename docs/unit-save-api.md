# Unit Save API

`POST /api/v1/pages/{page_id}/units/save` applies one ordered batch of Unit
edits. Every request requires a `save_id` UUID query parameter. A successful save
returns `200 OK` with `data.created_unit_ids`, containing the `local_id` and `unit_id`
pairs for newly created Units (an empty array for batches without creates).
The client applies this mapping to pending local edits, then calls
`GET /api/v1/pages/{page_id}/units` to obtain the latest identities, visible
sequence, and counters.

The save runs in a Serializable transaction. A concurrent write can return
`409 Conflict` with error `code: 8`; the server does not retry it. The client
must retry the complete edit batch. A failed Serializable transaction commits
none of that batch, so resending the same request is safe.

## Request

The request body is the edit array. Each edit uses the `edit` tag, and unknown
fields are rejected.

```json
[
    {
      "edit": "create",
      "local_id": "local-1",
      "next_id": "unit-b",
      "is_bubble": true,
      "coord": {
        "x_coord": 12.5,
        "y_coord": 33.0
      },
      "translation": {
        "translated_text": "new text"
      }
    },
    {
      "edit": "patch",
      "id": "unit-a",
      "next_id": { "type": "clear" },
      "revision": {
        "type": "assign",
        "value": {
          "is_proofread": true,
          "proofread_text": "proofread"
        }
      }
    },
    {
      "edit": "delete",
      "id": "unit-c"
    }
]
```

- `create` requires `local_id`, `is_bubble`, and `coord`. Missing `next_id`
  inserts at the tail. Translation and revision are optional.
- `patch` requires a permanent `id`. Missing or null `is_bubble` and `coord`
  leave the stored value unchanged.
- `is_flagged` is a shared review flag: Create defaults it to false; Patch
  accepts true/false, while omission or null preserves it. See
  [Unit review flags](unit-flagged-api.md) for page navigation and upgrade steps.
- `next_id`, `translation`, and `revision` are three-state patch fields:
  missing or null means Skip; use `{ "type": "clear" }` to Clear, or
  `{ "type": "assign", "value": ... }` to Assign.
- `delete` contains only the permanent target `id` and creates a tombstone.

Each batch contains 1–100 edits. `local_id` exists only within the current
batch so Create edits and their `next_id` references can be resolved together.
The server generates permanent IDs and returns their mapping in the save response.

Completed save tokens live in a process-local Moka segmented TinyLFU cache.
Each key is a parsed UUID (16 bytes); the value is an immutable, shared array of
created Unit identity pairs. No request digest, edit payload, user ID, or page ID
is retained in the cache. Clients must generate a globally unique save ID for
every new batch and reuse it only when retrying that batch.

The cache has a 256 MiB estimated-memory budget across 16 segments, with no TTL.
Each entry is charged for its mapping array, allocated string capacities, and
an allowance for cache and allocator overhead. A minimum charge of 537 bytes
also keeps the retained entry count below 500,000. Batches with many creates
consume more budget and shorten retention; batches without creates cache an
empty mapping. This budget estimates retained memory, not a hard process RSS
limit. Moka enforces weighted capacity on a best-effort basis, and temporary
overshoot, in-flight requests, response copies, and allocator retention can add
memory outside that estimate. There is no guaranteed retention period.

A retained token makes an authorized, valid retry a no-op returning the
original identity mapping. Payload equality is not checked: reusing a completed
token with different edits also skips those edits. Authorization and writable-state
checks still run. Fixed asynchronous lock stripes coordinate retries across
lookup, transaction completion, and token publication. Locks are independent of
cache eviction, and all clones of a repository share them.

Tokens are recorded only after a successful transaction; failed or rolled-back
batches consume no token. Database commit and cache publication are not atomic.
Restart, eviction, interruption after commit, or routing a retry to another
process can lose deduplication and execute an old batch again. This is
intentionally best-effort process-local retry protection. Archival is not
required to bound token count.

The legacy `t_unit_save` migration and table remain for migration-history
compatibility, but application saves no longer read or write that table. Existing
database receipts are not loaded into memory; their cleanup is separate from
deploying this application change.

`last_translator_id` and `last_proofreader_id` are always derived from the
authenticated token and are not accepted from the client.

## Ordering and visibility

`next_id` identifies the Unit before which the edited Unit is placed. A Create with a null
successor or a Patch with `{ "type": "clear" }` places it at the tail. Patch first removes the target from the
complete linked list, then inserts it at the requested position. Patching a
hidden Unit restores it; deleting an unknown Unit or using an invalid anchor
returns 422.

The list endpoint returns visible Units only. Its array order is final and it
does not expose `index`, `next_id`, or tombstone state.

## Permissions

Translator may change translation content; proofreader may change revision
content. Either edit role may create, delete, move, or patch structural fields.
Sending a content field outside the authenticated role returns 403.

## Sparse persistence

After computing the final sequence, the server writes changed successor links,
visibility transitions, and explicitly submitted content fields. An empty Patch
on a visible Unit, a move with no final link changes, or a repeated Delete of a
tombstone does not update that Unit or its `updated_at`. An empty Patch on a
hidden Unit still restores it. Neighbor links affected by a move are updated.

Create and Patch targeting the same new Unit are folded into one final insert
after sequence planning. Existing-row writes remain batched; columns unused by
the whole batch are omitted. Submitted content values are not compared with
stored content, so explicitly assigning the same value still counts as a write.

Skipped targets still undergo existence/page validation. Successful saves still
cache their completed save token and mapping and retain the existing counters,
activity, and submitted-content workflow behavior. This optimization does not make an
empty Unit write plan an empty save transaction.
