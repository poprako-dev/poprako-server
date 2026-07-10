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

CREATE INDEX IF NOT EXISTS "idx_assignment_invitation_chapter_id"
    ON "t_assignment_invitation" ("f_chapter_id");

CREATE INDEX IF NOT EXISTS "idx_assignment_invitation_invitee_qid_pending"
    ON "t_assignment_invitation" ("f_invitee_qid", "f_pending", "f_created_at" DESC);

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_assignment_invitation_pending_code"
    ON "t_assignment_invitation" ("f_code")
    WHERE "f_pending" = TRUE;

CREATE UNIQUE INDEX IF NOT EXISTS "uidx_assignment_invitation_pending_chapter_invitee"
    ON "t_assignment_invitation" ("f_chapter_id", "f_invitee_qid")
    WHERE "f_pending" = TRUE;
