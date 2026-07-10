-- Your SQL goes here
ALTER TABLE "t_comic"
    ADD COLUMN IF NOT EXISTS "f_composed_title" TEXT NOT NULL DEFAULT '';

ALTER TABLE "t_chapter"
    ADD COLUMN IF NOT EXISTS "f_uploaded_at" TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS "f_translating_at" TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS "f_translated_at" TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS "f_proofreading_at" TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS "f_proofread_at" TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS "f_typesetting_at" TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS "f_typeset_at" TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS "f_reviewed_at" TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS "f_published_at" TIMESTAMPTZ;

ALTER TABLE "t_chapter"
    DROP COLUMN IF EXISTS "f_stages";

UPDATE "t_comic"
SET "f_composed_title" = CONCAT("f_index" + 1, ' ', "f_author", ' ', "f_title")
WHERE "f_composed_title" = '';

CREATE INDEX IF NOT EXISTS "idx_comic_composed_title_trgm"
    ON "t_comic" USING GIN ("f_composed_title" gin_trgm_ops);

CREATE INDEX IF NOT EXISTS "idx_comic_workset_last_active"
    ON "t_comic" ("f_workset_id", "f_last_active_at" DESC);

CREATE INDEX IF NOT EXISTS "idx_member_nickname_trgm"
    ON "t_member" USING GIN ("f_user_nickname" gin_trgm_ops);

CREATE INDEX IF NOT EXISTS "idx_member_team_raw_provider_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_raw_provider_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_translator_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_translator_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_proofreader_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_proofreader_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_typesetter_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_typesetter_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_redrawer_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_redrawer_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_reviewer_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_reviewer_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_publisher_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_publisher_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_admin_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_admin_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_member_team_bot_last_active"
    ON "t_member" ("f_team_id", "f_user_last_active_at" DESC)
    WHERE "f_assigned_bot_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_team_created_at"
    ON "t_team" ("f_created_at" DESC);

CREATE INDEX IF NOT EXISTS "idx_assignment_invitation_chapter_created_at"
    ON "t_assignment_invitation" ("f_chapter_id", "f_created_at" DESC);

CREATE INDEX IF NOT EXISTS "idx_assignment_invitation_chapter_pending_created_at"
    ON "t_assignment_invitation" ("f_chapter_id", "f_pending", "f_created_at" DESC);
