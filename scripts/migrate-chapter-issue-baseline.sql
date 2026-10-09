\set ON_ERROR_STOP on

-- Run explicitly before starting the new server, with business processes stopped.
BEGIN;

SET LOCAL search_path = public, pg_temp;
SET LOCAL lock_timeout = '5s';
SET LOCAL statement_timeout = '5min';

LOCK TABLE t_chapter, t_chapter_artwork IN ACCESS EXCLUSIVE MODE;

ALTER TABLE t_chapter
    ADD COLUMN IF NOT EXISTS f_confirmed_artwork_version BIGINT
        CHECK (f_confirmed_artwork_version BETWEEN 0 AND 4294967295);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_catalog.pg_attribute AS attribute
        WHERE attribute.attrelid = 'public.t_chapter'::regclass
            AND attribute.attname = 'f_confirmed_artwork_version'
            AND NOT attribute.attisdropped
            AND attribute.atttypid = 'bigint'::regtype
            AND NOT attribute.attnotnull
            AND NOT attribute.atthasdef
            AND attribute.attidentity = ''
            AND attribute.attgenerated = ''
    ) THEN
        RAISE EXCEPTION 't_chapter.f_confirmed_artwork_version must be nullable BIGINT without a default';
    END IF;
END
$$;

-- Preserve previously recorded confirmations and unrelated Chapter data.
UPDATE t_chapter AS chapter
SET f_confirmed_artwork_version = artwork.f_version
FROM t_chapter_artwork AS artwork
WHERE chapter.f_id = artwork.f_id
    AND artwork.f_is_uploaded = TRUE
    AND artwork.f_key IS NOT NULL
    AND artwork.f_hash IS NOT NULL
    AND artwork.f_ext IS NOT NULL
    AND chapter.f_confirmed_artwork_version IS NULL;

COMMIT;
