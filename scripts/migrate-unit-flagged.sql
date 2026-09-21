\set ON_ERROR_STOP on

-- Run explicitly against the intended database before starting the new server.
BEGIN;

LOCK TABLE "public"."t_unit" IN ACCESS EXCLUSIVE MODE;

ALTER TABLE "public"."t_unit"
    ADD COLUMN IF NOT EXISTS "f_is_flagged" BOOLEAN NOT NULL DEFAULT FALSE;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_catalog.pg_attribute AS attribute
        JOIN pg_catalog.pg_attrdef AS definition
          ON definition.adrelid = attribute.attrelid
         AND definition.adnum = attribute.attnum
        WHERE attribute.attrelid = 'public.t_unit'::regclass
          AND attribute.attname = 'f_is_flagged'
          AND NOT attribute.attisdropped
          AND attribute.atttypid = 'boolean'::regtype
          AND attribute.attnotnull
          AND attribute.attidentity = ''
          AND attribute.attgenerated = ''
          AND pg_catalog.pg_get_expr(
              definition.adbin, definition.adrelid
          ) = 'false'
    ) THEN
        RAISE EXCEPTION 't_unit.f_is_flagged must be BOOLEAN NOT NULL DEFAULT FALSE';
    END IF;
END
$$;

COMMIT;
