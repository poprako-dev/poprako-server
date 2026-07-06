# Unit Save API

`POST /api/v1/pages/{page_id}/units/save` saves a page unit operation stream.

This endpoint never accepts `index`, `order`, `candidate_order`, `cand_order`, or
any equivalent client-provided absolute ordering field. The only public page
order is the array order returned by `GET /api/v1/pages/{page_id}/units`.

The database `index` is internal storage only. The server derives it from the
resulting page sequence after applying operations.

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

Both `page_id` values must match the path page ID.

`opers` is an ordered event stream. The server applies operations exactly in
array order inside the page transaction.

## Unit Payload

Every operation that writes or restores a unit must carry a complete unit
payload:

```json
{
  "is_bubble": true,
  "is_proofread": false,
  "x_coord": 12.5,
  "y_coord": 33.0,
  "translated_text": "text",
  "last_translator_id": "user-1",
  "proofread_text": null,
  "last_proofreader_id": null
}
```

This is required for concurrent replay. If another submission deleted the unit
first, a later save or move-before operation can still restore the unit from the
complete payload.

## Operations

### Create

Creates a new unit from a local client ID.

```json
{
  "oper": "create",
  "local_id": "local-uuid-1",
  "move_before": "unit-b",
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

`move_before` is optional. When omitted or `null`, the new unit is inserted at
the tail. When set to an existing unit ID, the new unit is inserted before that
unit. If the target no longer exists, the server inserts at the tail.

The response returns a local-to-server ID mapping.

### Save

Writes a complete payload for an existing unit and keeps its current position.
If the unit was deleted by an earlier operation, the server restores it at the
tail.

```json
{
  "oper": "save",
  "id": "unit-a",
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

### Move Before

Writes a complete payload and moves or restores the unit before another unit.
This is a save operation with relative positioning, not a payload-free move.

```json
{
  "oper": "move_before",
  "id": "unit-a",
  "move_before": "unit-c",
  "is_bubble": true,
  "is_proofread": false,
  "x_coord": 10.0,
  "y_coord": 20.0,
  "translated_text": "translated",
  "last_translator_id": "user-1",
  "proofread_text": null,
  "last_proofreader_id": null
}
```

`move_before: null` means move to the tail. If the target ID has already been
deleted, the server also treats it as tail. If `move_before` equals the subject
unit ID, the server treats it as tail.

### Delete

Deletes a unit from the page. Delete is the only operation that does not carry a
complete unit payload.

```json
{
  "oper": "delete",
  "id": "unit-a"
}
```

Deleting an already missing unit is a no-op.

## Response

The response contains only local ID mappings and refreshed counters:

```json
{
  "local_id_mappers": [
    {
      "local_id": "local-uuid-1",
      "unit_id": "unit-server-id"
    }
  ],
  "total_unit_count": 3,
  "translated_unit_count": 2,
  "proofread_unit_count": 1
}
```

It does not return unit order. Call `GET /api/v1/pages/{page_id}/units` and use
the returned array order.
