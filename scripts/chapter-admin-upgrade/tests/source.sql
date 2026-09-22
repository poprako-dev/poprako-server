-- Test-only reconstruction of the old nullable column and indexes.
ALTER TABLE "t_assignment" ADD COLUMN IF NOT EXISTS "f_assigned_admin_at" TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS "idx_assignment_chapter_admin_created_at"
    ON "t_assignment" ("f_chapter_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_admin_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_assignment_user_admin_created_at"
    ON "t_assignment" ("f_user_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_admin_at" IS NOT NULL;
