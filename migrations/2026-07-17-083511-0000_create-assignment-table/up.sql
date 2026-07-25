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
