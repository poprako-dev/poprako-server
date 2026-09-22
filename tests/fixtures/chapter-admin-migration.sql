-- Run with psql --set ON_ERROR_STOP=1 against the disposable db_poprako_ci.
DO $$
BEGIN
    IF current_database() <> 'db_poprako_ci' THEN
        RAISE EXCEPTION 'This regression requires db_poprako_ci';
    END IF;
END $$;

BEGIN;
SET LOCAL search_path = public, pg_temp;
LOCK TABLE t_assignment IN ACCESS EXCLUSIVE MODE;

\ir ../../scripts/chapter-admin-upgrade/tests/source.sql

INSERT INTO "t_workset" ("f_id", "f_team_id", "f_index", "f_name")
VALUES ('ci-admin-workset', 'team-11111111111', 987654, 'migration regression');

INSERT INTO "t_comic" ("f_id", "f_workset_id", "f_index", "f_title", "f_author", "f_creator_id")
VALUES ('ci-admin-comic', 'ci-admin-workset', 0, 'migration regression', 'test', 'user-11111111111');

INSERT INTO "t_chapter" ("f_id", "f_comic_id", "f_index", "f_subtitle", "f_creator_id")
VALUES
    ('ci-admin-only', 'ci-admin-comic', 0, 'admin', 'user-11111111111'),
    ('ci-admin-mixed', 'ci-admin-comic', 1, 'mixed', 'user-11111111111'),
    ('ci-admin-worker', 'ci-admin-comic', 2, 'worker', 'user-11111111111');

INSERT INTO "t_assignment" (
    "f_id", "f_chapter_id", "f_user_id", "f_assigned_admin_at", "f_assigned_translator_at"
) VALUES
    ('ci-admin-only', 'ci-admin-only', 'user-11111111111', NOW(), NULL),
    ('ci-admin-mixed', 'ci-admin-mixed', 'user-11111111111', NOW(), '2026-01-01 UTC'),
    ('ci-admin-worker', 'ci-admin-worker', 'user-11111111111', NULL, '2026-02-01 UTC');

\ir ../../scripts/chapter-admin-upgrade/changes.sql
\ir ../../migrations/2026-07-17-083511-0001_index-assignment-table/up.sql
\ir ../../scripts/chapter-admin-upgrade/changes.sql

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM "t_assignment" WHERE "f_id" = 'ci-admin-only') THEN
        RAISE EXCEPTION 'admin-only assignment survived';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM "t_assignment"
        WHERE "f_id" = 'ci-admin-mixed' AND "f_assigned_translator_at" = '2026-01-01 UTC'
    ) OR NOT EXISTS (
        SELECT 1 FROM "t_assignment"
        WHERE "f_id" = 'ci-admin-worker' AND "f_assigned_translator_at" = '2026-02-01 UTC'
    ) THEN
        RAISE EXCEPTION 'worker assignments or timestamps changed';
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = current_schema() AND table_name = 't_assignment'
            AND column_name = 'f_assigned_admin_at'
    ) OR EXISTS (
        SELECT 1 FROM pg_indexes WHERE schemaname = current_schema()
            AND indexname IN ('idx_assignment_chapter_admin_created_at', 'idx_assignment_user_admin_created_at')
    ) THEN
        RAISE EXCEPTION 'obsolete column or indexes survived';
    END IF;
END $$;

\ir ../../scripts/chapter-admin-upgrade/tests/source.sql

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM "t_assignment" WHERE "f_assigned_admin_at" IS NOT NULL)
        OR EXISTS (SELECT 1 FROM "t_assignment" WHERE "f_id" = 'ci-admin-only') THEN
        RAISE EXCEPTION 'structural rollback restored administrator identities';
    END IF;

    IF (SELECT COUNT(*) FROM pg_indexes WHERE schemaname = current_schema()
        AND indexname IN ('idx_assignment_chapter_admin_created_at', 'idx_assignment_user_admin_created_at')) <> 2 THEN
        RAISE EXCEPTION 'structural rollback did not restore both indexes';
    END IF;
END $$;

\ir ../../scripts/chapter-admin-upgrade/changes.sql

ROLLBACK;
