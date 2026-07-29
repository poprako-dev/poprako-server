CREATE TABLE IF NOT EXISTS "t_assignment_invitation" (
    "f_id"                         TEXT        PRIMARY KEY,

    "f_chapter_id"                 TEXT        NOT NULL REFERENCES "t_chapter" ("f_id") ON DELETE RESTRICT,
    "f_inviter_id"                 TEXT        NOT NULL REFERENCES "t_user" ("f_id") ON DELETE RESTRICT,

    "f_invitee_qid"                TEXT        NOT NULL,
    "f_code"                       TEXT        NOT NULL,

    "f_pending"                    BOOLEAN     NOT NULL DEFAULT TRUE,
    "f_role_mask"                  BIGINT      NOT NULL,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
