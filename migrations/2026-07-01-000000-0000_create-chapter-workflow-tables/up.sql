CREATE TABLE IF NOT EXISTS "t_chapter" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_comic_id"                   TEXT        NOT NULL REFERENCES "t_comic" ("f_id") ON DELETE CASCADE,
    "f_is_pinned"                  BOOLEAN     NOT NULL DEFAULT FALSE,

    "f_index"                      INTEGER     NOT NULL,
    "f_subtitle"                   TEXT        NOT NULL,

    "f_page_count"                 INTEGER     NOT NULL DEFAULT 0,
    "f_total_unit_count"           INTEGER     NOT NULL DEFAULT 0,
    "f_translated_unit_count"      INTEGER     NOT NULL DEFAULT 0,
    "f_proofread_unit_count"       INTEGER     NOT NULL DEFAULT 0,

    "f_stages"                     INTEGER     NOT NULL DEFAULT 0,

    "f_creator_id"                 TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_chapter_comic_id_index"
    ON "t_chapter" ("f_comic_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_chapter_comic_id_index_desc"
    ON "t_chapter" ("f_comic_id", "f_index" DESC);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_chapter_comic_id_pinned_true"
    ON "t_chapter" ("f_comic_id")
    WHERE "f_is_pinned" = TRUE;

CREATE INDEX IF NOT EXISTS "idx_chapter_creator_id"
    ON "t_chapter" ("f_creator_id");

CREATE TABLE IF NOT EXISTS "t_page" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_chapter_id"                 TEXT        NOT NULL REFERENCES "t_chapter" ("f_id") ON DELETE CASCADE,
    "f_index"                      INTEGER     NOT NULL,

    "f_image_key"                  TEXT,
    "f_image_uploaded"             BOOLEAN     NOT NULL DEFAULT FALSE,
    "f_image_version"              BIGINT      NOT NULL DEFAULT 0,

    "f_total_unit_count"           INTEGER     NOT NULL DEFAULT 0,
    "f_translated_unit_count"      INTEGER     NOT NULL DEFAULT 0,
    "f_proofread_unit_count"       INTEGER     NOT NULL DEFAULT 0,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_page_chapter_id_index"
    ON "t_page" ("f_chapter_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_page_chapter_id"
    ON "t_page" ("f_chapter_id");

CREATE TABLE IF NOT EXISTS "t_unit" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_page_id"                    TEXT        NOT NULL REFERENCES "t_page" ("f_id") ON DELETE CASCADE,
    "f_index"                      INTEGER     NOT NULL,

    "f_is_bubble"                  BOOLEAN     NOT NULL DEFAULT FALSE,
    "f_is_proofread"               BOOLEAN     NOT NULL DEFAULT FALSE,

    "f_x_coord"                    DOUBLE PRECISION NOT NULL,
    "f_y_coord"                    DOUBLE PRECISION NOT NULL,

    "f_translated_text"            TEXT,
    "f_translator_comment"         TEXT,
    "f_last_translator_id"         TEXT REFERENCES "t_user" ("f_id") ON DELETE SET NULL,

    "f_proofread_text"             TEXT,
    "f_proofreader_comment"        TEXT,
    "f_last_proofreader_id"        TEXT REFERENCES "t_user" ("f_id") ON DELETE SET NULL,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_unit_page_id_index"
    ON "t_unit" ("f_page_id", "f_index");

CREATE INDEX IF NOT EXISTS "idx_unit_page_id"
    ON "t_unit" ("f_page_id");

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

CREATE TABLE IF NOT EXISTS "t_assignment_invitation" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_chapter_id"                 TEXT        NOT NULL REFERENCES "t_chapter" ("f_id") ON DELETE CASCADE,
    "f_inviter_id"                 TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE CASCADE,

    "f_invitee_qid"                TEXT        NOT NULL,
    "f_invitation_code"            TEXT        NOT NULL,

    "f_pending"                    BOOLEAN     NOT NULL DEFAULT TRUE,
    "f_role_mask"                  BIGINT      NOT NULL,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "idx_assignment_invitation_chapter_id"
    ON "t_assignment_invitation" ("f_chapter_id");

CREATE INDEX IF NOT EXISTS "idx_assignment_invitation_invitee_qid_pending"
    ON "t_assignment_invitation" ("f_invitee_qid", "f_pending", "f_created_at" DESC);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_assignment_invitation_pending_code"
    ON "t_assignment_invitation" ("f_invitation_code")
    WHERE "f_pending" = TRUE;

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_assignment_invitation_pending_chapter_invitee"
    ON "t_assignment_invitation" ("f_chapter_id", "f_invitee_qid")
    WHERE "f_pending" = TRUE;

CREATE TABLE IF NOT EXISTS "t_announcement" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_team_id"                    TEXT        NOT NULL REFERENCES "t_team" ("f_id") ON DELETE CASCADE,
    "f_user_id"                    TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE CASCADE,

    "f_title"                      TEXT        NOT NULL,
    "f_content"                    TEXT        NOT NULL,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "idx_announcement_team_id_created_at_desc"
    ON "t_announcement" ("f_team_id", "f_created_at" DESC);

CREATE TABLE IF NOT EXISTS "t_comment" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_team_id"                    TEXT        NOT NULL REFERENCES "t_team" ("f_id") ON DELETE CASCADE,
    "f_user_id"                    TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE CASCADE,

    "f_content"                    TEXT        NOT NULL,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "idx_comment_team_id_created_at_desc"
    ON "t_comment" ("f_team_id", "f_created_at" DESC);
