CREATE TABLE IF NOT EXISTS "t_assignment" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_chapter_id"                 TEXT        NOT NULL REFERENCES "t_chapter" ("f_id") ON DELETE RESTRICT,
    "f_user_id"                    TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_assigned_raw_provider_at"   TIMESTAMPTZ,
    "f_assigned_translator_at"     TIMESTAMPTZ,
    "f_assigned_proofreader_at"    TIMESTAMPTZ,
    "f_assigned_typesetter_at"     TIMESTAMPTZ,
    "f_assigned_redrawer_at"       TIMESTAMPTZ,
    "f_assigned_reviewer_at"       TIMESTAMPTZ,
    "f_assigned_publisher_at"      TIMESTAMPTZ,
    "f_assigned_admin_at"          TIMESTAMPTZ,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_assignment_chapter_id_user_id"
    ON "t_assignment" ("f_chapter_id", "f_user_id");

CREATE INDEX IF NOT EXISTS "idx_assignment_chapter_id_created_at_desc"
    ON "t_assignment" ("f_chapter_id", "f_created_at" DESC, "f_id" ASC);

CREATE INDEX IF NOT EXISTS "idx_assignment_user_id_created_at_desc"
    ON "t_assignment" ("f_user_id", "f_created_at" DESC, "f_id" ASC);

CREATE INDEX IF NOT EXISTS "idx_assignment_chapter_raw_provider_created_at"
    ON "t_assignment" ("f_chapter_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_raw_provider_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_chapter_translator_created_at"
    ON "t_assignment" ("f_chapter_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_translator_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_chapter_proofreader_created_at"
    ON "t_assignment" ("f_chapter_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_proofreader_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_chapter_typesetter_created_at"
    ON "t_assignment" ("f_chapter_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_typesetter_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_chapter_redrawer_created_at"
    ON "t_assignment" ("f_chapter_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_redrawer_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_chapter_reviewer_created_at"
    ON "t_assignment" ("f_chapter_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_reviewer_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_chapter_publisher_created_at"
    ON "t_assignment" ("f_chapter_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_publisher_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_chapter_admin_created_at"
    ON "t_assignment" ("f_chapter_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_admin_at" IS NOT NULL;

CREATE INDEX IF NOT EXISTS "idx_assignment_user_raw_provider_created_at"
    ON "t_assignment" ("f_user_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_raw_provider_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_user_translator_created_at"
    ON "t_assignment" ("f_user_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_translator_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_user_proofreader_created_at"
    ON "t_assignment" ("f_user_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_proofreader_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_user_typesetter_created_at"
    ON "t_assignment" ("f_user_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_typesetter_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_user_redrawer_created_at"
    ON "t_assignment" ("f_user_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_redrawer_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_user_reviewer_created_at"
    ON "t_assignment" ("f_user_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_reviewer_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_user_publisher_created_at"
    ON "t_assignment" ("f_user_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_publisher_at" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_assignment_user_admin_created_at"
    ON "t_assignment" ("f_user_id", "f_created_at" DESC, "f_id" ASC)
    WHERE "f_assigned_admin_at" IS NOT NULL;
