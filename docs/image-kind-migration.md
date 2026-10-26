# Historical image task payload conversion

> Historical payload conversion only. Current Prom topics and payloads use a different contract. This is not a supported 1.0.0 upgrade; see [upgrade support](database-upgrade-support.md).

```sql
UPDATE "t_local_message"
SET "f_payload" = ("f_payload" - 'resource_kind')
    || jsonb_build_object('image_kind', "f_payload" -> 'resource_kind')
WHERE "f_topic" = 'image'
  AND "f_payload" ? 'resource_kind';
```
