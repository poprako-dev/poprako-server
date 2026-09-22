\set ON_ERROR_STOP on
\pset pager off
-- Stop all old business processes and take a restorable backup before running.
BEGIN;
SET LOCAL search_path = public, pg_temp;
SET LOCAL lock_timeout = '5s';
SET LOCAL statement_timeout = '5min';
DO $$
BEGIN
    IF current_database() NOT IN ('db_poprako_server_prod', 'db_poprako_ci') THEN
        RAISE EXCEPTION 'Expected db_poprako_server_prod or disposable db_poprako_ci';
    END IF;
END $$;

LOCK TABLE t_assignment IN ACCESS EXCLUSIVE MODE;

-- Preserve every retained row, including IDs and all timestamps, for comparison.
CREATE TEMP TABLE chapter_admin_retained_before ON COMMIT DROP AS
SELECT to_jsonb(a) - 'f_assigned_admin_at' AS value
FROM t_assignment a
WHERE to_jsonb(a)->>'f_assigned_admin_at' IS NULL
    OR f_assigned_raw_provider_at IS NOT NULL
    OR f_assigned_translator_at IS NOT NULL
    OR f_assigned_proofreader_at IS NOT NULL
    OR f_assigned_typesetter_at IS NOT NULL
    OR f_assigned_redrawer_at IS NOT NULL
    OR f_assigned_reviewer_at IS NOT NULL
    OR f_assigned_publisher_at IS NOT NULL;

\ir changes.sql

DO $$
BEGIN
    IF EXISTS (
        (SELECT value FROM chapter_admin_retained_before
         EXCEPT ALL SELECT to_jsonb(a) FROM t_assignment a)
        UNION ALL
        (SELECT to_jsonb(a) FROM t_assignment a
         EXCEPT ALL SELECT value FROM chapter_admin_retained_before)
    ) THEN
        RAISE EXCEPTION 'Retained assignments changed; upgrade rolled back';
    END IF;
END $$;
\ir verify.sql
COMMIT;
