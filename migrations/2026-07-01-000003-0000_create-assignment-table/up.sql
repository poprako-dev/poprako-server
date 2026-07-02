CREATE TABLE IF NOT EXISTS "t_assignment" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_chapter_id"                 TEXT        NOT NULL REFERENCES "t_chapter" ("f_id") ON DELETE CASCADE,
    "f_user_id"                    TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE CASCADE,

    "f_assigned_raw_provider_at"   TIMESTAMPTZ,
    "f_assigned_translator_at"     TIMESTAMPTZ,
    "f_assigned_proofreader_at"    TIMESTAMPTZ,
    "f_assigned_typesetter_at"     TIMESTAMPTZ,
    "f_assigned_redrawer_at"       TIMESTAMPTZ,
    "f_assigned_reviewer_at"       TIMESTAMPTZ,
    "f_assigned_publisher_at"      TIMESTAMPTZ,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_assignment_chapter_id_user_id"
    ON "t_assignment" ("f_chapter_id", "f_user_id");

CREATE INDEX IF NOT EXISTS "idx_assignment_chapter_id_created_at_desc"
    ON "t_assignment" ("f_chapter_id", "f_created_at" DESC);

CREATE INDEX IF NOT EXISTS "idx_assignment_user_id_created_at_desc"
    ON "t_assignment" ("f_user_id", "f_created_at" DESC);
