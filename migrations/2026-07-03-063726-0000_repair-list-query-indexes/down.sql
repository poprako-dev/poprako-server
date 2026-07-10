-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS "idx_assignment_invitation_chapter_pending_created_at";
DROP INDEX IF EXISTS "idx_assignment_invitation_chapter_created_at";
DROP INDEX IF EXISTS "idx_team_created_at";
DROP INDEX IF EXISTS "idx_member_team_bot_last_active";
DROP INDEX IF EXISTS "idx_member_team_admin_last_active";
DROP INDEX IF EXISTS "idx_member_team_publisher_last_active";
DROP INDEX IF EXISTS "idx_member_team_reviewer_last_active";
DROP INDEX IF EXISTS "idx_member_team_redrawer_last_active";
DROP INDEX IF EXISTS "idx_member_team_typesetter_last_active";
DROP INDEX IF EXISTS "idx_member_team_proofreader_last_active";
DROP INDEX IF EXISTS "idx_member_team_translator_last_active";
DROP INDEX IF EXISTS "idx_member_team_raw_provider_last_active";
DROP INDEX IF EXISTS "idx_member_nickname_trgm";
DROP INDEX IF EXISTS "idx_comic_workset_last_active";
DROP INDEX IF EXISTS "idx_comic_composed_title_trgm";

ALTER TABLE "t_comic"
    DROP COLUMN IF EXISTS "f_composed_title";

ALTER TABLE "t_chapter"
    ADD COLUMN IF NOT EXISTS "f_stages" INTEGER NOT NULL DEFAULT 0;

ALTER TABLE "t_chapter"
    DROP COLUMN IF EXISTS "f_published_at",
    DROP COLUMN IF EXISTS "f_reviewed_at",
    DROP COLUMN IF EXISTS "f_typeset_at",
    DROP COLUMN IF EXISTS "f_typesetting_at",
    DROP COLUMN IF EXISTS "f_proofread_at",
    DROP COLUMN IF EXISTS "f_proofreading_at",
    DROP COLUMN IF EXISTS "f_translated_at",
    DROP COLUMN IF EXISTS "f_translating_at",
    DROP COLUMN IF EXISTS "f_uploaded_at";
