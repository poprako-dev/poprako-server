```sql
UPDATE "t_local_message"
SET "f_payload" = ("f_payload" - 'resource_kind')
    || jsonb_build_object('image_kind', "f_payload" -> 'resource_kind')
WHERE "f_topic" = 'image'
  AND "f_payload" ? 'resource_kind';
```
