-- Included by apply.sql or the rollback-only regression fixture.
-- The caller owns the transaction and exclusive assignment-table lock.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = current_schema()
            AND table_name = 't_assignment'
            AND column_name = 'f_assigned_admin_at'
    ) THEN
        DELETE FROM "t_assignment"
        WHERE "f_assigned_admin_at" IS NOT NULL
            AND "f_assigned_raw_provider_at" IS NULL
            AND "f_assigned_translator_at" IS NULL
            AND "f_assigned_proofreader_at" IS NULL
            AND "f_assigned_typesetter_at" IS NULL
            AND "f_assigned_redrawer_at" IS NULL
            AND "f_assigned_reviewer_at" IS NULL
            AND "f_assigned_publisher_at" IS NULL;
    END IF;
END $$;

DROP INDEX IF EXISTS "idx_assignment_chapter_admin_created_at";
DROP INDEX IF EXISTS "idx_assignment_user_admin_created_at";

ALTER TABLE "t_assignment" DROP COLUMN IF EXISTS "f_assigned_admin_at";
