CREATE TABLE IF NOT EXISTS "t_member_invitation" (
    "f_id" TEXT PRIMARY KEY,

    "f_inviter_id" TEXT NOT NULL REFERENCES "t_user" ("f_id") ON DELETE CASCADE,
    "f_team_id" TEXT NOT NULL REFERENCES "t_team" ("f_id") ON DELETE CASCADE,

    "f_invitee_qid" TEXT NOT NULL,
    "f_code" TEXT NOT NULL,

    "f_pending" BOOLEAN NOT NULL DEFAULT TRUE,

    "f_role_mask" BIGINT NOT NULL DEFAULT 0,

    "f_created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "idx_member_invitation_invitee_qid"
    ON "t_member_invitation" ("f_invitee_qid");
CREATE UNIQUE INDEX IF NOT EXISTS "uidx_member_invitation_team_id_invitee_qid_pending"
    ON "t_member_invitation" ("f_team_id", "f_invitee_qid")
    WHERE "f_pending" = TRUE;
CREATE INDEX IF NOT EXISTS "idx_member_invitation_team_id_created_at_desc"
    ON "t_member_invitation" ("f_team_id", "f_created_at" DESC);
