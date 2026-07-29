\set ON_ERROR_STOP on

BEGIN;

LOCK TABLE "t_unit" IN ACCESS EXCLUSIVE MODE;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 't_unit'
          AND column_name = 'f_index'
    ) THEN
        RAISE EXCEPTION 't_unit.f_index is missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 't_unit'
          AND column_name IN ('f_next_id', 'f_hidden_at')
    ) THEN
        RAISE EXCEPTION 'Unit v2 columns already exist';
    END IF;
END
$$;

ALTER TABLE "t_unit"
    ADD COLUMN "f_next_id" TEXT,
    ADD COLUMN "f_hidden_at" TIMESTAMPTZ;

WITH ordered_units AS (
    SELECT
        "f_id",
        LEAD("f_id") OVER (
            PARTITION BY "f_page_id"
            ORDER BY "f_index", "f_id"
        ) AS "f_next_id"
    FROM "t_unit"
)
UPDATE "t_unit" AS unit
SET "f_next_id" = ordered_units."f_next_id"
FROM ordered_units
WHERE ordered_units."f_id" = unit."f_id";

DO $$
DECLARE
    invalid_page_count BIGINT;
BEGIN
    WITH RECURSIVE
    page_stats AS (
        SELECT
            "f_page_id",
            COUNT(*) AS node_count,
            COUNT(*) FILTER (
                WHERE NOT EXISTS (
                    SELECT 1
                    FROM "t_unit" AS predecessor
                    WHERE predecessor."f_page_id" = unit."f_page_id"
                      AND predecessor."f_next_id" = unit."f_id"
                )
            ) AS head_count,
            COUNT(*) FILTER (WHERE "f_next_id" IS NULL) AS tail_count
        FROM "t_unit" AS unit
        GROUP BY "f_page_id"
    ),
    heads AS (
        SELECT unit."f_page_id", unit."f_id"
        FROM "t_unit" AS unit
        WHERE NOT EXISTS (
            SELECT 1
            FROM "t_unit" AS predecessor
            WHERE predecessor."f_page_id" = unit."f_page_id"
              AND predecessor."f_next_id" = unit."f_id"
        )
    ),
    chain AS (
        SELECT
            heads."f_page_id",
            heads."f_id",
            ARRAY[heads."f_id"] AS visited
        FROM heads

        UNION ALL

        SELECT
            chain."f_page_id",
            unit."f_next_id",
            chain.visited || unit."f_next_id"
        FROM chain
        JOIN "t_unit" AS unit
          ON unit."f_page_id" = chain."f_page_id"
         AND unit."f_id" = chain."f_id"
        WHERE unit."f_next_id" IS NOT NULL
          AND NOT unit."f_next_id" = ANY(chain.visited)
    ),
    traversed AS (
        SELECT "f_page_id", COUNT(*) AS node_count
        FROM chain
        GROUP BY "f_page_id"
    ),
    invalid_pages AS (
        SELECT page_stats."f_page_id"
        FROM page_stats
        LEFT JOIN traversed
          ON traversed."f_page_id" = page_stats."f_page_id"
        WHERE page_stats.head_count <> 1
           OR page_stats.tail_count <> 1
           OR traversed.node_count IS DISTINCT FROM page_stats.node_count

        UNION

        SELECT unit."f_page_id"
        FROM "t_unit" AS unit
        JOIN "t_unit" AS next_unit
          ON next_unit."f_id" = unit."f_next_id"
        WHERE next_unit."f_page_id" <> unit."f_page_id"

        UNION

        SELECT unit."f_page_id"
        FROM "t_unit" AS unit
        WHERE unit."f_next_id" = unit."f_id"

        UNION

        SELECT unit."f_page_id"
        FROM "t_unit" AS unit
        WHERE unit."f_next_id" IS NOT NULL
        GROUP BY unit."f_page_id", unit."f_next_id"
        HAVING COUNT(*) > 1
    )
    SELECT COUNT(*) INTO invalid_page_count
    FROM invalid_pages;

    IF invalid_page_count <> 0 THEN
        RAISE EXCEPTION 'Unit v2 chain validation failed for % pages',
            invalid_page_count;
    END IF;
END
$$;

DROP INDEX IF EXISTS "uidx_unit_page_id_index";

ALTER TABLE "t_unit"
    ADD FOREIGN KEY ("f_next_id")
        REFERENCES "t_unit" ("f_id")
        DEFERRABLE INITIALLY DEFERRED,
    DROP COLUMN "f_index";

COMMIT;
