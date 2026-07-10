# Unit Save API

`POST /api/v1/pages/{page_id}/units/save` applies an ordered sequence of unit
create, save, and delete operations. The server serializes application of the
operations and returns current counters plus mappings for newly created units.

## Request

```json
{
  "page_id": "page-1",
  "diff": {
    "page_id": "page-1",
    "opers": []
  }
}
```

Both body `page_id` values must equal the `{page_id}` path parameter. Each
operation is a tagged, flat JSON object; unknown fields are rejected.

| Operation | Identifier | Placement | Payload |
| --- | --- | --- | --- |
| `create` | `local_id` | optional `before_id` | required |
| `save` | `id` | optional `before_id` | required |
| `delete` | `id` | none | none |

The shared payload fields are `is_bubble`, `is_proofread`, `x_coord`,
`y_coord`, `translated_text`, `last_translator_id`, `proofread_text`, and
`last_proofreader_id`. When translated or proofread text is non-null, its
corresponding editor identifier must also be non-null.

```json
{
  "oper": "create",
  "local_id": "local-uuid-1",
  "before_id": "unit-b",
  "is_bubble": true,
  "is_proofread": false,
  "x_coord": 12.5,
  "y_coord": 33.0,
  "translated_text": "new text",
  "last_translator_id": "user-1",
  "proofread_text": null,
  "last_proofreader_id": null
}
```

```json
{
  "oper": "save",
  "id": "unit-a",
  "before_id": "unit-c",
  "is_bubble": true,
  "is_proofread": true,
  "x_coord": 10.0,
  "y_coord": 20.0,
  "translated_text": "translated",
  "last_translator_id": "user-1",
  "proofread_text": "proofread",
  "last_proofreader_id": "user-2"
}
```

```json
{
  "oper": "delete",
  "id": "unit-a"
}
```

## Ordering

`before_id` places a created or saved unit immediately before the referenced
unit. Omit it, use `null`, reference the same unit, or reference a missing or
deleted unit to place the unit at the end. There is no separate move operation:
a `save` with a `before_id` changes both stored data and position.

Deleting a missing unit is valid and idempotent. A save can restore a unit that
was deleted by a concurrent request; the server then applies its payload and
ordering rule.

## Response

```json
{
  "local_id_mappers": [
    {
      "local_id": "local-uuid-1",
      "unit_id": "snowflake-server-id"
    }
  ],
  "total_unit_count": 3,
  "translated_unit_count": 2,
  "proofread_unit_count": 1
}
```

`local_id_mappers` contains only mappings created by `create` operations. Use
it to replace client-local IDs, then call
`GET /api/v1/pages/{page_id}/units` when the latest complete ordering is
needed.
