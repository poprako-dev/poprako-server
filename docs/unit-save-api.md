# Unit Save API

`POST /api/v1/pages/{page_id}/units/save` applies one ordered batch of Unit
edits. Every request requires a `save_id` UUID query parameter. A successful save
returns `200 OK` with `data.created_unit_ids`, an array of `{ local_id, unit_id }`
structs (empty when nothing was created). The client then calls
`GET /api/v1/pages/{page_id}/units` to obtain the latest visible sequence and
counters.

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
      "next_id": null,
      "revision": {
        "is_proofread": true,
        "proofread_text": "proofread"
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
The server generates permanent IDs and returns explicit identity pairs.

`t_unit_save` records the authenticated user, page, save ID, typed-payload SHA-256
and created identities atomically with the edits. Retry an uncertain result with
the same save ID and identical edits. Successful replays return the original
identity pairs without applying edits or workflow effects again. A changed payload
with the same save ID returns 422; rolled-back batches consume no save identity.
Authorization is checked on replay. Receipts cascade with their page or user.

`last_translator_id` and `last_proofreader_id` are always derived from the
authenticated token and are not accepted from the client.

## Ordering and visibility

`next_id` identifies the Unit before which the edited Unit is placed. A null
successor places it at the tail. Patch first removes the target from the
complete linked list, then inserts it at the requested position. Patching a
hidden Unit restores it; deleting an unknown Unit or using an invalid anchor
returns 422.

The list endpoint returns visible Units only. Its array order is final and it
does not expose `index`, `next_id`, or tombstone state.

## Permissions

Translator may change translation content; proofreader may change revision
content. Either edit role may create, delete, move, or patch structural fields.
Sending a content field outside the authenticated role returns 403.
