# API shape contract

Enforces the PopRaKo API boundary rules:

- Data DTOs in `src/data/**` must use named fields. Tuple DTOs and raw
  `BTreeMap`/`HashMap` fields or aliases are rejected.
- Every route handler in `src/api/http/handler/**` must return
  `HttpResult<T>` or `HttpNoContent`. Handlers returning raw `Response`,
  `Json<T>`, or other result types are rejected.

Download and operational handlers remain `HttpResult<HttpRawBody>`: the typed
raw body preserves a non-enveloped response while keeping the unified handler
return type.

Codes:

- `SHAPE001` — public data tuple struct.
- `SHAPE002` — public data field with a raw map type.
- `SHAPE003` — public data type alias with a raw map type.
- `HTTP001` — route handler returns a type other than `HttpResult`/`HttpNoContent`.

```sh
python3 linters-extra/api-shape-contract/check.py --root .
```
